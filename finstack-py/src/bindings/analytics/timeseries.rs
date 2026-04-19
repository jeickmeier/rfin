//! Python bindings for GARCH-family volatility models and residual diagnostics.
//!
//! Exposes typed `GarchFit` / `GarchParams` result classes and a
//! function-based fitting API.  Mirrors the public API in
//! `finstack_analytics::timeseries`.

use std::str::FromStr;

use crate::errors::core_to_py;
use finstack_analytics::timeseries as ts;
use finstack_analytics::timeseries::{Egarch11, Garch11, GarchModel, GjrGarch11, InnovationDist};
use pyo3::prelude::*;

// -------------------------------------------------------------------
// PyGarchParams
// -------------------------------------------------------------------

/// Estimated GARCH model parameters.
///
/// Mirrors ``finstack_analytics::timeseries::GarchParams``.
/// Fields use the same names as the Rust struct for serde parity.
#[pyclass(frozen, name = "GarchParams", module = "finstack.analytics")]
pub struct PyGarchParams {
    pub(crate) inner: ts::GarchParams,
}

#[pymethods]
impl PyGarchParams {
    /// Intercept (omega).
    #[getter]
    fn omega(&self) -> f64 {
        self.inner.omega
    }
    /// ARCH coefficient (alpha).
    #[getter]
    fn alpha(&self) -> f64 {
        self.inner.alpha
    }
    /// GARCH coefficient (beta).
    #[getter]
    fn beta(&self) -> f64 {
        self.inner.beta
    }
    /// Leverage / asymmetry parameter (``None`` for symmetric GARCH).
    #[getter]
    fn gamma(&self) -> Option<f64> {
        self.inner.gamma
    }
    /// Innovation distribution name (``"gaussian"`` or ``"student_t"``).
    #[getter]
    fn distribution(&self) -> &str {
        match self.inner.dist {
            InnovationDist::Gaussian => "gaussian",
            InnovationDist::StudentT(_) => "student_t",
        }
    }
    /// Student-t degrees of freedom (``None`` for Gaussian).
    #[getter]
    fn nu(&self) -> Option<f64> {
        match self.inner.dist {
            InnovationDist::StudentT(nu) => Some(nu),
            InnovationDist::Gaussian => None,
        }
    }
    /// Constant mean used in demeaning.
    #[getter]
    fn mean(&self) -> f64 {
        self.inner.mean
    }
    /// Persistence of volatility shocks.
    #[getter]
    fn persistence(&self) -> f64 {
        self.inner.persistence()
    }
    /// Unconditional variance (``None`` for EGARCH or non-stationary).
    #[getter]
    fn unconditional_variance(&self) -> Option<f64> {
        self.inner.unconditional_variance()
    }
    /// Shock half-life in periods (``None`` when undefined).
    #[getter]
    fn half_life(&self) -> Option<f64> {
        self.inner.half_life()
    }

    fn __repr__(&self) -> String {
        format!(
            "GarchParams(omega={:.6}, alpha={:.6}, beta={:.6}, persistence={:.6})",
            self.inner.omega,
            self.inner.alpha,
            self.inner.beta,
            self.inner.persistence(),
        )
    }
}

// -------------------------------------------------------------------
// PyGarchFit
// -------------------------------------------------------------------

/// Complete result of a GARCH model fit.
///
/// Mirrors ``finstack_analytics::timeseries::GarchFit``.
#[pyclass(frozen, name = "GarchFit", module = "finstack.analytics")]
pub struct PyGarchFit {
    pub(crate) inner: ts::GarchFit,
}

impl PyGarchFit {
    /// Build from the Rust fit result.
    pub(crate) fn from_inner(inner: ts::GarchFit) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyGarchFit {
    /// Model name (e.g. ``"GARCH(1,1)"``).
    #[getter]
    fn model(&self) -> &str {
        &self.inner.model
    }
    /// Estimated parameters.
    #[getter]
    fn params(&self) -> PyGarchParams {
        PyGarchParams {
            inner: self.inner.params.clone(),
        }
    }
    /// Intercept (omega) — shortcut for ``fit.params.omega``.
    #[getter]
    fn omega(&self) -> f64 {
        self.inner.params.omega
    }
    /// ARCH coefficient (alpha) — shortcut for ``fit.params.alpha``.
    #[getter]
    fn alpha(&self) -> f64 {
        self.inner.params.alpha
    }
    /// GARCH coefficient (beta) — shortcut for ``fit.params.beta``.
    #[getter]
    fn beta(&self) -> f64 {
        self.inner.params.beta
    }
    /// Leverage parameter — shortcut for ``fit.params.gamma``.
    #[getter]
    fn gamma(&self) -> Option<f64> {
        self.inner.params.gamma
    }
    /// Student-t dof — shortcut for ``fit.params.nu``.
    #[getter]
    fn nu(&self) -> Option<f64> {
        match self.inner.params.dist {
            InnovationDist::StudentT(nu) => Some(nu),
            InnovationDist::Gaussian => None,
        }
    }
    /// Persistence — shortcut for ``fit.params.persistence``.
    #[getter]
    fn persistence(&self) -> f64 {
        self.inner.params.persistence()
    }
    /// Unconditional variance — shortcut for ``fit.params.unconditional_variance``.
    #[getter]
    fn unconditional_variance(&self) -> Option<f64> {
        self.inner.params.unconditional_variance()
    }
    /// Shock half-life — shortcut for ``fit.params.half_life``.
    #[getter]
    fn half_life(&self) -> Option<f64> {
        self.inner.params.half_life()
    }
    /// Approximate standard errors (``None`` if Hessian inversion failed).
    #[getter]
    fn std_errors(&self) -> Option<Vec<f64>> {
        self.inner.std_errors.clone()
    }
    /// Maximized log-likelihood.
    #[getter]
    fn log_likelihood(&self) -> f64 {
        self.inner.log_likelihood
    }
    /// Number of observations used in fitting.
    #[getter]
    fn n_obs(&self) -> usize {
        self.inner.n_obs
    }
    /// Number of estimated parameters.
    #[getter]
    fn n_params(&self) -> usize {
        self.inner.n_params
    }
    /// Akaike Information Criterion.
    #[getter]
    fn aic(&self) -> f64 {
        self.inner.aic
    }
    /// Bayesian Information Criterion.
    #[getter]
    fn bic(&self) -> f64 {
        self.inner.bic
    }
    /// Hannan-Quinn Information Criterion.
    #[getter]
    fn hqic(&self) -> f64 {
        self.inner.hqic
    }
    /// Conditional variance series (length = ``n_obs``).
    #[getter]
    fn conditional_variances(&self) -> Vec<f64> {
        self.inner.conditional_variances.clone()
    }
    /// Standardized residuals: ``z_t = (r_t - mu) / sigma_t``.
    #[getter]
    fn standardized_residuals(&self) -> Vec<f64> {
        self.inner.standardized_residuals.clone()
    }
    /// Terminal conditional variance (last ``sigma^2_t``).
    #[getter]
    fn terminal_variance(&self) -> f64 {
        self.inner.terminal_variance
    }
    /// Whether the optimizer converged.
    #[getter]
    fn converged(&self) -> bool {
        self.inner.converged
    }
    /// Number of optimizer iterations.
    #[getter]
    fn iterations(&self) -> usize {
        self.inner.iterations
    }
    /// Ljung-Box p-value on squared standardized residuals (lag=10).
    #[getter]
    fn ljung_box_squared_p10(&self) -> f64 {
        self.inner.ljung_box_squared(10)
    }
    /// ARCH-LM p-value on standardized residuals (lag=5).
    #[getter]
    fn arch_lm_p5(&self) -> f64 {
        self.inner.arch_lm_test(5)
    }

    fn __repr__(&self) -> String {
        format!(
            "GarchFit(model='{}', ll={:.4}, aic={:.4}, converged={})",
            self.inner.model, self.inner.log_likelihood, self.inner.aic, self.inner.converged,
        )
    }
}

// -------------------------------------------------------------------
// Helpers
// -------------------------------------------------------------------

/// Parse an innovation distribution string.
///
/// Delegates to [`InnovationDist::from_str`], which accepts
/// ``"gaussian"``/``"normal"`` and ``"student_t"``/``"t"`` (case insensitive).
fn parse_dist(s: &str) -> PyResult<InnovationDist> {
    InnovationDist::from_str(s).map_err(pyo3::exceptions::PyValueError::new_err)
}

// -------------------------------------------------------------------
// fit_garch11
// -------------------------------------------------------------------

/// Fit a standard GARCH(1,1) model by maximum likelihood.
///
/// # Arguments
///
/// * ``returns`` - Log return series (at least 10 observations).
/// * ``distribution`` - Innovation distribution: ``"gaussian"`` (default)
///   or ``"student_t"``.
///
/// # Returns
///
/// :class:`GarchFit` with estimated parameters, diagnostics, and
/// conditional variance series.
#[pyfunction]
#[pyo3(signature = (returns, distribution = "gaussian"))]
fn fit_garch11(returns: Vec<f64>, distribution: &str) -> PyResult<PyGarchFit> {
    let dist = parse_dist(distribution)?;
    let fit = Garch11.fit(&returns, dist, None).map_err(core_to_py)?;
    Ok(PyGarchFit::from_inner(fit))
}

// -------------------------------------------------------------------
// fit_egarch11
// -------------------------------------------------------------------

/// Fit an EGARCH(1,1) model (Nelson, 1991) with leverage via log-variance.
///
/// # Arguments
///
/// * ``returns`` - Log return series.
/// * ``distribution`` - ``"gaussian"`` (default) or ``"student_t"``.
///
/// # Returns
///
/// :class:`GarchFit` result.
#[pyfunction]
#[pyo3(signature = (returns, distribution = "gaussian"))]
fn fit_egarch11(returns: Vec<f64>, distribution: &str) -> PyResult<PyGarchFit> {
    let dist = parse_dist(distribution)?;
    let fit = Egarch11.fit(&returns, dist, None).map_err(core_to_py)?;
    Ok(PyGarchFit::from_inner(fit))
}

// -------------------------------------------------------------------
// fit_gjr_garch11
// -------------------------------------------------------------------

/// Fit a GJR-GARCH(1,1) model (Glosten, Jagannathan & Runkle, 1993).
///
/// Asymmetric threshold term: variance increases more after negative shocks
/// via the non-negative leverage coefficient ``gamma``.
///
/// # Arguments
///
/// * ``returns`` - Log return series.
/// * ``distribution`` - ``"gaussian"`` (default) or ``"student_t"``.
///
/// # Returns
///
/// :class:`GarchFit` result.
#[pyfunction]
#[pyo3(signature = (returns, distribution = "gaussian"))]
fn fit_gjr_garch11(returns: Vec<f64>, distribution: &str) -> PyResult<PyGarchFit> {
    let dist = parse_dist(distribution)?;
    let fit = GjrGarch11.fit(&returns, dist, None).map_err(core_to_py)?;
    Ok(PyGarchFit::from_inner(fit))
}

// -------------------------------------------------------------------
// garch11_forecast
// -------------------------------------------------------------------

/// Closed-form h-step-ahead GARCH(1,1) variance forecast.
///
/// Iterates the recurrence
///
/// ```text
/// sigma^2_{t+h} = omega + (alpha + beta) * sigma^2_{t+h-1}      (h >= 2)
/// sigma^2_{t+1} = omega + alpha * r_t^2 + beta * sigma^2_t      (h == 1)
/// ```
///
/// and returns the variance path for horizons ``1..=horizon``.
///
/// # Arguments
///
/// * ``omega``, ``alpha``, ``beta`` - Fitted GARCH(1,1) parameters.
/// * ``last_variance`` - Terminal conditional variance ``sigma^2_t``.
/// * ``last_return`` - Terminal return ``r_t`` (enters only the h=1 step).
/// * ``horizon`` - Number of horizons to forecast (``>= 1``).
///
/// # Returns
///
/// List of ``horizon`` forecasted variances in order ``h=1..horizon``.
#[pyfunction]
fn garch11_forecast(
    omega: f64,
    alpha: f64,
    beta: f64,
    last_variance: f64,
    last_return: f64,
    horizon: usize,
) -> Vec<f64> {
    ts::garch11_forecast(omega, alpha, beta, last_variance, last_return, horizon)
}

// -------------------------------------------------------------------
// ljung_box
// -------------------------------------------------------------------

/// Ljung-Box Q-statistic for serial correlation up to ``lags`` lags.
///
/// # Arguments
///
/// * ``residuals`` - Series to test (commonly standardized or squared
///   residuals).
/// * ``lags`` - Number of autocorrelation lags to include in Q.
///
/// # Returns
///
/// Tuple ``(q_stat, p_value)``. Low p-value rejects the null of no serial
/// correlation up to ``lags``.
#[pyfunction]
fn ljung_box(residuals: Vec<f64>, lags: usize) -> (f64, f64) {
    ts::ljung_box(&residuals, lags)
}

// -------------------------------------------------------------------
// arch_lm
// -------------------------------------------------------------------

/// Engle's ARCH-LM test for remaining heteroskedasticity.
///
/// Regresses ``z_t^2`` on a constant plus ``lags`` of its own past. The
/// statistic is ``T * R^2 ~ chi^2(lags)`` under the null of no ARCH
/// effects.
///
/// # Arguments
///
/// * ``residuals`` - Standardized residuals from a mean/volatility model.
/// * ``lags`` - Number of squared-residual lags to include.
///
/// # Returns
///
/// Tuple ``(lm_stat, p_value)``. Low p-value indicates remaining ARCH.
#[pyfunction]
fn arch_lm(residuals: Vec<f64>, lags: usize) -> (f64, f64) {
    ts::arch_lm(&residuals, lags)
}

// -------------------------------------------------------------------
// Information-criterion convenience wrappers
// -------------------------------------------------------------------

/// Akaike Information Criterion: ``-2 * log_likelihood + 2 * n_params``.
#[pyfunction]
fn aic(log_likelihood: f64, n_params: usize) -> f64 {
    ts::aic(log_likelihood, n_params)
}

/// Bayesian Information Criterion: ``-2 * log_likelihood + n_params * ln(n_obs)``.
#[pyfunction]
fn bic(log_likelihood: f64, n_params: usize, n_obs: usize) -> f64 {
    ts::bic(log_likelihood, n_params, n_obs)
}

/// Hannan-Quinn Information Criterion: ``-2*LL + 2*k*ln(ln(n_obs))``.
#[pyfunction]
fn hqic(log_likelihood: f64, n_params: usize, n_obs: usize) -> f64 {
    ts::hqic(log_likelihood, n_params, n_obs)
}

// -------------------------------------------------------------------
// Registration
// -------------------------------------------------------------------

pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyGarchParams>()?;
    m.add_class::<PyGarchFit>()?;
    m.add_function(wrap_pyfunction!(fit_garch11, m)?)?;
    m.add_function(wrap_pyfunction!(fit_egarch11, m)?)?;
    m.add_function(wrap_pyfunction!(fit_gjr_garch11, m)?)?;
    m.add_function(wrap_pyfunction!(garch11_forecast, m)?)?;
    m.add_function(wrap_pyfunction!(ljung_box, m)?)?;
    m.add_function(wrap_pyfunction!(arch_lm, m)?)?;
    m.add_function(wrap_pyfunction!(aic, m)?)?;
    m.add_function(wrap_pyfunction!(bic, m)?)?;
    m.add_function(wrap_pyfunction!(hqic, m)?)?;
    Ok(())
}
