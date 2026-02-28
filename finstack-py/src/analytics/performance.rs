//! Python bindings for the `Performance` analytics struct.
//!
//! Accepts Polars DataFrames on the Python side, extracts columns to Rust
//! slices, delegates to `finstack_core::analytics::Performance`, and packs
//! results back into Polars DataFrames or Python dicts.

use crate::core::dates::utils::py_to_date;
use crate::errors::core_to_py;
use finstack_core::analytics::Performance;
use finstack_core::dates::PeriodKind;
use polars::prelude::*;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_polars::PyDataFrame;

type ExtractedData = (Vec<finstack_core::dates::Date>, Vec<Vec<f64>>, Vec<String>);

fn parse_freq(s: &str) -> PyResult<PeriodKind> {
    match s.to_ascii_lowercase().as_str() {
        "daily" | "d" => Ok(PeriodKind::Daily),
        "weekly" | "w" => Ok(PeriodKind::Weekly),
        "monthly" | "m" => Ok(PeriodKind::Monthly),
        "quarterly" | "q" => Ok(PeriodKind::Quarterly),
        "semiannual" | "semi_annual" | "h" => Ok(PeriodKind::SemiAnnual),
        "annual" | "yearly" | "a" | "y" => Ok(PeriodKind::Annual),
        other => Err(PyValueError::new_err(format!(
            "Unknown frequency '{other}'. Expected: daily, weekly, monthly, quarterly, semiannual, annual"
        ))),
    }
}

fn extract_dates_and_prices(df: &DataFrame) -> PyResult<ExtractedData> {
    let columns = df.get_columns();
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
        let vals: Vec<f64> = ca.into_iter().map(|opt| opt.unwrap_or(f64::NAN)).collect();
        prices.push(vals);
    }

    Ok((dates, prices, ticker_names))
}

fn vec_to_series(name: &str, data: &[f64]) -> Series {
    Series::new(name.into(), data)
}

fn rolling_to_df(
    dates: &[finstack_core::dates::Date],
    values: &[f64],
    metric_name: &str,
) -> PyResult<PyDataFrame> {
    let epoch = finstack_core::dates::Date::from_calendar_date(1970, time::Month::January, 1)
        .map_err(|_| PyValueError::new_err("Cannot create epoch"))?;
    let days: Vec<i32> = dates
        .iter()
        .map(|d| (*d - epoch).whole_days() as i32)
        .collect();
    let date_col: Column = Series::new("date".into(), &days)
        .cast(&DataType::Date)
        .map_err(|e| PyValueError::new_err(format!("Date cast error: {e}")))?
        .into_column();
    let val_col = vec_to_series(metric_name, values).into_column();
    let df = DataFrame::new(vec![date_col, val_col])
        .map_err(|e| PyValueError::new_err(format!("DataFrame error: {e}")))?;
    Ok(PyDataFrame(df))
}

fn vecs_to_df(tickers: &[String], data: &[Vec<f64>]) -> PyResult<DataFrame> {
    let columns: Vec<Column> = tickers
        .iter()
        .zip(data.iter())
        .map(|(name, vals)| vec_to_series(name, vals).into_column())
        .collect();
    DataFrame::new(columns).map_err(|e| PyValueError::new_err(format!("DataFrame error: {e}")))
}

fn scalars_to_df(tickers: &[String], values: &[f64], metric_name: &str) -> PyResult<DataFrame> {
    let ticker_col = Column::new("ticker".into(), tickers);
    let value_col = vec_to_series(metric_name, values).into_column();
    DataFrame::new(vec![ticker_col, value_col])
        .map_err(|e| PyValueError::new_err(format!("DataFrame error: {e}")))
}

/// Performance analytics engine.
///
/// Construct from a Polars DataFrame with a Date column followed by price columns
/// (one per ticker). Computes returns, drawdowns, and benchmark-relative metrics.
///
/// Parameters
/// ----------
/// prices : polars.DataFrame
///     DataFrame with ``Date`` column as the first column and price series
///     as subsequent columns (one per ticker).
/// benchmark_ticker : str, optional
///     Name of the benchmark column. Defaults to the first price column.
/// freq : str
///     Observation frequency: ``"daily"``, ``"weekly"``, ``"monthly"``,
///     ``"quarterly"``, ``"semiannual"``, ``"annual"``.
/// log_returns : bool
///     If True, use log returns; otherwise use simple returns.
#[pyclass(name = "Performance", module = "finstack.analytics")]
pub struct PyPerformance {
    inner: Performance,
    tickers: Vec<String>,
}

#[pymethods]
impl PyPerformance {
    #[new]
    #[pyo3(signature = (prices, benchmark_ticker=None, freq="daily", log_returns=false))]
    fn new(
        prices: PyDataFrame,
        benchmark_ticker: Option<&str>,
        freq: &str,
        log_returns: bool,
    ) -> PyResult<Self> {
        let period_kind = parse_freq(freq)?;
        let (dates, price_cols, tickers) = extract_dates_and_prices(&prices.0)?;
        let inner = Performance::new(
            dates,
            price_cols,
            tickers.clone(),
            benchmark_ticker,
            period_kind,
            log_returns,
        )
        .map_err(core_to_py)?;
        Ok(Self { inner, tickers })
    }

    /// Reset the analysis date range.
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
    fn reset_bench_ticker(&mut self, ticker: &str) -> PyResult<()> {
        self.inner.reset_bench_ticker(ticker).map_err(core_to_py)
    }

    /// CAGR for each ticker.
    fn cagr(&self) -> PyResult<PyDataFrame> {
        let vals = self.inner.cagr();
        let df = scalars_to_df(&self.tickers, &vals, "cagr")?;
        Ok(PyDataFrame(df))
    }

    /// Mean return for each ticker.
    #[pyo3(signature = (annualize=true))]
    fn mean_return(&self, annualize: bool) -> PyResult<PyDataFrame> {
        let vals = self.inner.mean_return(annualize);
        let df = scalars_to_df(&self.tickers, &vals, "mean_return")?;
        Ok(PyDataFrame(df))
    }

    /// Volatility for each ticker.
    #[pyo3(signature = (annualize=true))]
    fn volatility(&self, annualize: bool) -> PyResult<PyDataFrame> {
        let vals = self.inner.volatility(annualize);
        let df = scalars_to_df(&self.tickers, &vals, "volatility")?;
        Ok(PyDataFrame(df))
    }

    /// Sharpe ratio for each ticker.
    #[pyo3(signature = (risk_free_rate=0.0))]
    fn sharpe(&self, risk_free_rate: f64) -> PyResult<PyDataFrame> {
        let vals = self.inner.sharpe(risk_free_rate);
        let df = scalars_to_df(&self.tickers, &vals, "sharpe")?;
        Ok(PyDataFrame(df))
    }

    /// Sortino ratio for each ticker.
    fn sortino(&self) -> PyResult<PyDataFrame> {
        let vals = self.inner.sortino();
        let df = scalars_to_df(&self.tickers, &vals, "sortino")?;
        Ok(PyDataFrame(df))
    }

    /// Calmar ratio for each ticker.
    fn calmar(&self) -> PyResult<PyDataFrame> {
        let vals = self.inner.calmar();
        let df = scalars_to_df(&self.tickers, &vals, "calmar")?;
        Ok(PyDataFrame(df))
    }

    /// Max drawdown for each ticker.
    fn max_drawdown(&self) -> PyResult<PyDataFrame> {
        let vals = self.inner.max_drawdown();
        let df = scalars_to_df(&self.tickers, &vals, "max_drawdown")?;
        Ok(PyDataFrame(df))
    }

    /// Value-at-Risk for each ticker.
    #[pyo3(signature = (confidence=0.95))]
    fn value_at_risk(&self, confidence: f64) -> PyResult<PyDataFrame> {
        let vals = self.inner.value_at_risk(confidence);
        let df = scalars_to_df(&self.tickers, &vals, "var")?;
        Ok(PyDataFrame(df))
    }

    /// Expected shortfall (CVaR) for each ticker.
    #[pyo3(signature = (confidence=0.95))]
    fn expected_shortfall(&self, confidence: f64) -> PyResult<PyDataFrame> {
        let vals = self.inner.expected_shortfall(confidence);
        let df = scalars_to_df(&self.tickers, &vals, "es")?;
        Ok(PyDataFrame(df))
    }

    /// Tail ratio for each ticker.
    #[pyo3(signature = (confidence=0.95))]
    fn tail_ratio(&self, confidence: f64) -> PyResult<PyDataFrame> {
        let vals = self.inner.tail_ratio(confidence);
        let df = scalars_to_df(&self.tickers, &vals, "tail_ratio")?;
        Ok(PyDataFrame(df))
    }

    /// Ulcer index for each ticker.
    fn ulcer_index(&self) -> PyResult<PyDataFrame> {
        let vals = self.inner.ulcer_index();
        let df = scalars_to_df(&self.tickers, &vals, "ulcer_index")?;
        Ok(PyDataFrame(df))
    }

    /// Risk of ruin for each ticker.
    fn risk_of_ruin(&self) -> PyResult<PyDataFrame> {
        let vals = self.inner.risk_of_ruin();
        let df = scalars_to_df(&self.tickers, &vals, "risk_of_ruin")?;
        Ok(PyDataFrame(df))
    }

    /// Tracking error for each ticker vs benchmark.
    fn tracking_error(&self) -> PyResult<PyDataFrame> {
        let vals = self.inner.tracking_error();
        let df = scalars_to_df(&self.tickers, &vals, "tracking_error")?;
        Ok(PyDataFrame(df))
    }

    /// Information ratio for each ticker vs benchmark.
    fn information_ratio(&self) -> PyResult<PyDataFrame> {
        let vals = self.inner.information_ratio();
        let df = scalars_to_df(&self.tickers, &vals, "information_ratio")?;
        Ok(PyDataFrame(df))
    }

    /// R-squared for each ticker vs benchmark.
    fn r_squared(&self) -> PyResult<PyDataFrame> {
        let vals = self.inner.r_squared();
        let df = scalars_to_df(&self.tickers, &vals, "r_squared")?;
        Ok(PyDataFrame(df))
    }

    /// Beta for each ticker vs benchmark.
    fn beta(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        let betas = self.inner.beta();
        let dict = PyDict::new(py);
        for (ticker, br) in self.tickers.iter().zip(betas.iter()) {
            let inner = PyDict::new(py);
            inner.set_item("beta", br.beta)?;
            inner.set_item("std_err", br.std_err)?;
            inner.set_item("ci_lower", br.ci_lower)?;
            inner.set_item("ci_upper", br.ci_upper)?;
            dict.set_item(ticker, inner)?;
        }
        Ok(dict.into())
    }

    /// Greeks (alpha, beta, r²) for each ticker vs benchmark.
    fn greeks(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        let g = self.inner.greeks();
        let dict = PyDict::new(py);
        for (ticker, gr) in self.tickers.iter().zip(g.iter()) {
            let inner = PyDict::new(py);
            inner.set_item("alpha", gr.alpha)?;
            inner.set_item("beta", gr.beta)?;
            inner.set_item("r_squared", gr.r_squared)?;
            dict.set_item(ticker, inner)?;
        }
        Ok(dict.into())
    }

    /// Cumulative returns for each ticker.
    fn cumulative_returns(&self) -> PyResult<PyDataFrame> {
        let data = self.inner.cumulative_returns();
        let df = vecs_to_df(&self.tickers, &data)?;
        Ok(PyDataFrame(df))
    }

    /// Drawdown series for each ticker.
    fn drawdown_series(&self) -> PyResult<PyDataFrame> {
        let data = self.inner.drawdown_series();
        let df = vecs_to_df(&self.tickers, &data)?;
        Ok(PyDataFrame(df))
    }

    /// Correlation matrix of all tickers.
    fn correlation(&self) -> PyResult<PyDataFrame> {
        let matrix = self.inner.correlation_matrix();
        let columns: Vec<Column> = self
            .tickers
            .iter()
            .zip(matrix.iter())
            .map(|(name, vals)| vec_to_series(name, vals).into_column())
            .collect();
        let df = DataFrame::new(columns)
            .map_err(|e| PyValueError::new_err(format!("DataFrame error: {e}")))?;
        Ok(PyDataFrame(df))
    }

    /// Skewness for each ticker.
    fn skewness(&self) -> PyResult<PyDataFrame> {
        let vals = self.inner.skewness();
        let df = scalars_to_df(&self.tickers, &vals, "skewness")?;
        Ok(PyDataFrame(df))
    }

    /// Excess kurtosis for each ticker.
    fn kurtosis(&self) -> PyResult<PyDataFrame> {
        let vals = self.inner.kurtosis();
        let df = scalars_to_df(&self.tickers, &vals, "kurtosis")?;
        Ok(PyDataFrame(df))
    }

    /// Geometric mean return for each ticker.
    fn geometric_mean(&self) -> PyResult<PyDataFrame> {
        let vals = self.inner.geometric_mean();
        let df = scalars_to_df(&self.tickers, &vals, "geometric_mean")?;
        Ok(PyDataFrame(df))
    }

    /// Downside deviation for each ticker.
    #[pyo3(signature = (mar=0.0))]
    fn downside_deviation(&self, mar: f64) -> PyResult<PyDataFrame> {
        let vals = self.inner.downside_deviation(mar);
        let df = scalars_to_df(&self.tickers, &vals, "downside_deviation")?;
        Ok(PyDataFrame(df))
    }

    /// Maximum drawdown duration (days) for each ticker.
    fn max_drawdown_duration(&self) -> PyResult<PyDataFrame> {
        let vals = self.inner.max_drawdown_duration();
        let ticker_col = polars::prelude::Column::new("ticker".into(), &self.tickers);
        let value_col = Series::new("max_drawdown_duration".into(), &vals).into_column();
        let df = DataFrame::new(vec![ticker_col, value_col])
            .map_err(|e| PyValueError::new_err(format!("DataFrame error: {e}")))?;
        Ok(PyDataFrame(df))
    }

    /// Up-market capture ratio for each ticker vs benchmark.
    fn up_capture(&self) -> PyResult<PyDataFrame> {
        let vals = self.inner.up_capture();
        let df = scalars_to_df(&self.tickers, &vals, "up_capture")?;
        Ok(PyDataFrame(df))
    }

    /// Down-market capture ratio for each ticker vs benchmark.
    fn down_capture(&self) -> PyResult<PyDataFrame> {
        let vals = self.inner.down_capture();
        let df = scalars_to_df(&self.tickers, &vals, "down_capture")?;
        Ok(PyDataFrame(df))
    }

    /// Capture ratio (up/down) for each ticker vs benchmark.
    fn capture_ratio(&self) -> PyResult<PyDataFrame> {
        let vals = self.inner.capture_ratio();
        let df = scalars_to_df(&self.tickers, &vals, "capture_ratio")?;
        Ok(PyDataFrame(df))
    }

    /// Omega ratio for each ticker.
    #[pyo3(signature = (threshold=0.0))]
    fn omega_ratio(&self, threshold: f64) -> PyResult<PyDataFrame> {
        let vals = self.inner.omega_ratio(threshold);
        let df = scalars_to_df(&self.tickers, &vals, "omega_ratio")?;
        Ok(PyDataFrame(df))
    }

    /// Treynor ratio for each ticker.
    #[pyo3(signature = (risk_free_rate=0.0))]
    fn treynor(&self, risk_free_rate: f64) -> PyResult<PyDataFrame> {
        let vals = self.inner.treynor(risk_free_rate);
        let df = scalars_to_df(&self.tickers, &vals, "treynor")?;
        Ok(PyDataFrame(df))
    }

    /// Gain-to-pain ratio for each ticker.
    fn gain_to_pain(&self) -> PyResult<PyDataFrame> {
        let vals = self.inner.gain_to_pain();
        let df = scalars_to_df(&self.tickers, &vals, "gain_to_pain")?;
        Ok(PyDataFrame(df))
    }

    /// Martin ratio (CAGR / Ulcer Index) for each ticker.
    fn martin_ratio(&self) -> PyResult<PyDataFrame> {
        let vals = self.inner.martin_ratio();
        let df = scalars_to_df(&self.tickers, &vals, "martin_ratio")?;
        Ok(PyDataFrame(df))
    }

    /// Parametric (Gaussian) VaR for each ticker.
    #[pyo3(signature = (confidence=0.95))]
    fn parametric_var(&self, confidence: f64) -> PyResult<PyDataFrame> {
        let vals = self.inner.parametric_var(confidence);
        let df = scalars_to_df(&self.tickers, &vals, "parametric_var")?;
        Ok(PyDataFrame(df))
    }

    /// Cornish-Fisher adjusted VaR for each ticker.
    #[pyo3(signature = (confidence=0.95))]
    fn cornish_fisher_var(&self, confidence: f64) -> PyResult<PyDataFrame> {
        let vals = self.inner.cornish_fisher_var(confidence);
        let df = scalars_to_df(&self.tickers, &vals, "cornish_fisher_var")?;
        Ok(PyDataFrame(df))
    }

    /// Recovery factor for each ticker.
    fn recovery_factor(&self) -> PyResult<PyDataFrame> {
        let vals = self.inner.recovery_factor();
        let df = scalars_to_df(&self.tickers, &vals, "recovery_factor")?;
        Ok(PyDataFrame(df))
    }

    /// Sterling ratio for each ticker.
    #[pyo3(signature = (risk_free_rate=0.0, n=5))]
    fn sterling_ratio(&self, risk_free_rate: f64, n: usize) -> PyResult<PyDataFrame> {
        let vals = self.inner.sterling_ratio(risk_free_rate, n);
        let df = scalars_to_df(&self.tickers, &vals, "sterling_ratio")?;
        Ok(PyDataFrame(df))
    }

    /// Burke ratio for each ticker.
    #[pyo3(signature = (risk_free_rate=0.0, n=5))]
    fn burke_ratio(&self, risk_free_rate: f64, n: usize) -> PyResult<PyDataFrame> {
        let vals = self.inner.burke_ratio(risk_free_rate, n);
        let df = scalars_to_df(&self.tickers, &vals, "burke_ratio")?;
        Ok(PyDataFrame(df))
    }

    /// Pain index for each ticker.
    fn pain_index(&self) -> PyResult<PyDataFrame> {
        let vals = self.inner.pain_index();
        let df = scalars_to_df(&self.tickers, &vals, "pain_index")?;
        Ok(PyDataFrame(df))
    }

    /// Pain ratio for each ticker.
    #[pyo3(signature = (risk_free_rate=0.0))]
    fn pain_ratio(&self, risk_free_rate: f64) -> PyResult<PyDataFrame> {
        let vals = self.inner.pain_ratio(risk_free_rate);
        let df = scalars_to_df(&self.tickers, &vals, "pain_ratio")?;
        Ok(PyDataFrame(df))
    }

    /// Conditional Drawdown at Risk for each ticker.
    #[pyo3(signature = (confidence=0.95))]
    fn cdar(&self, confidence: f64) -> PyResult<PyDataFrame> {
        let vals = self.inner.cdar(confidence);
        let df = scalars_to_df(&self.tickers, &vals, "cdar")?;
        Ok(PyDataFrame(df))
    }

    /// Rolling annualized volatility for a specific ticker.
    ///
    /// Returns a DataFrame with ``date`` and ``value`` columns.
    #[pyo3(signature = (ticker_idx, window))]
    fn rolling_volatility(&self, ticker_idx: usize, window: usize) -> PyResult<PyDataFrame> {
        let rv = self.inner.rolling_volatility(ticker_idx, window);
        rolling_to_df(&rv.dates, &rv.values, "volatility")
    }

    /// Rolling Sortino ratio for a specific ticker.
    ///
    /// Returns a DataFrame with ``date`` and ``value`` columns.
    #[pyo3(signature = (ticker_idx, window))]
    fn rolling_sortino(&self, ticker_idx: usize, window: usize) -> PyResult<PyDataFrame> {
        let rs = self.inner.rolling_sortino(ticker_idx, window);
        rolling_to_df(&rs.dates, &rs.values, "sortino")
    }

    /// Multi-factor OLS regression for a specific ticker.
    ///
    /// Parameters
    /// ----------
    /// ticker_idx : int
    ///     Zero-based column index of the portfolio ticker.
    /// factor_returns : polars.DataFrame
    ///     DataFrame where each column is a factor return series (no date column).
    ///
    /// Returns
    /// -------
    /// dict
    ///     ``{"alpha": float, "betas": list[float], "r_squared": float, "residual_vol": float}``
    #[pyo3(signature = (ticker_idx, factor_returns))]
    fn multi_factor_greeks(
        &self,
        py: Python<'_>,
        ticker_idx: usize,
        factor_returns: PyDataFrame,
    ) -> PyResult<Py<PyDict>> {
        let cols = factor_returns.0.get_columns();
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
        let result = self.inner.multi_factor_greeks(ticker_idx, &factor_refs);
        let dict = PyDict::new(py);
        dict.set_item("alpha", result.alpha)?;
        dict.set_item("betas", result.betas)?;
        dict.set_item("r_squared", result.r_squared)?;
        dict.set_item("adjusted_r_squared", result.adjusted_r_squared)?;
        dict.set_item("residual_vol", result.residual_vol)?;
        Ok(dict.into())
    }

    /// Batting average for each ticker vs benchmark.
    fn batting_average(&self) -> PyResult<PyDataFrame> {
        let vals = self.inner.batting_average();
        let df = scalars_to_df(&self.tickers, &vals, "batting_average")?;
        Ok(PyDataFrame(df))
    }

    /// M-squared (Modigliani-Modigliani) for each ticker.
    #[pyo3(signature = (risk_free_rate=0.0))]
    fn m_squared(&self, risk_free_rate: f64) -> PyResult<PyDataFrame> {
        let vals = self.inner.m_squared(risk_free_rate);
        let df = scalars_to_df(&self.tickers, &vals, "m_squared")?;
        Ok(PyDataFrame(df))
    }

    /// Modified Sharpe ratio for each ticker.
    #[pyo3(signature = (risk_free_rate=0.0, confidence=0.95))]
    fn modified_sharpe(&self, risk_free_rate: f64, confidence: f64) -> PyResult<PyDataFrame> {
        let vals = self.inner.modified_sharpe(risk_free_rate, confidence);
        let df = scalars_to_df(&self.tickers, &vals, "modified_sharpe")?;
        Ok(PyDataFrame(df))
    }

    /// Summary statistics string.
    fn __repr__(&self) -> String {
        format!(
            "Performance(tickers={:?}, freq={:?})",
            self.tickers,
            self.inner.freq()
        )
    }
}
