//! Python bindings for `finstack_core::math::special_functions`.

use finstack_core::math::special_functions;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};

// ---------------------------------------------------------------------------
// Module-level functions
// ---------------------------------------------------------------------------

/// Standard normal cumulative distribution function Φ(x).
///
/// Returns the probability P(Z ≤ x) where Z ~ N(0, 1).
#[pyfunction]
#[pyo3(text_signature = "(x)")]
fn norm_cdf(x: f64) -> f64 {
    special_functions::norm_cdf(x)
}

/// Standard normal probability density function φ(x).
///
/// Returns (1/√(2π)) · exp(-x²/2).
#[pyfunction]
#[pyo3(text_signature = "(x)")]
fn norm_pdf(x: f64) -> f64 {
    special_functions::norm_pdf(x)
}

/// Inverse standard normal CDF (Φ⁻¹).
///
/// Returns x such that Φ(x) = p.
#[pyfunction]
#[pyo3(text_signature = "(p)")]
fn standard_normal_inv_cdf(p: f64) -> f64 {
    special_functions::standard_normal_inv_cdf(p)
}

/// Error function erf(x) = (2/√π) ∫₀ˣ e^(-t²) dt.
#[pyfunction]
#[pyo3(text_signature = "(x)")]
fn erf(x: f64) -> f64 {
    special_functions::erf(x)
}

/// Natural logarithm of the Gamma function ln(Γ(x)).
///
/// Returns `f64::INFINITY` for x ≤ 0.
#[pyfunction]
#[pyo3(text_signature = "(x)")]
fn ln_gamma(x: f64) -> f64 {
    special_functions::ln_gamma(x)
}

// ---------------------------------------------------------------------------
// Register
// ---------------------------------------------------------------------------

/// Build the `finstack.core.math.special_functions` submodule.
pub fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(py, "special_functions")?;
    m.setattr(
        "__doc__",
        "Special mathematical functions: normal distribution, error function, gamma.",
    )?;

    m.add_function(wrap_pyfunction!(norm_cdf, &m)?)?;
    m.add_function(wrap_pyfunction!(norm_pdf, &m)?)?;
    m.add_function(wrap_pyfunction!(standard_normal_inv_cdf, &m)?)?;
    m.add_function(wrap_pyfunction!(erf, &m)?)?;
    m.add_function(wrap_pyfunction!(ln_gamma, &m)?)?;

    let all = PyList::new(
        py,
        [
            "norm_cdf",
            "norm_pdf",
            "standard_normal_inv_cdf",
            "erf",
            "ln_gamma",
        ],
    )?;
    m.setattr("__all__", all)?;
    parent.add_submodule(&m)?;

    let pkg: String = match parent.getattr("__package__") {
        Ok(attr) => match attr.extract::<String>() {
            Ok(s) => s,
            Err(_) => "finstack.core.math".to_string(),
        },
        Err(_) => "finstack.core.math".to_string(),
    };
    let qual = format!("{pkg}.special_functions");
    m.setattr("__package__", &qual)?;
    let sys = PyModule::import(py, "sys")?;
    let modules = sys.getattr("modules")?;
    modules.set_item(&qual, &m)?;

    Ok(())
}
