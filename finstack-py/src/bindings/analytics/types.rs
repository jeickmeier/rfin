//! Result structs and enums for the analytics domain.

use std::str::FromStr;

use crate::bindings::core::dates::utils::{date_to_py, py_to_date};
use crate::bindings::pandas_utils::{dates_to_pylist, dict_to_dataframe};
use finstack_analytics as fa;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyType};

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
// CagrBasis
// ---------------------------------------------------------------------------

fn parse_cagr_convention(
    convention: Option<&str>,
) -> PyResult<fa::risk_metrics::AnnualizationConvention> {
    fa::risk_metrics::AnnualizationConvention::from_str(convention.unwrap_or("act365_25"))
        .map_err(pyo3::exceptions::PyValueError::new_err)
}

/// CAGR annualization basis.
#[pyclass(
    name = "CagrBasis",
    module = "finstack.analytics",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub struct PyCagrBasis {
    pub(super) inner: fa::risk_metrics::CagrBasis,
}

#[pymethods]
impl PyCagrBasis {
    /// Build a factor-based CAGR basis from periods per year.
    #[classmethod]
    #[pyo3(text_signature = "(cls, ann_factor)")]
    fn factor(_cls: &Bound<'_, PyType>, ann_factor: f64) -> Self {
        Self {
            inner: fa::risk_metrics::CagrBasis::factor(ann_factor),
        }
    }

    /// Build a date-based CAGR basis from start/end dates.
    #[classmethod]
    #[pyo3(signature = (start, end, convention = None), text_signature = "(cls, start, end, convention=None)")]
    fn dates(
        _cls: &Bound<'_, PyType>,
        start: Bound<'_, PyAny>,
        end: Bound<'_, PyAny>,
        convention: Option<&str>,
    ) -> PyResult<Self> {
        Ok(Self {
            inner: fa::risk_metrics::CagrBasis::dates_with_convention(
                py_to_date(&start)?,
                py_to_date(&end)?,
                parse_cagr_convention(convention)?,
            ),
        })
    }

    fn __repr__(&self) -> String {
        format!("CagrBasis({:?})", self.inner)
    }
}

// ---------------------------------------------------------------------------
// Ruin types
// ---------------------------------------------------------------------------

/// Definition of a ruin event for Monte Carlo ruin estimation.
#[pyclass(
    name = "RuinDefinition",
    module = "finstack.analytics",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub struct PyRuinDefinition {
    pub(super) inner: fa::risk_metrics::RuinDefinition,
}

#[pymethods]
impl PyRuinDefinition {
    /// Ruin occurs if wealth falls below ``floor_fraction`` of initial wealth.
    #[classmethod]
    #[pyo3(text_signature = "(cls, floor_fraction)")]
    fn wealth_floor(_cls: &Bound<'_, PyType>, floor_fraction: f64) -> Self {
        Self {
            inner: fa::risk_metrics::RuinDefinition::WealthFloor { floor_fraction },
        }
    }

    /// Ruin occurs if terminal wealth is below ``floor_fraction`` of initial.
    #[classmethod]
    #[pyo3(text_signature = "(cls, floor_fraction)")]
    fn terminal_floor(_cls: &Bound<'_, PyType>, floor_fraction: f64) -> Self {
        Self {
            inner: fa::risk_metrics::RuinDefinition::TerminalFloor { floor_fraction },
        }
    }

    /// Ruin occurs if drawdown exceeds ``max_drawdown`` (positive threshold).
    #[classmethod]
    #[pyo3(text_signature = "(cls, max_drawdown)")]
    fn drawdown_breach(_cls: &Bound<'_, PyType>, max_drawdown: f64) -> Self {
        Self {
            inner: fa::risk_metrics::RuinDefinition::DrawdownBreach { max_drawdown },
        }
    }

    fn __repr__(&self) -> String {
        format!("RuinDefinition({:?})", self.inner)
    }
}

/// Configuration for Monte Carlo ruin estimation.
#[pyclass(
    name = "RuinModel",
    module = "finstack.analytics",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub struct PyRuinModel {
    pub(super) inner: fa::risk_metrics::RuinModel,
}

#[pymethods]
impl PyRuinModel {
    /// Create a ruin simulation model.
    #[new]
    #[pyo3(signature = (horizon_periods=252, n_paths=10_000, block_size=5, seed=42, confidence_level=0.95))]
    fn new(
        horizon_periods: usize,
        n_paths: usize,
        block_size: usize,
        seed: u64,
        confidence_level: f64,
    ) -> Self {
        Self {
            inner: fa::risk_metrics::RuinModel {
                horizon_periods,
                n_paths,
                block_size,
                seed,
                confidence_level,
            },
        }
    }

    /// Number of forward periods to simulate.
    #[getter]
    fn horizon_periods(&self) -> usize {
        self.inner.horizon_periods
    }
    /// Number of Monte Carlo paths.
    #[getter]
    fn n_paths(&self) -> usize {
        self.inner.n_paths
    }
    /// Bootstrap block size.
    #[getter]
    fn block_size(&self) -> usize {
        self.inner.block_size
    }
    /// RNG seed.
    #[getter]
    fn seed(&self) -> u64 {
        self.inner.seed
    }
    /// Confidence level for the CI.
    #[getter]
    fn confidence_level(&self) -> f64 {
        self.inner.confidence_level
    }

    fn __repr__(&self) -> String {
        format!(
            "RuinModel(horizon={}, paths={}, seed={})",
            self.inner.horizon_periods, self.inner.n_paths, self.inner.seed
        )
    }
}

/// Monte Carlo ruin probability estimate with confidence interval.
#[pyclass(name = "RuinEstimate", module = "finstack.analytics", frozen)]
pub struct PyRuinEstimate {
    pub(super) inner: fa::risk_metrics::RuinEstimate,
}

#[pymethods]
impl PyRuinEstimate {
    /// Estimated ruin probability.
    #[getter]
    fn probability(&self) -> f64 {
        self.inner.probability
    }
    /// Standard error of the estimate.
    #[getter]
    fn std_err(&self) -> f64 {
        self.inner.std_err
    }
    /// Lower confidence bound.
    #[getter]
    fn ci_lower(&self) -> f64 {
        self.inner.ci_lower
    }
    /// Upper confidence bound.
    #[getter]
    fn ci_upper(&self) -> f64 {
        self.inner.ci_upper
    }

    fn __repr__(&self) -> String {
        format!(
            "RuinEstimate(p={:.4}, se={:.4}, ci=[{:.4}, {:.4}])",
            self.inner.probability, self.inner.std_err, self.inner.ci_lower, self.inner.ci_upper
        )
    }
}

// ---------------------------------------------------------------------------
// BenchmarkAlignmentPolicy
// ---------------------------------------------------------------------------

/// Policy for handling missing dates during benchmark alignment.
#[pyclass(
    name = "BenchmarkAlignmentPolicy",
    module = "finstack.analytics",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub struct PyBenchmarkAlignmentPolicy {
    pub(super) inner: fa::benchmark::BenchmarkAlignmentPolicy,
}

#[pymethods]
impl PyBenchmarkAlignmentPolicy {
    /// Fill missing benchmark dates with zero returns.
    #[classmethod]
    fn zero_on_missing(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: fa::benchmark::BenchmarkAlignmentPolicy::ZeroReturnOnMissingDates,
        }
    }

    /// Raise an error if benchmark dates don't cover all target dates.
    #[classmethod]
    fn error_on_missing(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: fa::benchmark::BenchmarkAlignmentPolicy::ErrorOnMissingDates,
        }
    }

    fn __repr__(&self) -> String {
        format!("BenchmarkAlignmentPolicy({:?})", self.inner)
    }
}

// ---------------------------------------------------------------------------
// KupiecResult
// ---------------------------------------------------------------------------

/// Result of the Kupiec Proportion of Failures (POF) test.
#[pyclass(name = "KupiecResult", module = "finstack.analytics", frozen)]
pub struct PyKupiecResult {
    pub(super) inner: fa::backtesting::KupiecResult,
}

#[pymethods]
impl PyKupiecResult {
    /// Likelihood-ratio test statistic LR_uc.
    #[getter]
    fn lr_statistic(&self) -> f64 {
        self.inner.lr_statistic
    }
    /// p-value from chi-squared(1) distribution.
    #[getter]
    fn p_value(&self) -> f64 {
        self.inner.p_value
    }
    /// Number of observed VaR breaches.
    #[getter]
    fn breach_count(&self) -> usize {
        self.inner.breach_count
    }
    /// Expected number of breaches under H0.
    #[getter]
    fn expected_count(&self) -> f64 {
        self.inner.expected_count
    }
    /// Total number of observations.
    #[getter]
    fn total_observations(&self) -> usize {
        self.inner.total_observations
    }
    /// Observed breach rate.
    #[getter]
    fn observed_rate(&self) -> f64 {
        self.inner.observed_rate
    }
    /// Whether H0 is rejected at 5% significance.
    #[getter]
    fn reject_h0_5pct(&self) -> bool {
        self.inner.reject_h0_5pct
    }

    fn __repr__(&self) -> String {
        format!(
            "KupiecResult(lr={:.4}, p={:.4}, breaches={}/{})",
            self.inner.lr_statistic,
            self.inner.p_value,
            self.inner.breach_count,
            self.inner.total_observations
        )
    }
}

// ---------------------------------------------------------------------------
// ChristoffersenResult
// ---------------------------------------------------------------------------

/// Result of the Christoffersen conditional coverage test.
#[pyclass(name = "ChristoffersenResult", module = "finstack.analytics", frozen)]
pub struct PyChristoffersenResult {
    pub(super) inner: fa::backtesting::ChristoffersenResult,
}

#[pymethods]
impl PyChristoffersenResult {
    /// Unconditional coverage component (Kupiec LR_uc).
    #[getter]
    fn lr_uc(&self) -> f64 {
        self.inner.lr_uc
    }
    /// Independence component LR_ind.
    #[getter]
    fn lr_ind(&self) -> f64 {
        self.inner.lr_ind
    }
    /// Joint conditional coverage statistic LR_cc.
    #[getter]
    fn lr_cc(&self) -> f64 {
        self.inner.lr_cc
    }
    /// p-value for unconditional coverage test.
    #[getter]
    fn p_value_uc(&self) -> f64 {
        self.inner.p_value_uc
    }
    /// p-value for independence test.
    #[getter]
    fn p_value_ind(&self) -> f64 {
        self.inner.p_value_ind
    }
    /// p-value for joint conditional coverage test.
    #[getter]
    fn p_value_cc(&self) -> f64 {
        self.inner.p_value_cc
    }
    /// Transition matrix counts [n00, n01, n10, n11].
    #[getter]
    fn transition_counts(&self) -> Vec<usize> {
        self.inner.transition_counts.to_vec()
    }
    /// Whether H0 (joint) is rejected at 5% significance.
    #[getter]
    fn reject_h0_5pct(&self) -> bool {
        self.inner.reject_h0_5pct
    }

    fn __repr__(&self) -> String {
        format!(
            "ChristoffersenResult(lr_cc={:.4}, p_cc={:.4})",
            self.inner.lr_cc, self.inner.p_value_cc
        )
    }
}

// ---------------------------------------------------------------------------
// TrafficLightResult
// ---------------------------------------------------------------------------

/// Basel traffic-light assessment result.
#[pyclass(name = "TrafficLightResult", module = "finstack.analytics", frozen)]
pub struct PyTrafficLightResult {
    pub(super) inner: fa::backtesting::TrafficLightResult,
}

#[pymethods]
impl PyTrafficLightResult {
    /// Assigned zone name (``"Green"``, ``"Yellow"``, or ``"Red"``).
    #[getter]
    fn zone(&self) -> &str {
        match self.inner.zone {
            fa::backtesting::TrafficLightZone::Green => "Green",
            fa::backtesting::TrafficLightZone::Yellow => "Yellow",
            fa::backtesting::TrafficLightZone::Red => "Red",
        }
    }
    /// Number of exceptions in the evaluation window.
    #[getter]
    fn exceptions(&self) -> usize {
        self.inner.exceptions
    }
    /// Capital multiplier for the market risk charge.
    #[getter]
    fn capital_multiplier(&self) -> f64 {
        self.inner.capital_multiplier
    }
    /// Window size used.
    #[getter]
    fn window_size(&self) -> usize {
        self.inner.window_size
    }
    /// VaR confidence level used.
    #[getter]
    fn confidence(&self) -> f64 {
        self.inner.confidence
    }

    fn __repr__(&self) -> String {
        format!(
            "TrafficLightResult(zone={}, exceptions={}, multiplier={:.2})",
            self.zone(),
            self.inner.exceptions,
            self.inner.capital_multiplier
        )
    }
}

// ---------------------------------------------------------------------------
// BacktestResult
// ---------------------------------------------------------------------------

/// Full backtest result aggregating all statistical tests.
#[pyclass(name = "BacktestResult", module = "finstack.analytics", frozen)]
pub struct PyBacktestResult {
    pub(super) inner: fa::backtesting::BacktestResult,
}

#[pymethods]
impl PyBacktestResult {
    /// Kupiec unconditional coverage test result.
    #[getter]
    fn kupiec(&self) -> PyKupiecResult {
        PyKupiecResult {
            inner: self.inner.kupiec.clone(),
        }
    }
    /// Christoffersen conditional coverage test result.
    #[getter]
    fn christoffersen(&self) -> PyChristoffersenResult {
        PyChristoffersenResult {
            inner: self.inner.christoffersen.clone(),
        }
    }
    /// Basel traffic-light classification result.
    #[getter]
    fn traffic_light(&self) -> PyTrafficLightResult {
        PyTrafficLightResult {
            inner: self.inner.traffic_light.clone(),
        }
    }
    /// Number of observed VaR breaches.
    #[getter]
    fn breach_count(&self) -> usize {
        self.inner.kupiec.breach_count
    }
    /// VaR confidence level used for the backtest.
    #[getter]
    fn confidence(&self) -> f64 {
        self.inner.confidence
    }

    fn __repr__(&self) -> String {
        format!(
            "BacktestResult(breaches={}, confidence={:.2})",
            self.inner.kupiec.breach_count, self.inner.confidence
        )
    }
}

// ---------------------------------------------------------------------------
// PnlExplanation
// ---------------------------------------------------------------------------

/// P&L explanation metrics for VaR model validation.
#[pyclass(name = "PnlExplanation", module = "finstack.analytics", frozen)]
pub struct PyPnlExplanation {
    /// Wrapped Rust value.
    pub(super) inner: fa::backtesting::PnlExplanation,
}

#[pymethods]
impl PyPnlExplanation {
    /// Mean normalized unexplained P&L relative to VaR.
    #[getter]
    fn explanation_ratio(&self) -> f64 {
        self.inner.explanation_ratio
    }

    /// Aggregate unexplained P&L ratio using sums across the full sample.
    #[getter]
    fn aggregate_explanation_ratio(&self) -> f64 {
        self.inner.aggregate_explanation_ratio
    }

    /// Mean absolute unexplained P&L.
    #[getter]
    fn mean_abs_unexplained(&self) -> f64 {
        self.inner.mean_abs_unexplained
    }

    /// Standard deviation of unexplained P&L.
    #[getter]
    fn std_unexplained(&self) -> f64 {
        self.inner.std_unexplained
    }

    /// Number of observations used.
    #[getter]
    fn n(&self) -> usize {
        self.inner.n
    }

    fn __repr__(&self) -> String {
        format!(
            "PnlExplanation(ratio={:.4}, agg_ratio={:.4}, mean_abs={:.4}, n={})",
            self.inner.explanation_ratio,
            self.inner.aggregate_explanation_ratio,
            self.inner.mean_abs_unexplained,
            self.inner.n
        )
    }
}

// ---------------------------------------------------------------------------
// MultiModelComparison
// ---------------------------------------------------------------------------

/// Side-by-side VaR backtest comparison across multiple model methods.
#[pyclass(name = "MultiModelComparison", module = "finstack.analytics", frozen)]
pub struct PyMultiModelComparison {
    /// Wrapped Rust value.
    pub(super) inner: fa::backtesting::MultiModelComparison,
}

#[pymethods]
impl PyMultiModelComparison {
    /// Model-labelled backtest results.
    #[getter]
    fn results(&self) -> Vec<(String, PyBacktestResult)> {
        self.inner
            .results
            .iter()
            .map(|(method, result)| {
                (
                    match method {
                        fa::backtesting::VarMethod::Historical => "Historical",
                        fa::backtesting::VarMethod::Parametric => "Parametric",
                        fa::backtesting::VarMethod::CornishFisher => "CornishFisher",
                    }
                    .to_string(),
                    PyBacktestResult {
                        inner: result.clone(),
                    },
                )
            })
            .collect()
    }

    fn __repr__(&self) -> String {
        format!("MultiModelComparison(models={})", self.inner.results.len())
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
    m.add_class::<PyCagrBasis>()?;
    m.add_class::<PyRuinDefinition>()?;
    m.add_class::<PyRuinModel>()?;
    m.add_class::<PyRuinEstimate>()?;
    m.add_class::<PyBenchmarkAlignmentPolicy>()?;
    m.add_class::<PyKupiecResult>()?;
    m.add_class::<PyChristoffersenResult>()?;
    m.add_class::<PyTrafficLightResult>()?;
    m.add_class::<PyBacktestResult>()?;
    m.add_class::<PyPnlExplanation>()?;
    m.add_class::<PyMultiModelComparison>()?;
    let _ = py;
    Ok(())
}
