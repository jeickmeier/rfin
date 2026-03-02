//! Python bindings for the `Performance` analytics struct.
//!
//! Accepts Polars or pandas DataFrames on the Python side, extracts columns to Rust
//! slices, delegates to `finstack_core::analytics::Performance`, and packs
//! results back into Polars DataFrames or Python dicts.

use crate::core::dates::utils::py_to_date;
use crate::errors::core_to_py;
use finstack_core::analytics::{DrawdownEpisode, Performance};
use finstack_core::dates::PeriodKind;
use polars::prelude::*;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use pyo3_polars::PyDataFrame;

type ExtractedData = (Vec<finstack_core::dates::Date>, Vec<Vec<f64>>, Vec<String>);

/// Convert a Python object (pandas or polars DataFrame) to a Polars DataFrame.
fn py_to_polars_df(py: Python<'_>, obj: &Bound<'_, PyAny>) -> PyResult<DataFrame> {
    // Try extracting as PyDataFrame first (Polars path)
    if let Ok(pdf) = obj.extract::<PyDataFrame>() {
        return Ok(pdf.0);
    }

    // Check if it's a pandas DataFrame by looking for the `reset_index` method
    if obj.hasattr("reset_index")? {
        let pl = py.import("polars")?;
        let reset = obj.call_method0("reset_index")?;
        let polars_df = pl.call_method1("from_pandas", (&reset,))?;
        let pdf: PyDataFrame = polars_df.extract()?;
        let mut df = pdf.0;

        // pandas DatetimeIndex becomes Datetime in polars; cast to Date so
        // extract_dates_and_prices can handle it.
        if let Some(col) = df.columns().first() {
            if matches!(col.dtype(), DataType::Datetime(_, _)) {
                let name = col.name().clone();
                let casted = col.cast(&DataType::Date).map_err(|e| {
                    PyTypeError::new_err(format!("Cannot cast datetime column to Date: {e}"))
                })?;
                df.replace(&name, casted)
                    .map_err(|e| PyValueError::new_err(format!("Cannot replace column: {e}")))?;
            }
        }

        return Ok(df);
    }

    Err(PyTypeError::new_err(format!(
        "Expected a polars.DataFrame or pandas.DataFrame, got {}",
        obj.get_type().name()?
    )))
}

fn parse_freq(s: &str) -> PyResult<PeriodKind> {
    s.parse::<PeriodKind>().map_err(|_| {
        PyValueError::new_err(format!(
            "Unknown frequency '{s}'. Expected: daily, weekly, monthly, quarterly, semiannual, annual"
        ))
    })
}

fn extract_dates_and_prices(df: &DataFrame) -> PyResult<ExtractedData> {
    let columns = df.columns();
    if columns.is_empty() {
        return Err(PyValueError::new_err("DataFrame has no columns"));
    }

    let date_col = &columns[0];
    let dates: Vec<finstack_core::dates::Date> = match date_col.dtype() {
        DataType::Date => {
            let physical = date_col.to_physical_repr();
            let ca = physical.i32().map_err(|e| {
                PyTypeError::new_err(format!("Cannot read date column as i32: {e}"))
            })?;
            let epoch =
                finstack_core::dates::Date::from_calendar_date(1970, time::Month::January, 1)
                    .map_err(|_| PyValueError::new_err("Cannot create epoch"))?;
            ca.into_iter()
                .map(|opt| {
                    let days = opt.ok_or_else(|| PyValueError::new_err("Null date found"))?;
                    Ok(epoch + time::Duration::days(days as i64))
                })
                .collect::<PyResult<Vec<_>>>()?
        }
        _ => {
            return Err(PyTypeError::new_err(format!(
                "First column must be Date type, got {:?}",
                date_col.dtype()
            )));
        }
    };

    let mut prices: Vec<Vec<f64>> = Vec::with_capacity(columns.len() - 1);
    let mut ticker_names: Vec<String> = Vec::with_capacity(columns.len() - 1);

    for col in &columns[1..] {
        ticker_names.push(col.name().to_string());
        let f64_series = col.cast(&DataType::Float64).map_err(|e| {
            PyTypeError::new_err(format!(
                "Cannot cast column '{}' to Float64: {e}",
                col.name()
            ))
        })?;
        let ca = f64_series.f64().map_err(|e| {
            PyTypeError::new_err(format!("Cannot read column '{}' as f64: {e}", col.name()))
        })?;
        let col_name = col.name();
        let vals: Vec<f64> = ca
            .into_iter()
            .map(|opt| {
                opt.ok_or_else(|| {
                    PyValueError::new_err(format!(
                        "Column '{col_name}' contains null values. \
                         Fill or drop nulls before constructing Performance."
                    ))
                })
            })
            .collect::<PyResult<Vec<f64>>>()?;
        prices.push(vals);
    }

    Ok((dates, prices, ticker_names))
}

// ── DataFrame builder helpers ──

fn vec_to_series(name: &str, data: &[f64]) -> Series {
    Series::new(name.into(), data)
}

fn dates_to_column(dates: &[finstack_core::dates::Date]) -> PyResult<Column> {
    let epoch = finstack_core::dates::Date::from_calendar_date(1970, time::Month::January, 1)
        .map_err(|_| PyValueError::new_err("Cannot create epoch"))?;
    let days: Vec<i32> = dates
        .iter()
        .map(|d| (*d - epoch).whole_days() as i32)
        .collect();
    Series::new("date".into(), &days)
        .cast(&DataType::Date)
        .map_err(|e| PyValueError::new_err(format!("Date cast error: {e}")))
        .map(|s| s.into_column())
}

fn rolling_to_df(
    dates: &[finstack_core::dates::Date],
    values: &[f64],
    metric_name: &str,
) -> PyResult<PyDataFrame> {
    let date_col = dates_to_column(dates)?;
    let val_col = vec_to_series(metric_name, values).into_column();
    let df = DataFrame::new_infer_height(vec![date_col, val_col])
        .map_err(|e| PyValueError::new_err(format!("DataFrame error: {e}")))?;
    Ok(PyDataFrame(df))
}

fn vecs_to_df_with_dates(
    dates: &[finstack_core::dates::Date],
    tickers: &[String],
    data: &[Vec<f64>],
) -> PyResult<DataFrame> {
    let date_col = dates_to_column(dates)?;
    let mut columns: Vec<Column> = Vec::with_capacity(tickers.len() + 1);
    columns.push(date_col);
    for (name, vals) in tickers.iter().zip(data.iter()) {
        columns.push(vec_to_series(name, vals).into_column());
    }
    DataFrame::new_infer_height(columns)
        .map_err(|e| PyValueError::new_err(format!("DataFrame error: {e}")))
}

fn scalars_to_df(tickers: &[String], values: &[f64], metric_name: &str) -> PyResult<DataFrame> {
    let ticker_col = Column::new("ticker".into(), tickers);
    let value_col = vec_to_series(metric_name, values).into_column();
    DataFrame::new_infer_height(vec![ticker_col, value_col])
        .map_err(|e| PyValueError::new_err(format!("DataFrame error: {e}")))
}

fn episodes_to_py(py: Python<'_>, episodes: &[DrawdownEpisode]) -> PyResult<Py<PyList>> {
    let list = PyList::empty(py);
    for ep in episodes {
        let d = PyDict::new(py);
        d.set_item("start", ep.start.to_string())?;
        d.set_item("valley", ep.valley.to_string())?;
        d.set_item("end", ep.end.map(|e| e.to_string()))?;
        d.set_item("duration_days", ep.duration_days)?;
        d.set_item("max_drawdown", ep.max_drawdown)?;
        d.set_item("near_recovery_threshold", ep.near_recovery_threshold)?;
        list.append(d)?;
    }
    Ok(list.into())
}

fn scalars_i64_to_df(tickers: &[String], values: &[i64], metric_name: &str) -> PyResult<DataFrame> {
    let ticker_col = Column::new("ticker".into(), tickers);
    let value_col = Series::new(metric_name.into(), values).into_column();
    DataFrame::new_infer_height(vec![ticker_col, value_col])
        .map_err(|e| PyValueError::new_err(format!("DataFrame error: {e}")))
}

/// Performance analytics engine.
///
/// Construct from a Polars or pandas DataFrame. For Polars, the first column
/// must be a Date column followed by price columns (one per ticker). For pandas,
/// the index should contain dates and each column should be a price series.
///
/// Parameters
/// ----------
/// prices : polars.DataFrame | pandas.DataFrame
///     For polars: first column is Date, remaining are price series.
///     For pandas: index is dates, columns are price series.
/// benchmark_ticker : str, optional
///     Name of the benchmark column. Defaults to the first price column.
/// freq : str
///     Observation frequency: ``"daily"``, ``"weekly"``, ``"monthly"``,
///     ``"quarterly"``, ``"semiannual"``, ``"annual"``.
/// log_returns : bool
///     If True, use log returns; otherwise use simple returns.
#[pyclass(name = "Performance", module = "finstack.core.analytics")]
pub struct PyPerformance {
    inner: Performance,
}

impl PyPerformance {
    fn tickers(&self) -> &[String] {
        self.inner.ticker_names()
    }

    fn resolve_ticker(&self, ticker: &str) -> PyResult<usize> {
        self.inner
            .ticker_names()
            .iter()
            .position(|t| t == ticker)
            .ok_or_else(|| {
                PyValueError::new_err(format!(
                    "Unknown ticker '{ticker}'. Available: {:?}",
                    self.inner.ticker_names()
                ))
            })
    }
}

#[pymethods]
impl PyPerformance {
    #[new]
    #[pyo3(signature = (prices, benchmark_ticker=None, freq="daily", log_returns=false))]
    fn new(
        py: Python<'_>,
        prices: &Bound<'_, PyAny>,
        benchmark_ticker: Option<&str>,
        freq: &str,
        log_returns: bool,
    ) -> PyResult<Self> {
        let period_kind = parse_freq(freq)?;
        let df = py_to_polars_df(py, prices)?;
        let (dates, price_cols, tickers) = extract_dates_and_prices(&df)?;
        let inner = Performance::new(
            dates,
            price_cols,
            tickers,
            benchmark_ticker,
            period_kind,
            log_returns,
        )
        .map_err(core_to_py)?;
        Ok(Self { inner })
    }

    // ── Accessors ──

    /// Ticker names in column order.
    #[getter]
    fn ticker_names(&self) -> Vec<String> {
        self.inner.ticker_names().to_vec()
    }

    /// Index of the current benchmark ticker.
    #[getter]
    fn benchmark_idx(&self) -> usize {
        self.inner.benchmark_idx()
    }

    /// Observation frequency.
    #[getter]
    fn freq(&self) -> String {
        self.inner.freq().to_string()
    }

    /// Whether log returns are used internally.
    #[getter]
    fn log_returns(&self) -> bool {
        self.inner.uses_log_returns()
    }

    /// Active date vector as a Polars Series.
    ///
    /// After calling ``reset_date_range``, only the restricted dates are returned.
    #[getter]
    fn dates(&self) -> PyResult<PyDataFrame> {
        let date_col = dates_to_column(self.inner.active_dates())?;
        let df = DataFrame::new_infer_height(vec![date_col])
            .map_err(|e| PyValueError::new_err(format!("DataFrame error: {e}")))?;
        Ok(PyDataFrame(df))
    }

    // ── Mutators ──

    /// Reset the analysis date range.
    ///
    /// Parameters
    /// ----------
    /// start : datetime.date
    ///     First date to include (inclusive).
    /// end : datetime.date
    ///     Last date to include (inclusive).
    #[pyo3(signature = (start, end))]
    fn reset_date_range(
        &mut self,
        start: &Bound<'_, PyAny>,
        end: &Bound<'_, PyAny>,
    ) -> PyResult<()> {
        let s = py_to_date(start)?;
        let e = py_to_date(end)?;
        self.inner.reset_date_range(s, e);
        Ok(())
    }

    /// Reset the benchmark ticker.
    ///
    /// Parameters
    /// ----------
    /// ticker : str
    ///     Name of the ticker to use as benchmark.
    fn reset_bench_ticker(&mut self, ticker: &str) -> PyResult<()> {
        self.inner.reset_bench_ticker(ticker).map_err(core_to_py)
    }

    // ── Scalar metrics (return ticker × value DataFrames) ──

    /// CAGR for each ticker.
    fn cagr(&self) -> PyResult<PyDataFrame> {
        let vals = self.inner.cagr();
        let df = scalars_to_df(self.tickers(), &vals, "cagr")?;
        Ok(PyDataFrame(df))
    }

    /// Mean return for each ticker.
    ///
    /// Parameters
    /// ----------
    /// annualize : bool
    ///     If True, scales the mean by the annualization factor.
    #[pyo3(signature = (annualize=true))]
    fn mean_return(&self, annualize: bool) -> PyResult<PyDataFrame> {
        let vals = self.inner.mean_return(annualize);
        let df = scalars_to_df(self.tickers(), &vals, "mean_return")?;
        Ok(PyDataFrame(df))
    }

    /// Volatility for each ticker.
    ///
    /// Parameters
    /// ----------
    /// annualize : bool
    ///     If True, scales by ``sqrt(ann_factor)``.
    #[pyo3(signature = (annualize=true))]
    fn volatility(&self, annualize: bool) -> PyResult<PyDataFrame> {
        let vals = self.inner.volatility(annualize);
        let df = scalars_to_df(self.tickers(), &vals, "volatility")?;
        Ok(PyDataFrame(df))
    }

    /// Sharpe ratio for each ticker.
    ///
    /// Parameters
    /// ----------
    /// risk_free_rate : float
    ///     Annualized risk-free rate (e.g. 0.02 for 2%).
    #[pyo3(signature = (risk_free_rate=0.0))]
    fn sharpe(&self, risk_free_rate: f64) -> PyResult<PyDataFrame> {
        let vals = self.inner.sharpe(risk_free_rate);
        let df = scalars_to_df(self.tickers(), &vals, "sharpe")?;
        Ok(PyDataFrame(df))
    }

    /// Sortino ratio for each ticker.
    fn sortino(&self) -> PyResult<PyDataFrame> {
        let vals = self.inner.sortino();
        let df = scalars_to_df(self.tickers(), &vals, "sortino")?;
        Ok(PyDataFrame(df))
    }

    /// Calmar ratio for each ticker.
    fn calmar(&self) -> PyResult<PyDataFrame> {
        let vals = self.inner.calmar();
        let df = scalars_to_df(self.tickers(), &vals, "calmar")?;
        Ok(PyDataFrame(df))
    }

    /// Max drawdown for each ticker.
    fn max_drawdown(&self) -> PyResult<PyDataFrame> {
        let vals = self.inner.max_drawdown();
        let df = scalars_to_df(self.tickers(), &vals, "max_drawdown")?;
        Ok(PyDataFrame(df))
    }

    /// Value-at-Risk for each ticker.
    ///
    /// Parameters
    /// ----------
    /// confidence : float
    ///     Confidence level in (0, 1), e.g. 0.95 for 95% VaR.
    #[pyo3(signature = (confidence=0.95))]
    fn value_at_risk(&self, confidence: f64) -> PyResult<PyDataFrame> {
        let vals = self.inner.value_at_risk(confidence);
        let df = scalars_to_df(self.tickers(), &vals, "var")?;
        Ok(PyDataFrame(df))
    }

    /// Expected shortfall (CVaR) for each ticker.
    ///
    /// Parameters
    /// ----------
    /// confidence : float
    ///     Confidence level in (0, 1), e.g. 0.95.
    #[pyo3(signature = (confidence=0.95))]
    fn expected_shortfall(&self, confidence: f64) -> PyResult<PyDataFrame> {
        let vals = self.inner.expected_shortfall(confidence);
        let df = scalars_to_df(self.tickers(), &vals, "es")?;
        Ok(PyDataFrame(df))
    }

    /// Tail ratio for each ticker.
    ///
    /// Parameters
    /// ----------
    /// confidence : float
    ///     Quantile level for the upper tail (e.g. 0.95).
    #[pyo3(signature = (confidence=0.95))]
    fn tail_ratio(&self, confidence: f64) -> PyResult<PyDataFrame> {
        let vals = self.inner.tail_ratio(confidence);
        let df = scalars_to_df(self.tickers(), &vals, "tail_ratio")?;
        Ok(PyDataFrame(df))
    }

    /// Ulcer index for each ticker.
    fn ulcer_index(&self) -> PyResult<PyDataFrame> {
        let vals = self.inner.ulcer_index();
        let df = scalars_to_df(self.tickers(), &vals, "ulcer_index")?;
        Ok(PyDataFrame(df))
    }

    /// Risk of ruin for each ticker.
    fn risk_of_ruin(&self) -> PyResult<PyDataFrame> {
        let vals = self.inner.risk_of_ruin();
        let df = scalars_to_df(self.tickers(), &vals, "risk_of_ruin")?;
        Ok(PyDataFrame(df))
    }

    /// Skewness for each ticker.
    fn skewness(&self) -> PyResult<PyDataFrame> {
        let vals = self.inner.skewness();
        let df = scalars_to_df(self.tickers(), &vals, "skewness")?;
        Ok(PyDataFrame(df))
    }

    /// Excess kurtosis for each ticker.
    fn kurtosis(&self) -> PyResult<PyDataFrame> {
        let vals = self.inner.kurtosis();
        let df = scalars_to_df(self.tickers(), &vals, "kurtosis")?;
        Ok(PyDataFrame(df))
    }

    /// Geometric mean return for each ticker.
    fn geometric_mean(&self) -> PyResult<PyDataFrame> {
        let vals = self.inner.geometric_mean();
        let df = scalars_to_df(self.tickers(), &vals, "geometric_mean")?;
        Ok(PyDataFrame(df))
    }

    /// Downside deviation for each ticker.
    ///
    /// Parameters
    /// ----------
    /// mar : float
    ///     Minimum acceptable return threshold.
    #[pyo3(signature = (mar=0.0))]
    fn downside_deviation(&self, mar: f64) -> PyResult<PyDataFrame> {
        let vals = self.inner.downside_deviation(mar);
        let df = scalars_to_df(self.tickers(), &vals, "downside_deviation")?;
        Ok(PyDataFrame(df))
    }

    /// Maximum drawdown duration (calendar days) for each ticker.
    fn max_drawdown_duration(&self) -> PyResult<PyDataFrame> {
        let vals = self.inner.max_drawdown_duration();
        let df = scalars_i64_to_df(self.tickers(), &vals, "max_drawdown_duration")?;
        Ok(PyDataFrame(df))
    }

    // ── Benchmark-relative scalar metrics ──

    /// Tracking error for each ticker vs benchmark.
    fn tracking_error(&self) -> PyResult<PyDataFrame> {
        let vals = self.inner.tracking_error();
        let df = scalars_to_df(self.tickers(), &vals, "tracking_error")?;
        Ok(PyDataFrame(df))
    }

    /// Information ratio for each ticker vs benchmark.
    fn information_ratio(&self) -> PyResult<PyDataFrame> {
        let vals = self.inner.information_ratio();
        let df = scalars_to_df(self.tickers(), &vals, "information_ratio")?;
        Ok(PyDataFrame(df))
    }

    /// R-squared for each ticker vs benchmark.
    fn r_squared(&self) -> PyResult<PyDataFrame> {
        let vals = self.inner.r_squared();
        let df = scalars_to_df(self.tickers(), &vals, "r_squared")?;
        Ok(PyDataFrame(df))
    }

    /// Beta for each ticker vs benchmark.
    fn beta(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        let betas = self.inner.beta();
        let dict = PyDict::new(py);
        for (ticker, br) in self.tickers().iter().zip(betas.iter()) {
            let inner = PyDict::new(py);
            inner.set_item("beta", br.beta)?;
            inner.set_item("std_err", br.std_err)?;
            inner.set_item("ci_lower", br.ci_lower)?;
            inner.set_item("ci_upper", br.ci_upper)?;
            dict.set_item(ticker, inner)?;
        }
        Ok(dict.into())
    }

    /// Greeks (alpha, beta, r-squared) for each ticker vs benchmark.
    fn greeks(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        let g = self.inner.greeks();
        let dict = PyDict::new(py);
        for (ticker, gr) in self.tickers().iter().zip(g.iter()) {
            let inner = PyDict::new(py);
            inner.set_item("alpha", gr.alpha)?;
            inner.set_item("beta", gr.beta)?;
            inner.set_item("r_squared", gr.r_squared)?;
            dict.set_item(ticker, inner)?;
        }
        Ok(dict.into())
    }

    /// Up-market capture ratio for each ticker vs benchmark.
    fn up_capture(&self) -> PyResult<PyDataFrame> {
        let vals = self.inner.up_capture();
        let df = scalars_to_df(self.tickers(), &vals, "up_capture")?;
        Ok(PyDataFrame(df))
    }

    /// Down-market capture ratio for each ticker vs benchmark.
    fn down_capture(&self) -> PyResult<PyDataFrame> {
        let vals = self.inner.down_capture();
        let df = scalars_to_df(self.tickers(), &vals, "down_capture")?;
        Ok(PyDataFrame(df))
    }

    /// Capture ratio (up/down) for each ticker vs benchmark.
    fn capture_ratio(&self) -> PyResult<PyDataFrame> {
        let vals = self.inner.capture_ratio();
        let df = scalars_to_df(self.tickers(), &vals, "capture_ratio")?;
        Ok(PyDataFrame(df))
    }

    /// Batting average for each ticker vs benchmark.
    fn batting_average(&self) -> PyResult<PyDataFrame> {
        let vals = self.inner.batting_average();
        let df = scalars_to_df(self.tickers(), &vals, "batting_average")?;
        Ok(PyDataFrame(df))
    }

    // ── Standard ratios ──

    /// Omega ratio for each ticker.
    ///
    /// Parameters
    /// ----------
    /// threshold : float
    ///     Return threshold (typically 0.0).
    #[pyo3(signature = (threshold=0.0))]
    fn omega_ratio(&self, threshold: f64) -> PyResult<PyDataFrame> {
        let vals = self.inner.omega_ratio(threshold);
        let df = scalars_to_df(self.tickers(), &vals, "omega_ratio")?;
        Ok(PyDataFrame(df))
    }

    /// Treynor ratio for each ticker.
    ///
    /// Parameters
    /// ----------
    /// risk_free_rate : float
    ///     Annualized risk-free rate.
    #[pyo3(signature = (risk_free_rate=0.0))]
    fn treynor(&self, risk_free_rate: f64) -> PyResult<PyDataFrame> {
        let vals = self.inner.treynor(risk_free_rate);
        let df = scalars_to_df(self.tickers(), &vals, "treynor")?;
        Ok(PyDataFrame(df))
    }

    /// Gain-to-pain ratio for each ticker.
    fn gain_to_pain(&self) -> PyResult<PyDataFrame> {
        let vals = self.inner.gain_to_pain();
        let df = scalars_to_df(self.tickers(), &vals, "gain_to_pain")?;
        Ok(PyDataFrame(df))
    }

    /// Martin ratio (CAGR / Ulcer Index) for each ticker.
    fn martin_ratio(&self) -> PyResult<PyDataFrame> {
        let vals = self.inner.martin_ratio();
        let df = scalars_to_df(self.tickers(), &vals, "martin_ratio")?;
        Ok(PyDataFrame(df))
    }

    /// M-squared (Modigliani-Modigliani) for each ticker.
    ///
    /// Parameters
    /// ----------
    /// risk_free_rate : float
    ///     Annualized risk-free rate.
    #[pyo3(signature = (risk_free_rate=0.0))]
    fn m_squared(&self, risk_free_rate: f64) -> PyResult<PyDataFrame> {
        let vals = self.inner.m_squared(risk_free_rate);
        let df = scalars_to_df(self.tickers(), &vals, "m_squared")?;
        Ok(PyDataFrame(df))
    }

    /// Modified Sharpe ratio for each ticker.
    ///
    /// Parameters
    /// ----------
    /// risk_free_rate : float
    ///     Annualized risk-free rate.
    /// confidence : float
    ///     VaR confidence level (e.g. 0.95).
    #[pyo3(signature = (risk_free_rate=0.0, confidence=0.95))]
    fn modified_sharpe(&self, risk_free_rate: f64, confidence: f64) -> PyResult<PyDataFrame> {
        let vals = self.inner.modified_sharpe(risk_free_rate, confidence);
        let df = scalars_to_df(self.tickers(), &vals, "modified_sharpe")?;
        Ok(PyDataFrame(df))
    }

    // ── VaR variants ──

    /// Parametric (Gaussian) VaR for each ticker.
    ///
    /// Parameters
    /// ----------
    /// confidence : float
    ///     Confidence level in (0, 1), e.g. 0.95.
    #[pyo3(signature = (confidence=0.95))]
    fn parametric_var(&self, confidence: f64) -> PyResult<PyDataFrame> {
        let vals = self.inner.parametric_var(confidence);
        let df = scalars_to_df(self.tickers(), &vals, "parametric_var")?;
        Ok(PyDataFrame(df))
    }

    /// Cornish-Fisher adjusted VaR for each ticker.
    ///
    /// Parameters
    /// ----------
    /// confidence : float
    ///     Confidence level in (0, 1), e.g. 0.95.
    #[pyo3(signature = (confidence=0.95))]
    fn cornish_fisher_var(&self, confidence: f64) -> PyResult<PyDataFrame> {
        let vals = self.inner.cornish_fisher_var(confidence);
        let df = scalars_to_df(self.tickers(), &vals, "cornish_fisher_var")?;
        Ok(PyDataFrame(df))
    }

    // ── Drawdown-family ratios ──

    /// Recovery factor for each ticker.
    fn recovery_factor(&self) -> PyResult<PyDataFrame> {
        let vals = self.inner.recovery_factor();
        let df = scalars_to_df(self.tickers(), &vals, "recovery_factor")?;
        Ok(PyDataFrame(df))
    }

    /// Sterling ratio for each ticker.
    ///
    /// Parameters
    /// ----------
    /// risk_free_rate : float
    ///     Annualized risk-free rate.
    /// n : int
    ///     Number of worst drawdowns to average.
    #[pyo3(signature = (risk_free_rate=0.0, n=5))]
    fn sterling_ratio(&self, risk_free_rate: f64, n: usize) -> PyResult<PyDataFrame> {
        let vals = self.inner.sterling_ratio(risk_free_rate, n);
        let df = scalars_to_df(self.tickers(), &vals, "sterling_ratio")?;
        Ok(PyDataFrame(df))
    }

    /// Burke ratio for each ticker.
    ///
    /// Parameters
    /// ----------
    /// risk_free_rate : float
    ///     Annualized risk-free rate.
    /// n : int
    ///     Number of worst drawdown episodes to use.
    #[pyo3(signature = (risk_free_rate=0.0, n=5))]
    fn burke_ratio(&self, risk_free_rate: f64, n: usize) -> PyResult<PyDataFrame> {
        let vals = self.inner.burke_ratio(risk_free_rate, n);
        let df = scalars_to_df(self.tickers(), &vals, "burke_ratio")?;
        Ok(PyDataFrame(df))
    }

    /// Pain index for each ticker.
    fn pain_index(&self) -> PyResult<PyDataFrame> {
        let vals = self.inner.pain_index();
        let df = scalars_to_df(self.tickers(), &vals, "pain_index")?;
        Ok(PyDataFrame(df))
    }

    /// Pain ratio for each ticker.
    ///
    /// Parameters
    /// ----------
    /// risk_free_rate : float
    ///     Annualized risk-free rate.
    #[pyo3(signature = (risk_free_rate=0.0))]
    fn pain_ratio(&self, risk_free_rate: f64) -> PyResult<PyDataFrame> {
        let vals = self.inner.pain_ratio(risk_free_rate);
        let df = scalars_to_df(self.tickers(), &vals, "pain_ratio")?;
        Ok(PyDataFrame(df))
    }

    /// Conditional Drawdown at Risk for each ticker.
    ///
    /// Parameters
    /// ----------
    /// confidence : float
    ///     Confidence level in (0, 1), e.g. 0.95.
    #[pyo3(signature = (confidence=0.95))]
    fn cdar(&self, confidence: f64) -> PyResult<PyDataFrame> {
        let vals = self.inner.cdar(confidence);
        let df = scalars_to_df(self.tickers(), &vals, "cdar")?;
        Ok(PyDataFrame(df))
    }

    // ── Series outputs (include date column) ──

    /// Cumulative returns for each ticker.
    ///
    /// Returns a DataFrame with a ``date`` column followed by one column per ticker.
    fn cumulative_returns(&self) -> PyResult<PyDataFrame> {
        let data = self.inner.cumulative_returns();
        let df = vecs_to_df_with_dates(self.inner.active_dates(), self.tickers(), &data)?;
        Ok(PyDataFrame(df))
    }

    /// Drawdown series for each ticker.
    ///
    /// Returns a DataFrame with a ``date`` column followed by one column per ticker.
    fn drawdown_series(&self) -> PyResult<PyDataFrame> {
        let data = self.inner.drawdown_series();
        let df = vecs_to_df_with_dates(self.inner.active_dates(), self.tickers(), &data)?;
        Ok(PyDataFrame(df))
    }

    /// Correlation matrix of all tickers.
    ///
    /// Returns a DataFrame with a leading ``ticker`` column and one
    /// column per ticker containing pairwise correlations.
    fn correlation(&self) -> PyResult<PyDataFrame> {
        let matrix = self.inner.correlation_matrix();
        let ticker_col = Column::new("ticker".into(), self.tickers());
        let mut columns: Vec<Column> = Vec::with_capacity(self.tickers().len() + 1);
        columns.push(ticker_col);
        for (name, vals) in self.tickers().iter().zip(matrix.iter()) {
            columns.push(vec_to_series(name, vals).into_column());
        }
        let df = DataFrame::new_infer_height(columns)
            .map_err(|e| PyValueError::new_err(format!("DataFrame error: {e}")))?;
        Ok(PyDataFrame(df))
    }

    // ── Per-ticker rolling metrics (accept ticker name) ──

    /// Rolling annualized volatility for a specific ticker.
    ///
    /// Parameters
    /// ----------
    /// ticker : str
    ///     Name of the ticker.
    /// window : int
    ///     Look-back window length in periods.
    ///
    /// Returns
    /// -------
    /// polars.DataFrame
    ///     DataFrame with ``date`` and ``volatility`` columns.
    #[pyo3(signature = (ticker, window))]
    fn rolling_volatility(&self, ticker: &str, window: usize) -> PyResult<PyDataFrame> {
        let idx = self.resolve_ticker(ticker)?;
        let rv = self.inner.rolling_volatility(idx, window);
        rolling_to_df(&rv.dates, &rv.values, "volatility")
    }

    /// Rolling Sortino ratio for a specific ticker.
    ///
    /// Parameters
    /// ----------
    /// ticker : str
    ///     Name of the ticker.
    /// window : int
    ///     Look-back window length in periods.
    ///
    /// Returns
    /// -------
    /// polars.DataFrame
    ///     DataFrame with ``date`` and ``sortino`` columns.
    #[pyo3(signature = (ticker, window))]
    fn rolling_sortino(&self, ticker: &str, window: usize) -> PyResult<PyDataFrame> {
        let idx = self.resolve_ticker(ticker)?;
        let rs = self.inner.rolling_sortino(idx, window);
        rolling_to_df(&rs.dates, &rs.values, "sortino")
    }

    /// Rolling Sharpe ratio for a specific ticker.
    ///
    /// Parameters
    /// ----------
    /// ticker : str
    ///     Name of the ticker.
    /// window : int
    ///     Look-back window length in periods.
    /// risk_free_rate : float
    ///     Annualized risk-free rate to subtract.
    ///
    /// Returns
    /// -------
    /// polars.DataFrame
    ///     DataFrame with ``date`` and ``sharpe`` columns.
    #[pyo3(signature = (ticker, window, risk_free_rate=0.0))]
    fn rolling_sharpe(
        &self,
        ticker: &str,
        window: usize,
        risk_free_rate: f64,
    ) -> PyResult<PyDataFrame> {
        let idx = self.resolve_ticker(ticker)?;
        let rs = self.inner.rolling_sharpe(idx, window, risk_free_rate);
        rolling_to_df(&rs.dates, &rs.values, "sharpe")
    }

    /// Rolling greeks (alpha, beta) for a specific ticker vs the benchmark.
    ///
    /// Parameters
    /// ----------
    /// ticker : str
    ///     Name of the ticker.
    /// window : int
    ///     Look-back window length in periods.
    ///
    /// Returns
    /// -------
    /// polars.DataFrame
    ///     DataFrame with ``date``, ``alpha``, and ``beta`` columns.
    #[pyo3(signature = (ticker, window))]
    fn rolling_greeks(&self, ticker: &str, window: usize) -> PyResult<PyDataFrame> {
        let idx = self.resolve_ticker(ticker)?;
        let rg = self.inner.rolling_greeks(idx, window);
        let date_col = dates_to_column(&rg.dates)?;
        let alpha_col = vec_to_series("alpha", &rg.alphas).into_column();
        let beta_col = vec_to_series("beta", &rg.betas).into_column();
        let df = DataFrame::new_infer_height(vec![date_col, alpha_col, beta_col])
            .map_err(|e| PyValueError::new_err(format!("DataFrame error: {e}")))?;
        Ok(PyDataFrame(df))
    }

    /// Multi-factor OLS regression for a specific ticker.
    ///
    /// Parameters
    /// ----------
    /// ticker : str
    ///     Name of the portfolio ticker.
    /// factor_returns : polars.DataFrame
    ///     DataFrame where each column is a factor return series (no date column).
    ///
    /// Returns
    /// -------
    /// dict
    ///     Keys: ``alpha``, ``betas``, ``r_squared``, ``adjusted_r_squared``,
    ///     ``residual_vol``.
    #[pyo3(signature = (ticker, factor_returns))]
    fn multi_factor_greeks(
        &self,
        py: Python<'_>,
        ticker: &str,
        factor_returns: PyDataFrame,
    ) -> PyResult<Py<PyDict>> {
        let idx = self.resolve_ticker(ticker)?;
        let cols = factor_returns.0.columns();
        let factor_vecs: Vec<Vec<f64>> = cols
            .iter()
            .map(|col| {
                let f64_series = col.cast(&DataType::Float64).map_err(|e| {
                    PyTypeError::new_err(format!(
                        "Cannot cast factor column '{}' to Float64: {e}",
                        col.name()
                    ))
                })?;
                let ca = f64_series.f64().map_err(|e| {
                    PyTypeError::new_err(format!(
                        "Cannot read factor column '{}' as f64: {e}",
                        col.name()
                    ))
                })?;
                Ok(ca.into_iter().map(|opt| opt.unwrap_or(f64::NAN)).collect())
            })
            .collect::<PyResult<Vec<Vec<f64>>>>()?;
        let factor_refs: Vec<&[f64]> = factor_vecs.iter().map(|v| v.as_slice()).collect();
        let result = self.inner.multi_factor_greeks(idx, &factor_refs);
        let dict = PyDict::new(py);
        dict.set_item("alpha", result.alpha)?;
        dict.set_item("betas", result.betas)?;
        dict.set_item("r_squared", result.r_squared)?;
        dict.set_item("adjusted_r_squared", result.adjusted_r_squared)?;
        dict.set_item("residual_vol", result.residual_vol)?;
        Ok(dict.into())
    }

    // ── Drawdown episodes ──

    /// Top-N drawdown episodes for a specific ticker.
    ///
    /// Parameters
    /// ----------
    /// ticker : str
    ///     Name of the ticker.
    /// n : int
    ///     Maximum number of episodes to return, sorted by severity.
    ///
    /// Returns
    /// -------
    /// list[dict]
    ///     Each dict has keys: ``start``, ``valley``, ``end`` (or None),
    ///     ``duration_days``, ``max_drawdown``, ``near_recovery_threshold``.
    #[pyo3(signature = (ticker, n=5))]
    fn drawdown_details(&self, py: Python<'_>, ticker: &str, n: usize) -> PyResult<Py<PyList>> {
        let idx = self.resolve_ticker(ticker)?;
        let episodes = self.inner.drawdown_details(idx, n);
        episodes_to_py(py, &episodes)
    }

    /// Top-N drawdown episodes in the benchmark series.
    ///
    /// Useful for stress-test analysis: examine how the portfolio
    /// performs during the benchmark's worst historical periods.
    ///
    /// Parameters
    /// ----------
    /// n : int
    ///     Maximum number of episodes to return, sorted by severity.
    ///
    /// Returns
    /// -------
    /// list[dict]
    ///     Same structure as ``drawdown_details``.
    #[pyo3(signature = (n=5))]
    fn stats_during_bench_drawdowns(&self, py: Python<'_>, n: usize) -> PyResult<Py<PyList>> {
        let episodes = self.inner.stats_during_bench_drawdowns(n);
        episodes_to_py(py, &episodes)
    }

    // ── Outperformance series ──

    /// Cumulative outperformance (portfolio - benchmark) for each ticker.
    ///
    /// Returns a DataFrame with a ``date`` column followed by one column per ticker.
    fn cumulative_returns_outperformance(&self) -> PyResult<PyDataFrame> {
        let data = self.inner.cumulative_returns_outperformance();
        let df = vecs_to_df_with_dates(self.inner.active_dates(), self.tickers(), &data)?;
        Ok(PyDataFrame(df))
    }

    /// Drawdown outperformance (portfolio drawdown - benchmark drawdown) for each ticker.
    ///
    /// Returns a DataFrame with a ``date`` column followed by one column per ticker.
    fn drawdown_outperformance(&self) -> PyResult<PyDataFrame> {
        let data = self.inner.drawdown_outperformance();
        let df = vecs_to_df_with_dates(self.inner.active_dates(), self.tickers(), &data)?;
        Ok(PyDataFrame(df))
    }

    // ── Excess returns ──

    /// Excess returns (portfolio minus risk-free) for each ticker.
    ///
    /// Parameters
    /// ----------
    /// rf : list[float]
    ///     Risk-free rate series aligned with the active date window.
    /// nperiods : float, optional
    ///     If provided, de-compounds the risk-free rate from annual to
    ///     the observation frequency before subtraction.
    ///
    /// Returns
    /// -------
    /// polars.DataFrame
    ///     DataFrame with a ``date`` column and one column per ticker.
    #[pyo3(signature = (rf, nperiods=None))]
    fn excess_returns(&self, rf: Vec<f64>, nperiods: Option<f64>) -> PyResult<PyDataFrame> {
        let data = self.inner.excess_returns(&rf, nperiods);
        let df = vecs_to_df_with_dates(self.inner.active_dates(), self.tickers(), &data)?;
        Ok(PyDataFrame(df))
    }

    // ── Lookback returns ──

    /// Compounded returns for standard lookback periods (MTD, QTD, YTD).
    ///
    /// Parameters
    /// ----------
    /// ref_date : datetime.date
    ///     Reference date (typically the most recent business day).
    ///
    /// Returns
    /// -------
    /// dict
    ///     Keys: ``mtd``, ``qtd``, ``ytd`` (each a list of floats, one per ticker).
    #[pyo3(signature = (ref_date))]
    fn lookback_returns(
        &self,
        py: Python<'_>,
        ref_date: &Bound<'_, PyAny>,
    ) -> PyResult<Py<PyDict>> {
        let d = py_to_date(ref_date)?;
        let lb = self.inner.lookback_returns(d, None);
        let dict = PyDict::new(py);
        dict.set_item("mtd", &lb.mtd)?;
        dict.set_item("qtd", &lb.qtd)?;
        dict.set_item("ytd", &lb.ytd)?;
        Ok(dict.into())
    }

    // ── Period-aggregated stats ──

    /// Period-aggregated statistics for a specific ticker.
    ///
    /// Groups returns into ``agg_freq`` buckets, compounds within each,
    /// then computes win rate, payoff ratio, Kelly criterion, etc.
    ///
    /// Parameters
    /// ----------
    /// ticker : str
    ///     Name of the ticker.
    /// agg_freq : str
    ///     Aggregation frequency (e.g. ``"monthly"``, ``"annual"``).
    ///
    /// Returns
    /// -------
    /// dict
    ///     Keys: ``best``, ``worst``, ``consecutive_wins``, ``consecutive_losses``,
    ///     ``win_rate``, ``avg_return``, ``avg_win``, ``avg_loss``,
    ///     ``payoff_ratio``, ``profit_ratio``, ``profit_factor``,
    ///     ``cpc_ratio``, ``kelly_criterion``.
    #[pyo3(signature = (ticker, agg_freq="monthly"))]
    fn period_stats(&self, py: Python<'_>, ticker: &str, agg_freq: &str) -> PyResult<Py<PyDict>> {
        let idx = self.resolve_ticker(ticker)?;
        let freq = parse_freq(agg_freq)?;
        let ps = self.inner.period_stats(idx, freq, None);
        let dict = PyDict::new(py);
        dict.set_item("best", ps.best)?;
        dict.set_item("worst", ps.worst)?;
        dict.set_item("consecutive_wins", ps.consecutive_wins)?;
        dict.set_item("consecutive_losses", ps.consecutive_losses)?;
        dict.set_item("win_rate", ps.win_rate)?;
        dict.set_item("avg_return", ps.avg_return)?;
        dict.set_item("avg_win", ps.avg_win)?;
        dict.set_item("avg_loss", ps.avg_loss)?;
        dict.set_item("payoff_ratio", ps.payoff_ratio)?;
        dict.set_item("profit_ratio", ps.profit_ratio)?;
        dict.set_item("profit_factor", ps.profit_factor)?;
        dict.set_item("cpc_ratio", ps.cpc_ratio)?;
        dict.set_item("kelly_criterion", ps.kelly_criterion)?;
        Ok(dict.into())
    }

    // ── Repr ──

    fn __repr__(&self) -> String {
        let tickers = self.inner.ticker_names();
        let bench = tickers
            .get(self.inner.benchmark_idx())
            .map(|s| s.as_str())
            .unwrap_or("?");
        format!(
            "Performance(tickers={:?}, freq={}, benchmark='{}', n_dates={})",
            tickers,
            self.inner.freq(),
            bench,
            self.inner.active_dates().len(),
        )
    }
}
