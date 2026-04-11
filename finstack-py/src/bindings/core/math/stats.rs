//! Python bindings for `finstack_core::math::stats`.

use finstack_core::math::stats;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};

// ---------------------------------------------------------------------------
// Module-level functions
// ---------------------------------------------------------------------------

/// Arithmetic mean of a data series.
///
/// Returns ``0.0`` for an empty list.
#[pyfunction]
#[pyo3(text_signature = "(data)")]
fn mean(data: Vec<f64>) -> f64 {
    stats::mean(&data)
}

/// Sample variance (unbiased, n-1 denominator).
///
/// Returns ``0.0`` for fewer than 2 observations.
#[pyfunction]
#[pyo3(text_signature = "(data)")]
fn variance(data: Vec<f64>) -> f64 {
    stats::variance(&data)
}

/// Population variance (n denominator).
///
/// Returns ``0.0`` for an empty list.
#[pyfunction]
#[pyo3(text_signature = "(data)")]
fn population_variance(data: Vec<f64>) -> f64 {
    stats::population_variance(&data)
}

/// Pearson correlation coefficient between two equal-length series.
///
/// Returns ``NaN`` if the input lengths differ.
#[pyfunction]
#[pyo3(text_signature = "(x, y)")]
fn correlation(x: Vec<f64>, y: Vec<f64>) -> f64 {
    stats::correlation(&x, &y)
}

/// Sample covariance (unbiased, n-1 denominator).
///
/// Returns ``NaN`` if the input lengths differ.
#[pyfunction]
#[pyo3(text_signature = "(x, y)")]
fn covariance(x: Vec<f64>, y: Vec<f64>) -> f64 {
    stats::covariance(&x, &y)
}

/// Empirical quantile (R-7 / NumPy default) with linear interpolation.
///
/// Returns ``NaN`` for empty data, `q` outside ``[0, 1]``, or non-finite inputs.
#[pyfunction]
#[pyo3(text_signature = "(data, q)")]
fn quantile(mut data: Vec<f64>, q: f64) -> f64 {
    stats::quantile(&mut data, q)
}

// ---------------------------------------------------------------------------
// Register
// ---------------------------------------------------------------------------

/// Build the `finstack.core.math.stats` submodule.
pub fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(py, "stats")?;
    m.setattr(
        "__doc__",
        "Statistical functions: mean, variance, correlation, covariance, quantile.",
    )?;

    m.add_function(wrap_pyfunction!(mean, &m)?)?;
    m.add_function(wrap_pyfunction!(variance, &m)?)?;
    m.add_function(wrap_pyfunction!(population_variance, &m)?)?;
    m.add_function(wrap_pyfunction!(correlation, &m)?)?;
    m.add_function(wrap_pyfunction!(covariance, &m)?)?;
    m.add_function(wrap_pyfunction!(quantile, &m)?)?;

    let all = PyList::new(
        py,
        [
            "mean",
            "variance",
            "population_variance",
            "correlation",
            "covariance",
            "quantile",
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
    let qual = format!("{pkg}.stats");
    m.setattr("__package__", &qual)?;
    let sys = PyModule::import(py, "sys")?;
    let modules = sys.getattr("modules")?;
    modules.set_item(&qual, &m)?;

    Ok(())
}
