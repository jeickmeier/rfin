use crate::core::common::labels::normalize_label;
use crate::errors::core_to_py;
use finstack_core::math::stats::{
    correlation as core_correlation, covariance as core_covariance,
    log_returns as core_log_returns, mean as core_mean, mean_var as core_mean_var,
    moment_match as core_moment_match, population_variance as core_population_variance,
    quantile as core_quantile, realized_variance as core_realized_variance,
    realized_variance_ohlc as core_realized_variance_ohlc,
    required_samples as core_required_samples, variance as core_variance, OnlineCovariance,
    OnlineStats, RealizedVarMethod,
};
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule, PyType};
use pyo3::Bound;

#[pyclass(
    name = "RealizedVarMethod",
    module = "finstack.core.math.stats",
    frozen,
    from_py_object
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Methods for calculating realized variance from price or OHLC series.
pub struct PyRealizedVarMethod {
    pub(crate) inner: RealizedVarMethod,
}

impl PyRealizedVarMethod {
    pub(crate) const fn new(inner: RealizedVarMethod) -> Self {
        Self { inner }
    }

    fn label(&self) -> &'static str {
        match self.inner {
            RealizedVarMethod::CloseToClose => "close_to_close",
            RealizedVarMethod::Parkinson => "parkinson",
            RealizedVarMethod::GarmanKlass => "garman_klass",
            RealizedVarMethod::RogersSatchell => "rogers_satchell",
            RealizedVarMethod::YangZhang => "yang_zhang",
        }
    }
}

fn parse_realized_var_method_name(name: &str) -> PyResult<RealizedVarMethod> {
    match normalize_label(name).as_str() {
        "close_to_close" | "close" => Ok(RealizedVarMethod::CloseToClose),
        "parkinson" => Ok(RealizedVarMethod::Parkinson),
        "garman_klass" | "garmanklass" => Ok(RealizedVarMethod::GarmanKlass),
        "rogers_satchell" => Ok(RealizedVarMethod::RogersSatchell),
        "yang_zhang" => Ok(RealizedVarMethod::YangZhang),
        other => Err(PyValueError::new_err(format!(
            "Unknown realized variance method: {other}",
        ))),
    }
}

fn parse_realized_var_method(method: Option<Bound<'_, PyAny>>) -> PyResult<RealizedVarMethod> {
    if let Some(obj) = method {
        if let Ok(wrapper) = obj.extract::<PyRef<PyRealizedVarMethod>>() {
            return Ok(wrapper.inner);
        }
        if let Ok(name) = obj.extract::<&str>() {
            return parse_realized_var_method_name(name);
        }
        return Err(PyTypeError::new_err(
            "Expected RealizedVarMethod or string label",
        ));
    }
    Ok(RealizedVarMethod::CloseToClose)
}

#[pymethods]
impl PyRealizedVarMethod {
    #[classattr]
    const CLOSE_TO_CLOSE: Self = Self {
        inner: RealizedVarMethod::CloseToClose,
    };

    #[classattr]
    const PARKINSON: Self = Self {
        inner: RealizedVarMethod::Parkinson,
    };

    #[classattr]
    const GARMAN_KLASS: Self = Self {
        inner: RealizedVarMethod::GarmanKlass,
    };

    #[classattr]
    const ROGERS_SATCHELL: Self = Self {
        inner: RealizedVarMethod::RogersSatchell,
    };

    #[classattr]
    const YANG_ZHANG: Self = Self {
        inner: RealizedVarMethod::YangZhang,
    };

    #[classmethod]
    #[pyo3(text_signature = "(cls, name)")]
    /// Parse a realized variance method from a snake-/kebab-case label.
    fn from_name(_cls: &Bound<'_, PyType>, name: &str) -> PyResult<Self> {
        Ok(Self::new(parse_realized_var_method_name(name)?))
    }

    #[getter]
    /// Snake-case label for this method.
    fn name(&self) -> &'static str {
        self.label()
    }

    #[getter]
    /// Whether this method requires OHLC data.
    ///
    /// Returns ``True`` for all methods except ``close_to_close``.
    ///
    /// Returns:
    ///     bool: ``True`` if the method needs open/high/low/close data.
    fn requires_ohlc(&self) -> bool {
        self.inner.requires_ohlc()
    }

    fn __repr__(&self) -> String {
        format!("RealizedVarMethod('{}')", self.label())
    }

    fn __str__(&self) -> &'static str {
        self.label()
    }

    fn __hash__(&self) -> isize {
        self.inner as isize
    }
}

#[pyfunction(name = "mean")]
#[pyo3(text_signature = "(data)")]
pub fn mean_py(data: Vec<f64>) -> PyResult<f64> {
    if data.is_empty() {
        return Err(PyValueError::new_err("Data must not be empty"));
    }
    Ok(core_mean(&data))
}

#[pyfunction(name = "variance")]
#[pyo3(text_signature = "(data)")]
pub fn variance_py(data: Vec<f64>) -> PyResult<f64> {
    if data.len() < 2 {
        return Err(PyValueError::new_err("Data must have at least 2 elements"));
    }
    Ok(core_variance(&data))
}

#[pyfunction(name = "covariance")]
#[pyo3(text_signature = "(x, y)")]
pub fn covariance_py(x: Vec<f64>, y: Vec<f64>) -> PyResult<f64> {
    if x.len() != y.len() {
        return Err(PyValueError::new_err("Input arrays must have same length"));
    }
    if x.len() < 2 {
        return Err(PyValueError::new_err("Data must have at least 2 elements"));
    }
    Ok(core_covariance(&x, &y))
}

#[pyfunction(name = "correlation")]
#[pyo3(text_signature = "(x, y)")]
pub fn correlation_py(x: Vec<f64>, y: Vec<f64>) -> PyResult<f64> {
    if x.len() != y.len() {
        return Err(PyValueError::new_err("Input arrays must have same length"));
    }
    if x.len() < 2 {
        return Err(PyValueError::new_err("Data must have at least 2 elements"));
    }
    Ok(core_correlation(&x, &y))
}

#[pyfunction(name = "mean_var")]
#[pyo3(text_signature = "(data)")]
/// Return the (mean, sample_variance) pair for a data series.
///
/// The variance uses an unbiased estimator with ``n - 1`` denominator
/// (Bessel's correction), consistent with :func:`variance`. Requires at
/// least 2 observations; with fewer elements the variance is ``0.0``.
///
/// Parameters
/// ----------
/// data : list[float]
///     Numeric data with at least 2 elements.
///
/// Returns
/// -------
/// tuple[float, float]
///     ``(mean, sample_variance)`` pair.
///
/// Raises
/// ------
/// ValueError
///     If *data* has fewer than 2 elements.
pub fn mean_var_py(data: Vec<f64>) -> PyResult<(f64, f64)> {
    if data.len() < 2 {
        return Err(PyValueError::new_err("Data must have at least 2 elements"));
    }
    Ok(core_mean_var(&data))
}

#[pyfunction(name = "log_returns")]
#[pyo3(text_signature = "(prices)")]
/// Compute log returns from a price series.
///
/// Parameters
/// ----------
/// prices : list[float]
///     Price series ordered in time.
///
/// Returns
/// -------
/// list[float]
///     Log returns ``ln(P_t / P_{t-1})``. Empty list if fewer than 2 prices.
pub fn log_returns_py(prices: Vec<f64>) -> PyResult<Vec<f64>> {
    Ok(core_log_returns(&prices))
}

#[pyfunction(
    name = "realized_variance",
    text_signature = "(prices, method='close_to_close', annualization_factor=252.0)"
)]
/// Calculate realized variance from a close price series.
///
/// Only the ``close_to_close`` method is supported here. OHLC-based
/// estimators (``parkinson``, ``garman_klass``, ``rogers_satchell``,
/// ``yang_zhang``) require intraday high/low/open data and must be
/// called via :func:`realized_variance_ohlc`.
///
/// Parameters
/// ----------
/// prices : list[float]
///     Close prices ordered in time.
/// method : RealizedVarMethod or str, optional
///     Estimation method (default ``'close_to_close'``). OHLC-only
///     methods raise :class:`ValueError`.
/// annualization_factor : float, optional
///     Scaling factor for annualization (default ``252.0``).
///
/// Raises
/// ------
/// ValueError
///     If *method* requires OHLC data.
pub fn realized_variance_py(
    prices: Vec<f64>,
    method: Option<Bound<'_, PyAny>>,
    annualization_factor: Option<f64>,
) -> PyResult<f64> {
    use crate::errors::core_to_py;
    let m = parse_realized_var_method(method)?;
    let af = annualization_factor.unwrap_or(252.0);
    core_realized_variance(&prices, m, af).map_err(core_to_py)
}

#[pyfunction(
    name = "realized_variance_ohlc",
    text_signature = "(open, high, low, close, method='yang_zhang', annualization_factor=252.0)"
)]
/// Calculate realized variance from OHLC data using advanced estimators.
///
/// Parameters
/// ----------
/// open, high, low, close : list[float]
///     OHLC price series of equal length.
/// method : RealizedVarMethod or str, optional
///     Estimation method (default ``'yang_zhang'``).
/// annualization_factor : float, optional
///     Scaling factor for annualization (default ``252.0``).
pub fn realized_variance_ohlc_py(
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    method: Option<Bound<'_, PyAny>>,
    annualization_factor: Option<f64>,
) -> PyResult<f64> {
    let n = open.len();
    if high.len() != n || low.len() != n || close.len() != n {
        return Err(PyValueError::new_err(
            "open, high, low, and close must have the same length",
        ));
    }
    let m = parse_realized_var_method(method)?;
    let af = annualization_factor.unwrap_or(252.0);
    core_realized_variance_ohlc(&open, &high, &low, &close, m, af).map_err(core_to_py)
}

#[pyfunction(name = "population_variance")]
#[pyo3(text_signature = "(data)")]
/// Population variance (n denominator) of a data series.
///
/// Parameters
/// ----------
/// data : list[float]
///     Numeric data. Must not be empty.
///
/// Returns
/// -------
/// float
///     Population variance.
///
/// Raises
/// ------
/// ValueError
///     If *data* is empty.
pub fn population_variance_py(data: Vec<f64>) -> PyResult<f64> {
    if data.is_empty() {
        return Err(PyValueError::new_err("data must not be empty"));
    }
    Ok(core_population_variance(&data))
}

#[pyfunction(name = "quantile")]
#[pyo3(text_signature = "(data, p)")]
/// Empirical quantile (R-7 / NumPy default) via partial sort.
///
/// Parameters
/// ----------
/// data : list[float]
///     Numeric data. Must not be empty.
/// p : float
///     Quantile probability in ``[0, 1]``.
///
/// Returns
/// -------
/// float
///     The *p*-th quantile.
///
/// Raises
/// ------
/// ValueError
///     If *data* is empty or *p* is outside ``[0, 1]``.
pub fn quantile_py(data: Vec<f64>, p: f64) -> PyResult<f64> {
    if data.is_empty() {
        return Err(PyValueError::new_err("data must not be empty"));
    }
    if !(0.0..=1.0).contains(&p) {
        return Err(PyValueError::new_err("p must be in [0, 1]"));
    }
    let mut data = data;
    Ok(core_quantile(&mut data, p))
}

// ====== Online Statistics Classes ======

#[pyclass(name = "OnlineStats", module = "finstack.core.math.stats")]
/// Streaming mean / variance accumulator (Welford's algorithm).
///
/// Use this when you need to compute statistics over a stream of values
/// without storing them all in memory.
pub struct PyOnlineStats {
    inner: OnlineStats,
}

#[pymethods]
impl PyOnlineStats {
    #[new]
    /// Create an empty statistics accumulator.
    fn new() -> Self {
        Self {
            inner: OnlineStats::new(),
        }
    }

    /// Feed a single observation.
    #[pyo3(text_signature = "($self, value)")]
    fn update(&mut self, value: f64) {
        self.inner.update(value);
    }

    /// Merge another ``OnlineStats`` into this one (parallel reduce).
    #[pyo3(text_signature = "($self, other)")]
    fn merge(&mut self, other: &PyOnlineStats) {
        self.inner.merge(&other.inner);
    }

    /// Number of observations seen so far.
    #[getter]
    fn count(&self) -> usize {
        self.inner.count()
    }

    /// Current sample mean.
    #[getter]
    fn mean(&self) -> f64 {
        self.inner.mean()
    }

    /// Current sample variance (unbiased, n-1 denominator).
    #[getter]
    fn variance(&self) -> f64 {
        self.inner.variance()
    }

    /// Current sample standard deviation.
    #[getter]
    fn std_dev(&self) -> f64 {
        self.inner.std_dev()
    }

    /// Standard error of the mean.
    #[getter]
    fn stderr(&self) -> f64 {
        self.inner.stderr()
    }

    #[pyo3(text_signature = "($self, alpha)")]
    /// Confidence interval at the given significance level.
    ///
    /// Returns ``(mean, mean)`` when fewer than 2 samples are available.
    ///
    /// Args:
    ///     alpha (float): Significance level (e.g. 0.05 for 95% CI).
    ///
    /// Returns:
    ///     tuple[float, float]: ``(lower, upper)`` bounds.
    fn confidence_interval(&self, alpha: f64) -> (f64, f64) {
        self.inner.confidence_interval(alpha)
    }

    /// Half-width of the 95% confidence interval.
    ///
    /// Equivalent to ``(upper - lower) / 2`` at the 95% level.
    ///
    /// Returns:
    ///     float: Half-width of the confidence interval.
    #[getter]
    fn ci_half_width(&self) -> f64 {
        self.inner.ci_half_width()
    }

    /// Reset to empty state.
    fn reset(&mut self) {
        self.inner.reset();
    }

    fn __repr__(&self) -> String {
        format!(
            "OnlineStats(count={}, mean={:.6}, variance={:.6})",
            self.inner.count(),
            self.inner.mean(),
            self.inner.variance()
        )
    }
}

#[pyclass(name = "OnlineCovariance", module = "finstack.core.math.stats")]
/// Streaming covariance / correlation accumulator (Welford's algorithm).
///
/// Computes mean, variance, and covariance for two variables in a single
/// pass without storing every observation.
pub struct PyOnlineCovariance {
    inner: OnlineCovariance,
}

#[pymethods]
impl PyOnlineCovariance {
    #[new]
    /// Create an empty covariance accumulator.
    fn new() -> Self {
        Self {
            inner: OnlineCovariance::new(),
        }
    }

    /// Feed a single (x, y) observation pair.
    #[pyo3(text_signature = "($self, x, y)")]
    fn update(&mut self, x: f64, y: f64) {
        self.inner.update(x, y);
    }

    /// Merge another ``OnlineCovariance`` into this one (parallel reduce).
    #[pyo3(text_signature = "($self, other)")]
    fn merge(&mut self, other: &PyOnlineCovariance) {
        self.inner.merge(&other.inner);
    }

    /// Number of observation pairs seen so far.
    #[getter]
    fn count(&self) -> usize {
        self.inner.count()
    }

    /// Sample covariance (unbiased, n-1 denominator).
    #[getter]
    fn covariance(&self) -> f64 {
        self.inner.covariance()
    }

    /// Sample Pearson correlation.
    #[getter]
    fn correlation(&self) -> f64 {
        self.inner.correlation()
    }

    /// Current sample mean of x.
    #[getter]
    fn mean_x(&self) -> f64 {
        self.inner.mean_x()
    }

    /// Current sample mean of y.
    #[getter]
    fn mean_y(&self) -> f64 {
        self.inner.mean_y()
    }

    /// Sample variance of x (unbiased, n-1 denominator).
    #[getter]
    fn variance_x(&self) -> f64 {
        self.inner.variance_x()
    }

    /// Sample variance of y (unbiased, n-1 denominator).
    #[getter]
    fn variance_y(&self) -> f64 {
        self.inner.variance_y()
    }

    /// Optimal beta coefficient for control variate.
    ///
    /// Returns ``Cov(X, Y) / Var(Y)``, the coefficient that minimizes
    /// the variance of ``X - beta * (Y - E[Y])``.
    ///
    /// Returns:
    ///     float: Optimal beta coefficient. Returns 0.0 when Var(Y) is zero.
    #[getter]
    fn optimal_beta(&self) -> f64 {
        self.inner.optimal_beta()
    }

    /// Reset to empty state.
    fn reset(&mut self) {
        self.inner.reset();
    }

    fn __repr__(&self) -> String {
        format!(
            "OnlineCovariance(count={}, covariance={:.6}, correlation={:.6})",
            self.inner.count(),
            self.inner.covariance(),
            self.inner.correlation()
        )
    }
}

#[pyfunction(name = "moment_match")]
#[pyo3(text_signature = "(samples, target_mean, target_std)")]
/// Adjust samples in-place to have exact target mean and standard deviation.
///
/// This variance reduction technique forces the sample to match the
/// theoretical first two moments exactly.
///
/// Parameters
/// ----------
/// samples : list[float]
///     Sample values to adjust.
/// target_mean : float
///     Desired mean of the adjusted samples.
/// target_std : float
///     Desired standard deviation of the adjusted samples.
///
/// Returns
/// -------
/// list[float]
///     Adjusted samples with exact target moments.
pub fn moment_match_py(samples: Vec<f64>, target_mean: f64, target_std: f64) -> Vec<f64> {
    let mut data = samples;
    core_moment_match(&mut data, target_mean, target_std);
    data
}

#[pyfunction(name = "required_samples")]
#[pyo3(text_signature = "(cv, target_rel_error, alpha)")]
/// Minimum sample count for a target relative error at a given confidence level.
///
/// Parameters
/// ----------
/// cv : float
///     Coefficient of variation (sigma / mu).
/// target_rel_error : float
///     Target relative standard error (stderr / mean).
/// alpha : float
///     Significance level (e.g. 0.05 for 95% confidence).
///
/// Returns
/// -------
/// int
///     Minimum number of samples required.
pub fn required_samples_py(cv: f64, target_rel_error: f64, alpha: f64) -> usize {
    core_required_samples(cv, target_rel_error, alpha)
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "stats")?;
    module.setattr(
        "__doc__",
        "Statistical helpers (means, variances, covariances, realized variance).",
    )?;
    module.add_class::<PyRealizedVarMethod>()?;
    module.add_class::<PyOnlineStats>()?;
    module.add_class::<PyOnlineCovariance>()?;
    module.add_function(wrap_pyfunction!(mean_py, &module)?)?;
    module.add_function(wrap_pyfunction!(variance_py, &module)?)?;
    module.add_function(wrap_pyfunction!(population_variance_py, &module)?)?;
    module.add_function(wrap_pyfunction!(covariance_py, &module)?)?;
    module.add_function(wrap_pyfunction!(correlation_py, &module)?)?;
    module.add_function(wrap_pyfunction!(mean_var_py, &module)?)?;
    module.add_function(wrap_pyfunction!(quantile_py, &module)?)?;
    module.add_function(wrap_pyfunction!(log_returns_py, &module)?)?;
    module.add_function(wrap_pyfunction!(realized_variance_py, &module)?)?;
    module.add_function(wrap_pyfunction!(realized_variance_ohlc_py, &module)?)?;
    module.add_function(wrap_pyfunction!(moment_match_py, &module)?)?;
    module.add_function(wrap_pyfunction!(required_samples_py, &module)?)?;

    let exports = [
        "RealizedVarMethod",
        "OnlineStats",
        "OnlineCovariance",
        "mean",
        "variance",
        "population_variance",
        "covariance",
        "correlation",
        "mean_var",
        "quantile",
        "log_returns",
        "realized_variance",
        "realized_variance_ohlc",
        "moment_match",
        "required_samples",
    ];
    module.setattr("__all__", PyList::new(py, exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports.to_vec())
}
