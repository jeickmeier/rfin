//! Result structs and enums for the analytics domain.

use crate::bindings::core::dates::utils::date_to_py;
use crate::bindings::pandas_utils::{dates_to_pylist, dict_to_dataframe};
use finstack_analytics as fa;
use pyo3::prelude::*;
use pyo3::types::PyDict;

// ---------------------------------------------------------------------------
// PeriodStats
// ---------------------------------------------------------------------------

/// Aggregated statistics for grouped periodic returns.
#[pyclass(name = "PeriodStats", module = "finstack.analytics", frozen)]
pub struct PyPeriodStats {
    pub(super) inner: fa::PeriodStats,
}

#[pymethods]
impl PyPeriodStats {
    /// Best period return.
    #[getter]
    fn best(&self) -> f64 {
        self.inner.best
    }
    /// Worst period return.
    #[getter]
    fn worst(&self) -> f64 {
        self.inner.worst
    }
    /// Longest consecutive winning streak.
    #[getter]
    fn consecutive_wins(&self) -> usize {
        self.inner.consecutive_wins
    }
    /// Longest consecutive losing streak.
    #[getter]
    fn consecutive_losses(&self) -> usize {
        self.inner.consecutive_losses
    }
    /// Fraction of positive-return periods.
    #[getter]
    fn win_rate(&self) -> f64 {
        self.inner.win_rate
    }
    /// Average return across all periods.
    #[getter]
    fn avg_return(&self) -> f64 {
        self.inner.avg_return
    }
    /// Average return of positive periods.
    #[getter]
    fn avg_win(&self) -> f64 {
        self.inner.avg_win
    }
    /// Average return of negative periods.
    #[getter]
    fn avg_loss(&self) -> f64 {
        self.inner.avg_loss
    }
    /// Payoff ratio (avg win / |avg loss|).
    #[getter]
    fn payoff_ratio(&self) -> f64 {
        self.inner.payoff_ratio
    }
    /// Profit factor (gross profits / gross losses).
    #[getter]
    fn profit_factor(&self) -> f64 {
        self.inner.profit_factor
    }
    /// Common-sense ratio (CPC).
    #[getter]
    fn cpc_ratio(&self) -> f64 {
        self.inner.cpc_ratio
    }
    /// Kelly criterion optimal fraction.
    #[getter]
    fn kelly_criterion(&self) -> f64 {
        self.inner.kelly_criterion
    }

    fn __repr__(&self) -> String {
        format!(
            "PeriodStats(win_rate={:.4}, avg_return={:.6})",
            self.inner.win_rate, self.inner.avg_return
        )
    }
}

// ---------------------------------------------------------------------------
// BetaResult
// ---------------------------------------------------------------------------

/// Regression beta with confidence interval.
#[pyclass(name = "BetaResult", module = "finstack.analytics", frozen)]
pub struct PyBetaResult {
    pub(super) inner: fa::benchmark::BetaResult,
}

#[pymethods]
impl PyBetaResult {
    /// Beta coefficient.
    #[getter]
    fn beta(&self) -> f64 {
        self.inner.beta
    }
    /// Standard error of the beta estimate.
    #[getter]
    fn std_err(&self) -> f64 {
        self.inner.std_err
    }
    /// Lower 95% confidence bound.
    #[getter]
    fn ci_lower(&self) -> f64 {
        self.inner.ci_lower
    }
    /// Upper 95% confidence bound.
    #[getter]
    fn ci_upper(&self) -> f64 {
        self.inner.ci_upper
    }

    fn __repr__(&self) -> String {
        format!(
            "BetaResult(beta={:.4}, se={:.4}, ci=[{:.4}, {:.4}])",
            self.inner.beta, self.inner.std_err, self.inner.ci_lower, self.inner.ci_upper
        )
    }
}

// ---------------------------------------------------------------------------
// GreeksResult
// ---------------------------------------------------------------------------

/// Alpha, beta, and R-squared from a single-index regression.
#[pyclass(name = "GreeksResult", module = "finstack.analytics", frozen)]
pub struct PyGreeksResult {
    pub(super) inner: fa::benchmark::GreeksResult,
}

#[pymethods]
impl PyGreeksResult {
    /// Jensen's alpha (annualized).
    #[getter]
    fn alpha(&self) -> f64 {
        self.inner.alpha
    }
    /// Beta coefficient.
    #[getter]
    fn beta(&self) -> f64 {
        self.inner.beta
    }
    /// R-squared.
    #[getter]
    fn r_squared(&self) -> f64 {
        self.inner.r_squared
    }
    /// Adjusted R-squared.
    #[getter]
    fn adjusted_r_squared(&self) -> f64 {
        self.inner.adjusted_r_squared
    }

    fn __repr__(&self) -> String {
        format!(
            "GreeksResult(alpha={:.6}, beta={:.4}, r2={:.4}, adj_r2={:.4})",
            self.inner.alpha, self.inner.beta, self.inner.r_squared, self.inner.adjusted_r_squared
        )
    }
}

// ---------------------------------------------------------------------------
// RollingGreeks
// ---------------------------------------------------------------------------

/// Rolling alpha and beta time series.
#[pyclass(name = "RollingGreeks", module = "finstack.analytics", frozen)]
pub struct PyRollingGreeks {
    pub(super) inner: fa::benchmark::RollingGreeks,
}

#[pymethods]
impl PyRollingGreeks {
    /// Date labels for each rolling window.
    fn dates<'py>(&self, py: Python<'py>) -> PyResult<Vec<Bound<'py, PyAny>>> {
        self.inner
            .dates
            .iter()
            .map(|&d| date_to_py(py, d))
            .collect()
    }
    /// Rolling alpha values.
    #[getter]
    fn alphas(&self) -> Vec<f64> {
        self.inner.alphas.clone()
    }
    /// Rolling beta values.
    #[getter]
    fn betas(&self) -> Vec<f64> {
        self.inner.betas.clone()
    }

    /// Convert to a pandas ``DataFrame`` with date index and alpha/beta columns.
    fn to_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let data = PyDict::new(py);
        data.set_item("alpha", &self.inner.alphas)?;
        data.set_item("beta", &self.inner.betas)?;
        let dates = dates_to_pylist(py, &self.inner.dates)?;
        let idx = dates.into_pyobject(py)?.into_any();
        dict_to_dataframe(py, &data, Some(idx))
    }

    fn __repr__(&self) -> String {
        format!("RollingGreeks(len={})", self.inner.dates.len())
    }
}

// ---------------------------------------------------------------------------
// MultiFactorResult
// ---------------------------------------------------------------------------

/// Multi-factor regression result.
#[pyclass(name = "MultiFactorResult", module = "finstack.analytics", frozen)]
pub struct PyMultiFactorResult {
    pub(super) inner: fa::benchmark::MultiFactorResult,
}

#[pymethods]
impl PyMultiFactorResult {
    /// Intercept (alpha).
    #[getter]
    fn alpha(&self) -> f64 {
        self.inner.alpha
    }
    /// Factor betas.
    #[getter]
    fn betas(&self) -> Vec<f64> {
        self.inner.betas.clone()
    }
    /// R-squared.
    #[getter]
    fn r_squared(&self) -> f64 {
        self.inner.r_squared
    }
    /// Adjusted R-squared.
    #[getter]
    fn adjusted_r_squared(&self) -> f64 {
        self.inner.adjusted_r_squared
    }
    /// Residual volatility.
    #[getter]
    fn residual_vol(&self) -> f64 {
        self.inner.residual_vol
    }

    fn __repr__(&self) -> String {
        format!(
            "MultiFactorResult(alpha={:.6}, r2={:.4}, adj_r2={:.4})",
            self.inner.alpha, self.inner.r_squared, self.inner.adjusted_r_squared
        )
    }
}

// ---------------------------------------------------------------------------
// DrawdownEpisode
// ---------------------------------------------------------------------------

/// A single drawdown episode with timing and depth information.
#[pyclass(name = "DrawdownEpisode", module = "finstack.analytics", frozen)]
pub struct PyDrawdownEpisode {
    pub(super) inner: fa::drawdown::DrawdownEpisode,
}

#[pymethods]
impl PyDrawdownEpisode {
    /// Start date of the drawdown.
    fn start<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        date_to_py(py, self.inner.start)
    }
    /// Date of the maximum drawdown within this episode.
    fn valley<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        date_to_py(py, self.inner.valley)
    }
    /// Recovery date (``None`` if still in drawdown).
    fn end<'py>(&self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyAny>>> {
        match self.inner.end {
            Some(d) => date_to_py(py, d).map(Some),
            None => Ok(None),
        }
    }
    /// Duration in calendar days.
    #[getter]
    fn duration_days(&self) -> i64 {
        self.inner.duration_days
    }
    /// Maximum drawdown depth (negative).
    #[getter]
    fn max_drawdown(&self) -> f64 {
        self.inner.max_drawdown
    }
    /// Near-recovery threshold.
    #[getter]
    fn near_recovery_threshold(&self) -> f64 {
        self.inner.near_recovery_threshold
    }

    fn __repr__(&self) -> String {
        format!(
            "DrawdownEpisode(dd={:.4}, days={})",
            self.inner.max_drawdown, self.inner.duration_days
        )
    }
}

// ---------------------------------------------------------------------------
// LookbackReturns
// ---------------------------------------------------------------------------

/// Period-to-date returns for each ticker.
#[pyclass(name = "LookbackReturns", module = "finstack.analytics", frozen)]
pub struct PyLookbackReturns {
    pub(super) inner: fa::LookbackReturns,
}

#[pymethods]
impl PyLookbackReturns {
    /// Month-to-date returns per ticker.
    #[getter]
    fn mtd(&self) -> Vec<f64> {
        self.inner.mtd.clone()
    }
    /// Quarter-to-date returns per ticker.
    #[getter]
    fn qtd(&self) -> Vec<f64> {
        self.inner.qtd.clone()
    }
    /// Year-to-date returns per ticker.
    #[getter]
    fn ytd(&self) -> Vec<f64> {
        self.inner.ytd.clone()
    }
    /// Fiscal-year-to-date returns (``None`` if no fiscal config).
    #[getter]
    fn fytd(&self) -> Option<Vec<f64>> {
        self.inner.fytd.clone()
    }

    /// Convert to a pandas ``DataFrame`` with ticker names as index.
    ///
    /// Columns: mtd, qtd, ytd (and fytd when available).
    fn to_dataframe<'py>(
        &self,
        py: Python<'py>,
        ticker_names: Vec<String>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let data = PyDict::new(py);
        data.set_item("mtd", &self.inner.mtd)?;
        data.set_item("qtd", &self.inner.qtd)?;
        data.set_item("ytd", &self.inner.ytd)?;
        if let Some(ref fytd) = self.inner.fytd {
            data.set_item("fytd", fytd)?;
        }
        let idx = ticker_names.into_pyobject(py)?.into_any();
        dict_to_dataframe(py, &data, Some(idx))
    }

    fn __repr__(&self) -> String {
        format!(
            "LookbackReturns(mtd_len={}, has_fytd={})",
            self.inner.mtd.len(),
            self.inner.fytd.is_some()
        )
    }
}

// ---------------------------------------------------------------------------
// RollingSharpe / RollingSortino / RollingVolatility
// ---------------------------------------------------------------------------

/// Rolling Sharpe ratio time series.
#[pyclass(name = "RollingSharpe", module = "finstack.analytics", frozen)]
pub struct PyRollingSharpe {
    pub(super) inner: fa::risk_metrics::RollingSharpe,
}

#[pymethods]
impl PyRollingSharpe {
    /// Rolling Sharpe values.
    #[getter]
    fn values(&self) -> Vec<f64> {
        self.inner.values.clone()
    }
    /// Corresponding dates.
    fn dates<'py>(&self, py: Python<'py>) -> PyResult<Vec<Bound<'py, PyAny>>> {
        self.inner
            .dates
            .iter()
            .map(|&d| date_to_py(py, d))
            .collect()
    }

    /// Convert to a pandas ``DataFrame`` with date index and a ``sharpe`` column.
    fn to_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let data = PyDict::new(py);
        data.set_item("sharpe", &self.inner.values)?;
        let dates = dates_to_pylist(py, &self.inner.dates)?;
        let idx = dates.into_pyobject(py)?.into_any();
        dict_to_dataframe(py, &data, Some(idx))
    }

    fn __repr__(&self) -> String {
        format!("RollingSharpe(len={})", self.inner.values.len())
    }
}

/// Rolling Sortino ratio time series.
#[pyclass(name = "RollingSortino", module = "finstack.analytics", frozen)]
pub struct PyRollingSortino {
    pub(super) inner: fa::risk_metrics::RollingSortino,
}

#[pymethods]
impl PyRollingSortino {
    /// Rolling Sortino values.
    #[getter]
    fn values(&self) -> Vec<f64> {
        self.inner.values.clone()
    }
    /// Corresponding dates.
    fn dates<'py>(&self, py: Python<'py>) -> PyResult<Vec<Bound<'py, PyAny>>> {
        self.inner
            .dates
            .iter()
            .map(|&d| date_to_py(py, d))
            .collect()
    }

    /// Convert to a pandas ``DataFrame`` with date index and a ``sortino`` column.
    fn to_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let data = PyDict::new(py);
        data.set_item("sortino", &self.inner.values)?;
        let dates = dates_to_pylist(py, &self.inner.dates)?;
        let idx = dates.into_pyobject(py)?.into_any();
        dict_to_dataframe(py, &data, Some(idx))
    }

    fn __repr__(&self) -> String {
        format!("RollingSortino(len={})", self.inner.values.len())
    }
}

/// Rolling volatility time series.
#[pyclass(name = "RollingVolatility", module = "finstack.analytics", frozen)]
pub struct PyRollingVolatility {
    pub(super) inner: fa::risk_metrics::RollingVolatility,
}

#[pymethods]
impl PyRollingVolatility {
    /// Rolling volatility values.
    #[getter]
    fn values(&self) -> Vec<f64> {
        self.inner.values.clone()
    }
    /// Corresponding dates.
    fn dates<'py>(&self, py: Python<'py>) -> PyResult<Vec<Bound<'py, PyAny>>> {
        self.inner
            .dates
            .iter()
            .map(|&d| date_to_py(py, d))
            .collect()
    }

    /// Convert to a pandas ``DataFrame`` with date index and a ``volatility`` column.
    fn to_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let data = PyDict::new(py);
        data.set_item("volatility", &self.inner.values)?;
        let dates = dates_to_pylist(py, &self.inner.dates)?;
        let idx = dates.into_pyobject(py)?.into_any();
        dict_to_dataframe(py, &data, Some(idx))
    }

    fn __repr__(&self) -> String {
        format!("RollingVolatility(len={})", self.inner.values.len())
    }
}

// ---------------------------------------------------------------------------
// RollingReturns
// ---------------------------------------------------------------------------

/// Rolling total compounded return time series.
#[pyclass(name = "RollingReturns", module = "finstack.analytics", frozen)]
pub struct PyRollingReturns {
    pub(super) inner: fa::risk_metrics::DatedSeries,
}

#[pymethods]
impl PyRollingReturns {
    /// Rolling total-return values, one per completed window.
    #[getter]
    fn values(&self) -> Vec<f64> {
        self.inner.values.clone()
    }
    /// Window-end dates aligned 1:1 with `values`.
    fn dates<'py>(&self, py: Python<'py>) -> PyResult<Vec<Bound<'py, PyAny>>> {
        self.inner
            .dates
            .iter()
            .map(|&d| date_to_py(py, d))
            .collect()
    }

    /// Convert to a pandas ``DataFrame`` with date index and a ``return`` column.
    fn to_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let data = PyDict::new(py);
        data.set_item("return", &self.inner.values)?;
        let dates = dates_to_pylist(py, &self.inner.dates)?;
        let idx = dates.into_pyobject(py)?.into_any();
        dict_to_dataframe(py, &data, Some(idx))
    }

    fn __repr__(&self) -> String {
        format!("RollingReturns(len={})", self.inner.values.len())
    }
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

pub fn register(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyPeriodStats>()?;
    m.add_class::<PyBetaResult>()?;
    m.add_class::<PyGreeksResult>()?;
    m.add_class::<PyRollingGreeks>()?;
    m.add_class::<PyMultiFactorResult>()?;
    m.add_class::<PyDrawdownEpisode>()?;
    m.add_class::<PyLookbackReturns>()?;
    m.add_class::<PyRollingSharpe>()?;
    m.add_class::<PyRollingSortino>()?;
    m.add_class::<PyRollingVolatility>()?;
    m.add_class::<PyRollingReturns>()?;
    let _ = py;
    Ok(())
}
