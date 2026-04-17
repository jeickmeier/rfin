//! Python bindings for GARCH-family volatility models and residual diagnostics.
//!
//! Exposes a simple function-based API returning Python dicts/tuples so callers
//! do not need custom `#[pyclass]` wrappers. Mirrors the public API in
//! `finstack_analytics::timeseries`.

use crate::errors::core_to_py;
use finstack_analytics::timeseries as ts;
use finstack_analytics::timeseries::{
    Egarch11, Garch11, GarchFit, GarchModel, GjrGarch11, InnovationDist,
};
use pyo3::prelude::*;
use pyo3::types::PyDict;

// -------------------------------------------------------------------
// Helpers
// -------------------------------------------------------------------

/// Parse an innovation distribution string.
///
/// Accepts ``"gaussian"``/``"normal"`` and ``"student_t"``/``"t"`` (case
/// insensitive). For Student-t, an optional ``nu`` initial guess is used.
fn parse_dist(s: &str) -> PyResult<InnovationDist> {
    match s.to_ascii_lowercase().as_str() {
        "gaussian" | "normal" | "gauss" | "n" => Ok(InnovationDist::Gaussian),
        "student_t" | "student-t" | "studentt" | "t" => Ok(InnovationDist::StudentT(8.0)),
        other => Err(pyo3::exceptions::PyValueError::new_err(format!(
            "unknown distribution '{other}'; expected 'gaussian' or 'student_t'"
        ))),
    }
}

/// Populate a `PyDict` with the shared `GarchFit` fields.
fn fill_common_fit_fields<'py>(
    py: Python<'py>,
    out: &Bound<'py, PyDict>,
    fit: &GarchFit,
    param_names: &[&str],
) -> PyResult<()> {
    out.set_item("model", fit.model.clone())?;
    out.set_item("omega", fit.params.omega)?;
    out.set_item("alpha", fit.params.alpha)?;
    out.set_item("beta", fit.params.beta)?;
    if let Some(g) = fit.params.gamma {
        out.set_item("gamma", g)?;
    } else {
        out.set_item("gamma", py.None())?;
    }
    if let InnovationDist::StudentT(nu) = fit.params.dist {
        out.set_item("nu", nu)?;
    }
    out.set_item("persistence", fit.params.persistence())?;
    out.set_item(
        "unconditional_variance",
        fit.params.unconditional_variance(),
    )?;
    out.set_item("half_life", fit.params.half_life())?;
    out.set_item("log_likelihood", fit.log_likelihood)?;
    out.set_item("aic", fit.aic)?;
    out.set_item("bic", fit.bic)?;
    out.set_item("hqic", fit.hqic)?;
    out.set_item("n_obs", fit.n_obs)?;
    out.set_item("n_params", fit.n_params)?;
    out.set_item("converged", fit.converged)?;
    out.set_item("iterations", fit.iterations)?;
    out.set_item("terminal_variance", fit.terminal_variance)?;
    out.set_item("conditional_variances", fit.conditional_variances.clone())?;
    out.set_item("standardized_residuals", fit.standardized_residuals.clone())?;

    // Standard errors (paired with parameter names where possible).
    if let Some(se) = &fit.std_errors {
        let se_dict = PyDict::new(py);
        for (i, name) in param_names.iter().enumerate() {
            let v = se.get(i).copied().unwrap_or(f64::NAN);
            se_dict.set_item(*name, v)?;
        }
        out.set_item("std_errors", se_dict)?;
        out.set_item("std_errors_vec", se.clone())?;
    } else {
        out.set_item("std_errors", py.None())?;
        out.set_item("std_errors_vec", py.None())?;
    }

    // Convenience diagnostics on standardized residuals.
    out.set_item("ljung_box_squared_p10", fit.ljung_box_squared(10))?;
    out.set_item("arch_lm_p5", fit.arch_lm_test(5))?;

    Ok(())
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
/// Dict with keys ``omega``, ``alpha``, ``beta``, ``gamma`` (None for GARCH),
/// ``log_likelihood``, ``aic``, ``bic``, ``hqic``, ``converged``,
/// ``persistence``, ``unconditional_variance``, ``half_life``,
/// ``std_errors`` (dict keyed by parameter name), ``conditional_variances``,
/// ``standardized_residuals``, ``terminal_variance``, ``n_obs``,
/// ``n_params``, ``iterations``, plus quick residual diagnostics
/// ``ljung_box_squared_p10`` and ``arch_lm_p5``.
#[pyfunction]
#[pyo3(signature = (returns, distribution = "gaussian"))]
fn fit_garch11<'py>(
    py: Python<'py>,
    returns: Vec<f64>,
    distribution: &str,
) -> PyResult<Bound<'py, PyDict>> {
    let dist = parse_dist(distribution)?;
    let fit = Garch11.fit(&returns, dist, None).map_err(core_to_py)?;

    let out = PyDict::new(py);
    let mut names: Vec<&str> = Garch11.param_names();
    if matches!(fit.params.dist, InnovationDist::StudentT(_)) {
        names.push("nu");
    }
    fill_common_fit_fields(py, &out, &fit, &names)?;
    Ok(out)
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
/// Dict with keys ``omega``, ``alpha``, ``gamma``, ``beta``,
/// ``log_likelihood``, ``aic``, ``bic``, ``hqic``, ``converged``, and the
/// same residual/diagnostic fields as :func:`fit_garch11`.
#[pyfunction]
#[pyo3(signature = (returns, distribution = "gaussian"))]
fn fit_egarch11<'py>(
    py: Python<'py>,
    returns: Vec<f64>,
    distribution: &str,
) -> PyResult<Bound<'py, PyDict>> {
    let dist = parse_dist(distribution)?;
    let fit = Egarch11.fit(&returns, dist, None).map_err(core_to_py)?;

    let out = PyDict::new(py);
    let mut names: Vec<&str> = Egarch11.param_names();
    if matches!(fit.params.dist, InnovationDist::StudentT(_)) {
        names.push("nu");
    }
    fill_common_fit_fields(py, &out, &fit, &names)?;
    Ok(out)
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
/// Dict with keys ``omega``, ``alpha``, ``gamma``, ``beta``,
/// ``log_likelihood``, ``aic``, ``bic``, ``hqic``, ``converged``, and the
/// same residual/diagnostic fields as :func:`fit_garch11`.
#[pyfunction]
#[pyo3(signature = (returns, distribution = "gaussian"))]
fn fit_gjr_garch11<'py>(
    py: Python<'py>,
    returns: Vec<f64>,
    distribution: &str,
) -> PyResult<Bound<'py, PyDict>> {
    let dist = parse_dist(distribution)?;
    let fit = GjrGarch11.fit(&returns, dist, None).map_err(core_to_py)?;

    let out = PyDict::new(py);
    let mut names: Vec<&str> = GjrGarch11.param_names();
    if matches!(fit.params.dist, InnovationDist::StudentT(_)) {
        names.push("nu");
    }
    fill_common_fit_fields(py, &out, &fit, &names)?;
    Ok(out)
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
    if horizon == 0 {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(horizon);
    let mut s2 = omega + alpha * last_return * last_return + beta * last_variance;
    out.push(s2.max(0.0));
    let persistence = alpha + beta;
    for _ in 1..horizon {
        s2 = omega + persistence * s2;
        out.push(s2.max(0.0));
    }
    out
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
