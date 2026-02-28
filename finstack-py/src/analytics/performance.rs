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

    /// Summary statistics string.
    fn __repr__(&self) -> String {
        format!(
            "Performance(tickers={:?}, freq={:?})",
            self.tickers,
            self.inner.freq()
        )
    }
}
