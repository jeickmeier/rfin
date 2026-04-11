//! Stateful [`Performance`] analytics engine.

use super::types::*;
use crate::bindings::core::dates::utils::{date_to_py, py_to_date};
use crate::bindings::pandas_utils::{dates_to_pylist, dict_to_dataframe};
use crate::errors::core_to_py;
use finstack_analytics as fa;
use finstack_core::dates::PeriodKind;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyDict;

/// Convert an optional fiscal-year-start month into a FiscalConfig.
fn make_fiscal_config(month: Option<u8>) -> Option<finstack_core::dates::FiscalConfig> {
    month.and_then(|m| finstack_core::dates::FiscalConfig::new(m, 1).ok())
}

/// Parse a frequency string into a [`PeriodKind`].
pub(super) fn parse_freq(freq: &str) -> PyResult<PeriodKind> {
    freq.parse::<PeriodKind>().map_err(|_| {
        PyValueError::new_err(format!(
            "Unknown frequency {freq:?}; expected one of: \
             daily, weekly, monthly, quarterly, semiannual, annual"
        ))
    })
}

/// Decomposed DataFrame: dates, column-major prices, and ticker names.
struct DataFramePanel {
    /// Chronological observation dates.
    dates: Vec<time::Date>,
    /// `prices[ticker_idx][date_idx]`.
    prices: Vec<Vec<f64>>,
    /// Column names from the DataFrame.
    ticker_names: Vec<String>,
}

/// Extract dates, prices matrix, and ticker names from a pandas DataFrame.
///
/// Expects a DataFrame with a date-like index and float columns.
fn extract_dataframe(df: &Bound<'_, PyAny>) -> PyResult<DataFramePanel> {
    let index = df.getattr("index")?;
    let dates_list = index.call_method0("tolist")?;
    let dates_py: Vec<Bound<'_, PyAny>> = dates_list.extract()?;
    let dates: Vec<time::Date> = dates_py.iter().map(py_to_date).collect::<PyResult<_>>()?;

    let columns = df.getattr("columns")?;
    let cols_list = columns.call_method0("tolist")?;
    let ticker_names: Vec<String> = cols_list.extract()?;

    let n_tickers = ticker_names.len();
    let mut prices = Vec::with_capacity(n_tickers);
    for col in &ticker_names {
        let series = df.get_item(col)?;
        let values = series.call_method0("tolist")?;
        let col_data: Vec<f64> = values.extract()?;
        prices.push(col_data);
    }

    Ok(DataFramePanel {
        dates,
        prices,
        ticker_names,
    })
}

/// Build a `Performance` from pre-extracted arrays.
fn build_performance(
    dates: Vec<time::Date>,
    prices: Vec<Vec<f64>>,
    ticker_names: Vec<String>,
    benchmark_ticker: Option<&str>,
    freq: &str,
    use_log_returns: bool,
) -> PyResult<PyPerformance> {
    let period_kind = parse_freq(freq)?;
    let inner = fa::Performance::new(
        dates,
        prices,
        ticker_names,
        benchmark_ticker,
        period_kind,
        use_log_returns,
    )
    .map_err(core_to_py)?;
    Ok(PyPerformance { inner })
}

/// Stateful performance analytics engine over a panel of ticker price series.
///
/// Accepts a pandas ``DataFrame`` where the index contains dates and each
/// column is a price series for one ticker.
#[pyclass(name = "Performance", module = "finstack.analytics")]
pub(super) struct PyPerformance {
    inner: fa::Performance,
}

#[pymethods]
impl PyPerformance {
    /// Construct from a pandas DataFrame of prices.
    ///
    /// The DataFrame index must contain ``datetime.date`` or ``pd.Timestamp``
    /// values, and each column represents one ticker's price series.
    #[new]
    #[pyo3(signature = (prices, benchmark_ticker=None, freq="daily", use_log_returns=false))]
    fn new(
        prices: Bound<'_, PyAny>,
        benchmark_ticker: Option<&str>,
        freq: &str,
        use_log_returns: bool,
    ) -> PyResult<Self> {
        let py = prices.py();
        let pd = py.import("pandas")?;
        let df_type = pd.getattr("DataFrame")?;
        if !prices.is_instance(&df_type)? {
            return Err(PyTypeError::new_err(
                "Expected a pandas DataFrame; use Performance.from_arrays() for raw lists",
            ));
        }
        let panel = extract_dataframe(&prices)?;
        build_performance(
            panel.dates,
            panel.prices,
            panel.ticker_names,
            benchmark_ticker,
            freq,
            use_log_returns,
        )
    }

    /// Construct from raw arrays (dates, prices matrix, ticker names).
    #[staticmethod]
    #[pyo3(signature = (dates, prices, ticker_names, benchmark_ticker=None, freq="daily", use_log_returns=false))]
    fn from_arrays(
        dates: Vec<Bound<'_, PyAny>>,
        prices: Vec<Vec<f64>>,
        ticker_names: Vec<String>,
        benchmark_ticker: Option<&str>,
        freq: &str,
        use_log_returns: bool,
    ) -> PyResult<Self> {
        let rust_dates: Vec<time::Date> =
            dates.iter().map(py_to_date).collect::<PyResult<Vec<_>>>()?;
        build_performance(
            rust_dates,
            prices,
            ticker_names,
            benchmark_ticker,
            freq,
            use_log_returns,
        )
    }

    // -- Mutators --

    /// Restrict analytics to a date window.
    fn reset_date_range(&mut self, start: Bound<'_, PyAny>, end: Bound<'_, PyAny>) -> PyResult<()> {
        let s = py_to_date(&start)?;
        let e = py_to_date(&end)?;
        self.inner.reset_date_range(s, e);
        Ok(())
    }

    /// Change the benchmark ticker.
    fn reset_bench_ticker(&mut self, ticker: &str) -> PyResult<()> {
        self.inner.reset_bench_ticker(ticker).map_err(core_to_py)
    }

    // -- Getters --

    /// Ticker names in column order.
    #[getter]
    fn ticker_names(&self) -> Vec<String> {
        self.inner.ticker_names().to_vec()
    }

    /// Benchmark column index.
    #[getter]
    fn benchmark_idx(&self) -> usize {
        self.inner.benchmark_idx()
    }

    /// Observation frequency.
    #[getter]
    fn freq(&self) -> String {
        format!("{:?}", self.inner.freq())
    }

    /// Whether log returns are used internally.
    #[getter]
    fn uses_log_returns(&self) -> bool {
        self.inner.uses_log_returns()
    }

    /// Active date grid.
    fn dates<'py>(&self, py: Python<'py>) -> PyResult<Vec<Bound<'py, PyAny>>> {
        self.inner
            .active_dates()
            .iter()
            .map(|&d| date_to_py(py, d))
            .collect()
    }

    // -- Scalar-per-ticker methods --

    /// CAGR for each ticker.
    fn cagr(&self) -> Vec<f64> {
        self.inner.cagr()
    }

    /// Mean return for each ticker.
    #[pyo3(signature = (annualize = true))]
    fn mean_return(&self, annualize: bool) -> Vec<f64> {
        self.inner.mean_return(annualize)
    }

    /// Volatility for each ticker.
    #[pyo3(signature = (annualize = true))]
    fn volatility(&self, annualize: bool) -> Vec<f64> {
        self.inner.volatility(annualize)
    }

    /// Sharpe ratio for each ticker.
    #[pyo3(signature = (risk_free_rate = 0.0))]
    fn sharpe(&self, risk_free_rate: f64) -> Vec<f64> {
        self.inner.sharpe(risk_free_rate)
    }

    /// Sortino ratio for each ticker.
    fn sortino(&self) -> Vec<f64> {
        self.inner.sortino()
    }

    /// Calmar ratio for each ticker.
    fn calmar(&self) -> Vec<f64> {
        self.inner.calmar()
    }

    /// Max drawdown for each ticker.
    fn max_drawdown(&self) -> Vec<f64> {
        self.inner.max_drawdown()
    }

    /// Historical VaR for each ticker.
    #[pyo3(signature = (confidence = 0.95))]
    fn value_at_risk(&self, confidence: f64) -> Vec<f64> {
        self.inner.value_at_risk(confidence)
    }

    /// Expected Shortfall for each ticker.
    #[pyo3(signature = (confidence = 0.95))]
    fn expected_shortfall(&self, confidence: f64) -> Vec<f64> {
        self.inner.expected_shortfall(confidence)
    }

    /// Tracking error for each ticker vs benchmark.
    fn tracking_error(&self) -> Vec<f64> {
        self.inner.tracking_error()
    }

    /// Information ratio for each ticker vs benchmark.
    fn information_ratio(&self) -> Vec<f64> {
        self.inner.information_ratio()
    }

    /// Skewness for each ticker.
    fn skewness(&self) -> Vec<f64> {
        self.inner.skewness()
    }

    /// Kurtosis for each ticker.
    fn kurtosis(&self) -> Vec<f64> {
        self.inner.kurtosis()
    }

    /// Geometric mean for each ticker.
    fn geometric_mean(&self) -> Vec<f64> {
        self.inner.geometric_mean()
    }

    /// Downside deviation for each ticker.
    #[pyo3(signature = (mar = 0.0))]
    fn downside_deviation(&self, mar: f64) -> Vec<f64> {
        self.inner.downside_deviation(mar)
    }

    /// Max drawdown duration (calendar days) for each ticker.
    fn max_drawdown_duration(&self) -> Vec<i64> {
        self.inner.max_drawdown_duration()
    }

    /// Up-capture ratio for each ticker vs benchmark.
    fn up_capture(&self) -> Vec<f64> {
        self.inner.up_capture()
    }

    /// Down-capture ratio for each ticker vs benchmark.
    fn down_capture(&self) -> Vec<f64> {
        self.inner.down_capture()
    }

    /// Capture ratio for each ticker vs benchmark.
    fn capture_ratio(&self) -> Vec<f64> {
        self.inner.capture_ratio()
    }

    /// Omega ratio for each ticker.
    #[pyo3(signature = (threshold = 0.0))]
    fn omega_ratio(&self, threshold: f64) -> Vec<f64> {
        self.inner.omega_ratio(threshold)
    }

    /// Treynor ratio for each ticker.
    #[pyo3(signature = (risk_free_rate = 0.0))]
    fn treynor(&self, risk_free_rate: f64) -> Vec<f64> {
        self.inner.treynor(risk_free_rate)
    }

    /// Gain-to-pain ratio for each ticker.
    fn gain_to_pain(&self) -> Vec<f64> {
        self.inner.gain_to_pain()
    }

    /// Ulcer index for each ticker.
    fn ulcer_index(&self) -> Vec<f64> {
        self.inner.ulcer_index()
    }

    /// Martin ratio for each ticker.
    fn martin_ratio(&self) -> Vec<f64> {
        self.inner.martin_ratio()
    }

    /// Recovery factor for each ticker.
    fn recovery_factor(&self) -> Vec<f64> {
        self.inner.recovery_factor()
    }

    /// Pain index for each ticker.
    fn pain_index(&self) -> Vec<f64> {
        self.inner.pain_index()
    }

    /// Pain ratio for each ticker.
    #[pyo3(signature = (risk_free_rate = 0.0))]
    fn pain_ratio(&self, risk_free_rate: f64) -> Vec<f64> {
        self.inner.pain_ratio(risk_free_rate)
    }

    /// Tail ratio for each ticker.
    #[pyo3(signature = (confidence = 0.95))]
    fn tail_ratio(&self, confidence: f64) -> Vec<f64> {
        self.inner.tail_ratio(confidence)
    }

    /// R-squared for each ticker vs benchmark.
    fn r_squared(&self) -> Vec<f64> {
        self.inner.r_squared()
    }

    /// Batting average for each ticker vs benchmark.
    fn batting_average(&self) -> Vec<f64> {
        self.inner.batting_average()
    }

    /// Parametric VaR for each ticker.
    #[pyo3(signature = (confidence = 0.95))]
    fn parametric_var(&self, confidence: f64) -> Vec<f64> {
        self.inner.parametric_var(confidence)
    }

    /// Cornish-Fisher VaR for each ticker.
    #[pyo3(signature = (confidence = 0.95))]
    fn cornish_fisher_var(&self, confidence: f64) -> Vec<f64> {
        self.inner.cornish_fisher_var(confidence)
    }

    /// CDaR for each ticker.
    #[pyo3(signature = (confidence = 0.95))]
    fn cdar(&self, confidence: f64) -> Vec<f64> {
        self.inner.cdar(confidence)
    }

    /// M-squared for each ticker.
    #[pyo3(signature = (risk_free_rate = 0.0))]
    fn m_squared(&self, risk_free_rate: f64) -> Vec<f64> {
        self.inner.m_squared(risk_free_rate)
    }

    /// Modified Sharpe ratio for each ticker.
    #[pyo3(signature = (risk_free_rate = 0.0, confidence = 0.95))]
    fn modified_sharpe(&self, risk_free_rate: f64, confidence: f64) -> Vec<f64> {
        self.inner.modified_sharpe(risk_free_rate, confidence)
    }

    /// Sterling ratio for each ticker.
    #[pyo3(signature = (risk_free_rate = 0.0, n = 5))]
    fn sterling_ratio(&self, risk_free_rate: f64, n: usize) -> Vec<f64> {
        self.inner.sterling_ratio(risk_free_rate, n)
    }

    /// Burke ratio for each ticker.
    #[pyo3(signature = (risk_free_rate = 0.0, n = 5))]
    fn burke_ratio(&self, risk_free_rate: f64, n: usize) -> Vec<f64> {
        self.inner.burke_ratio(risk_free_rate, n)
    }

    // -- Vector-per-ticker methods --

    /// Cumulative returns for each ticker.
    fn cumulative_returns(&self) -> Vec<Vec<f64>> {
        self.inner.cumulative_returns()
    }

    /// Drawdown series for each ticker.
    fn drawdown_series(&self) -> Vec<Vec<f64>> {
        self.inner.drawdown_series()
    }

    /// Correlation matrix across all tickers.
    fn correlation_matrix(&self) -> Vec<Vec<f64>> {
        self.inner.correlation_matrix()
    }

    /// Cumulative returns outperformance vs benchmark.
    fn cumulative_returns_outperformance(&self) -> Vec<Vec<f64>> {
        self.inner.cumulative_returns_outperformance()
    }

    /// Drawdown outperformance vs benchmark.
    fn drawdown_outperformance(&self) -> Vec<Vec<f64>> {
        self.inner.drawdown_outperformance()
    }

    /// Excess returns over a risk-free rate series.
    #[pyo3(signature = (rf, nperiods = None))]
    fn excess_returns(&self, rf: Vec<f64>, nperiods: Option<f64>) -> Vec<Vec<f64>> {
        self.inner.excess_returns(&rf, nperiods)
    }

    // -- Per-ticker indexed methods --

    /// Beta for each ticker vs benchmark.
    fn beta(&self) -> Vec<PyBetaResult> {
        self.inner
            .beta()
            .into_iter()
            .map(|b| PyBetaResult { inner: b })
            .collect()
    }

    /// Greeks (alpha, beta, R²) for each ticker vs benchmark.
    fn greeks(&self) -> Vec<PyGreeksResult> {
        self.inner
            .greeks()
            .into_iter()
            .map(|g| PyGreeksResult { inner: g })
            .collect()
    }

    /// Rolling greeks for a specific ticker.
    #[pyo3(signature = (ticker_idx, window = 63))]
    fn rolling_greeks(&self, ticker_idx: usize, window: usize) -> PyRollingGreeks {
        PyRollingGreeks {
            inner: self.inner.rolling_greeks(ticker_idx, window),
        }
    }

    /// Rolling volatility for a specific ticker.
    #[pyo3(signature = (ticker_idx, window = 63))]
    fn rolling_volatility(&self, ticker_idx: usize, window: usize) -> PyRollingVolatility {
        PyRollingVolatility {
            inner: self.inner.rolling_volatility(ticker_idx, window),
        }
    }

    /// Rolling Sortino for a specific ticker.
    #[pyo3(signature = (ticker_idx, window = 63))]
    fn rolling_sortino(&self, ticker_idx: usize, window: usize) -> PyRollingSortino {
        PyRollingSortino {
            inner: self.inner.rolling_sortino(ticker_idx, window),
        }
    }

    /// Rolling Sharpe for a specific ticker.
    #[pyo3(signature = (ticker_idx, window = 63, risk_free_rate = 0.0))]
    fn rolling_sharpe(
        &self,
        ticker_idx: usize,
        window: usize,
        risk_free_rate: f64,
    ) -> PyRollingSharpe {
        PyRollingSharpe {
            inner: self
                .inner
                .rolling_sharpe(ticker_idx, window, risk_free_rate),
        }
    }

    /// Drawdown episodes for a specific ticker.
    #[pyo3(signature = (ticker_idx, n = 5))]
    fn drawdown_details(&self, ticker_idx: usize, n: usize) -> Vec<PyDrawdownEpisode> {
        self.inner
            .drawdown_details(ticker_idx, n)
            .into_iter()
            .map(|e| PyDrawdownEpisode { inner: e })
            .collect()
    }

    /// Stats during benchmark drawdown episodes.
    #[pyo3(signature = (n = 5))]
    fn stats_during_bench_drawdowns(&self, n: usize) -> Vec<PyDrawdownEpisode> {
        self.inner
            .stats_during_bench_drawdowns(n)
            .into_iter()
            .map(|e| PyDrawdownEpisode { inner: e })
            .collect()
    }

    /// Multi-factor regression for a specific ticker.
    fn multi_factor_greeks(
        &self,
        ticker_idx: usize,
        factor_returns: Vec<Vec<f64>>,
    ) -> PyResult<PyMultiFactorResult> {
        let refs: Vec<&[f64]> = factor_returns.iter().map(|v| v.as_slice()).collect();
        self.inner
            .multi_factor_greeks(ticker_idx, &refs)
            .map(|r| PyMultiFactorResult { inner: r })
            .map_err(core_to_py)
    }

    /// Ruin estimation for each ticker.
    fn estimate_ruin(
        &self,
        definition: &PyRuinDefinition,
        model: &PyRuinModel,
    ) -> Vec<PyRuinEstimate> {
        self.inner
            .estimate_ruin(definition.inner, &model.inner)
            .into_iter()
            .map(|e| PyRuinEstimate { inner: e })
            .collect()
    }

    /// Period-to-date lookback returns.
    fn lookback_returns(
        &self,
        ref_date: Bound<'_, PyAny>,
        fiscal_year_start_month: Option<u8>,
    ) -> PyResult<PyLookbackReturns> {
        let d = py_to_date(&ref_date)?;
        let fc = make_fiscal_config(fiscal_year_start_month);
        Ok(PyLookbackReturns {
            inner: self.inner.lookback_returns(d, fc),
        })
    }

    /// Period statistics for a specific ticker at a given aggregation frequency.
    #[pyo3(signature = (ticker_idx, agg_freq = "monthly", fiscal_year_start_month = None))]
    fn period_stats(
        &self,
        ticker_idx: usize,
        agg_freq: &str,
        fiscal_year_start_month: Option<u8>,
    ) -> PyResult<PyPeriodStats> {
        let pk = parse_freq(agg_freq)?;
        let fc = make_fiscal_config(fiscal_year_start_month);
        Ok(PyPeriodStats {
            inner: self.inner.period_stats(ticker_idx, pk, fc),
        })
    }

    // -- DataFrame export methods --

    /// Summary statistics for all tickers as a pandas ``DataFrame``.
    ///
    /// Returns a DataFrame with one row per ticker and columns for each
    /// scalar metric (CAGR, volatility, Sharpe, max drawdown, etc.).
    #[pyo3(signature = (risk_free_rate = 0.0, confidence = 0.95))]
    fn summary_to_dataframe<'py>(
        &self,
        py: Python<'py>,
        risk_free_rate: f64,
        confidence: f64,
    ) -> PyResult<Bound<'py, PyAny>> {
        let data = PyDict::new(py);
        data.set_item("cagr", self.inner.cagr())?;
        data.set_item("mean_return", self.inner.mean_return(true))?;
        data.set_item("volatility", self.inner.volatility(true))?;
        data.set_item("sharpe", self.inner.sharpe(risk_free_rate))?;
        data.set_item("sortino", self.inner.sortino())?;
        data.set_item("calmar", self.inner.calmar())?;
        data.set_item("max_drawdown", self.inner.max_drawdown())?;
        data.set_item("value_at_risk", self.inner.value_at_risk(confidence))?;
        data.set_item(
            "expected_shortfall",
            self.inner.expected_shortfall(confidence),
        )?;
        data.set_item("tracking_error", self.inner.tracking_error())?;
        data.set_item("information_ratio", self.inner.information_ratio())?;
        data.set_item("skewness", self.inner.skewness())?;
        data.set_item("kurtosis", self.inner.kurtosis())?;
        data.set_item("geometric_mean", self.inner.geometric_mean())?;
        data.set_item("downside_deviation", self.inner.downside_deviation(0.0))?;
        data.set_item("omega_ratio", self.inner.omega_ratio(0.0))?;
        data.set_item("gain_to_pain", self.inner.gain_to_pain())?;
        data.set_item("ulcer_index", self.inner.ulcer_index())?;
        data.set_item("pain_index", self.inner.pain_index())?;
        data.set_item("recovery_factor", self.inner.recovery_factor())?;
        data.set_item("tail_ratio", self.inner.tail_ratio(confidence))?;
        data.set_item("r_squared", self.inner.r_squared())?;

        let names = self.inner.ticker_names();
        let index: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
        let idx = index.into_pyobject(py)?.into_any();
        dict_to_dataframe(py, &data, Some(idx))
    }

    /// Cumulative returns for all tickers as a pandas ``DataFrame``.
    ///
    /// Returns a DataFrame with a date index and one column per ticker.
    fn cumulative_returns_to_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let data = PyDict::new(py);
        let names = self.inner.ticker_names();
        let cum_rets = self.inner.cumulative_returns();
        for (name, series) in names.iter().zip(cum_rets.iter()) {
            data.set_item(name, series)?;
        }
        let dates = dates_to_pylist(py, self.inner.active_dates())?;
        let idx = dates.into_pyobject(py)?.into_any();
        dict_to_dataframe(py, &data, Some(idx))
    }

    /// Drawdown series for all tickers as a pandas ``DataFrame``.
    ///
    /// Returns a DataFrame with a date index and one column per ticker.
    fn drawdown_series_to_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let data = PyDict::new(py);
        let names = self.inner.ticker_names();
        let dd = self.inner.drawdown_series();
        for (name, series) in names.iter().zip(dd.iter()) {
            data.set_item(name, series)?;
        }
        let dates = dates_to_pylist(py, self.inner.active_dates())?;
        let idx = dates.into_pyobject(py)?.into_any();
        dict_to_dataframe(py, &data, Some(idx))
    }

    /// Correlation matrix as a pandas ``DataFrame``.
    ///
    /// Returns a ticker × ticker matrix with ticker names as index and columns.
    fn correlation_to_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let names = self.inner.ticker_names();
        let matrix = self.inner.correlation_matrix();

        let pd = py.import("pandas")?;
        let kwargs = PyDict::new(py);
        let idx: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
        let cols: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
        kwargs.set_item("index", idx)?;
        kwargs.set_item("columns", cols)?;
        pd.call_method("DataFrame", (matrix,), Some(&kwargs))
    }

    /// Top-N drawdown episodes for a ticker as a pandas ``DataFrame``.
    ///
    /// Columns: start, valley, end, duration_days, max_drawdown,
    /// near_recovery_threshold.
    #[pyo3(signature = (ticker_idx, n = 5))]
    fn drawdown_details_to_dataframe<'py>(
        &self,
        py: Python<'py>,
        ticker_idx: usize,
        n: usize,
    ) -> PyResult<Bound<'py, PyAny>> {
        let episodes = self.inner.drawdown_details(ticker_idx, n);
        let data = PyDict::new(py);
        let starts: PyResult<Vec<_>> = episodes.iter().map(|e| date_to_py(py, e.start)).collect();
        let valleys: PyResult<Vec<_>> = episodes.iter().map(|e| date_to_py(py, e.valley)).collect();
        let ends: PyResult<Vec<_>> = episodes
            .iter()
            .map(|e| match e.end {
                Some(d) => date_to_py(py, d).map(|v| v.into_any()),
                None => Ok(py.None().into_bound(py)),
            })
            .collect();

        data.set_item("start", starts?)?;
        data.set_item("valley", valleys?)?;
        data.set_item("end", ends?)?;
        data.set_item(
            "duration_days",
            episodes.iter().map(|e| e.duration_days).collect::<Vec<_>>(),
        )?;
        data.set_item(
            "max_drawdown",
            episodes.iter().map(|e| e.max_drawdown).collect::<Vec<_>>(),
        )?;
        data.set_item(
            "near_recovery_threshold",
            episodes
                .iter()
                .map(|e| e.near_recovery_threshold)
                .collect::<Vec<_>>(),
        )?;
        dict_to_dataframe(py, &data, None)
    }

    /// Period-to-date lookback returns as a pandas ``DataFrame``.
    ///
    /// Returns a DataFrame with ticker names as index and columns:
    /// mtd, qtd, ytd (and fytd when a fiscal config is given).
    #[pyo3(signature = (ref_date, fiscal_year_start_month = None))]
    fn lookback_returns_to_dataframe<'py>(
        &self,
        py: Python<'py>,
        ref_date: Bound<'_, PyAny>,
        fiscal_year_start_month: Option<u8>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let d = py_to_date(&ref_date)?;
        let fc = make_fiscal_config(fiscal_year_start_month);
        let lb = self.inner.lookback_returns(d, fc);

        let data = PyDict::new(py);
        data.set_item("mtd", &lb.mtd)?;
        data.set_item("qtd", &lb.qtd)?;
        data.set_item("ytd", &lb.ytd)?;
        if let Some(ref fytd) = lb.fytd {
            data.set_item("fytd", fytd)?;
        }

        let names = self.inner.ticker_names();
        let index: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
        let idx = index.into_pyobject(py)?.into_any();
        dict_to_dataframe(py, &data, Some(idx))
    }
}

pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyPerformance>()?;
    Ok(())
}
