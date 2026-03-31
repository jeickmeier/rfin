//! Python bindings for consecutive streak counting.
//!
//! Since Python cannot pass closures, we expose convenience functions for
//! common predicates (positive / negative / above threshold / below threshold).

use finstack_analytics::consecutive;
use pyo3::prelude::*;

/// Longest consecutive streak of positive values.
///
/// Parameters
/// ----------
/// values : list[float]
///     Numeric series to scan.
///
/// Returns
/// -------
/// int
///     Length of the longest run of ``v > 0``.
#[pyfunction]
fn count_consecutive_positive(values: Vec<f64>) -> usize {
    consecutive::count_consecutive(&values, |v| v > 0.0)
}

/// Longest consecutive streak of negative values.
///
/// Parameters
/// ----------
/// values : list[float]
///     Numeric series to scan.
///
/// Returns
/// -------
/// int
///     Length of the longest run of ``v < 0``.
#[pyfunction]
fn count_consecutive_negative(values: Vec<f64>) -> usize {
    consecutive::count_consecutive(&values, |v| v < 0.0)
}

/// Longest consecutive streak of values above a threshold.
///
/// Parameters
/// ----------
/// values : list[float]
///     Numeric series to scan.
/// threshold : float
///     Comparison threshold.
///
/// Returns
/// -------
/// int
///     Length of the longest run of ``v > threshold``.
#[pyfunction]
fn count_consecutive_above(values: Vec<f64>, threshold: f64) -> usize {
    consecutive::count_consecutive(&values, |v| v > threshold)
}

/// Longest consecutive streak of values below a threshold.
///
/// Parameters
/// ----------
/// values : list[float]
///     Numeric series to scan.
/// threshold : float
///     Comparison threshold.
///
/// Returns
/// -------
/// int
///     Length of the longest run of ``v < threshold``.
#[pyfunction]
fn count_consecutive_below(values: Vec<f64>, threshold: f64) -> usize {
    consecutive::count_consecutive(&values, |v| v < threshold)
}

/// Register standalone consecutive functions and return export names.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<Vec<&'static str>> {
    m.add_function(wrap_pyfunction!(count_consecutive_positive, m)?)?;
    m.add_function(wrap_pyfunction!(count_consecutive_negative, m)?)?;
    m.add_function(wrap_pyfunction!(count_consecutive_above, m)?)?;
    m.add_function(wrap_pyfunction!(count_consecutive_below, m)?)?;
    Ok(vec![
        "count_consecutive_positive",
        "count_consecutive_negative",
        "count_consecutive_above",
        "count_consecutive_below",
    ])
}
