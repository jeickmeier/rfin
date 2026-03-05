//! Python bindings for cashflow utility functions: CPR/SMM conversions and rate helpers.

use finstack_valuations::cashflow::builder::{
    compute_compounded_rate, compute_overnight_rate, compute_simple_average_rate, cpr_to_smm,
    smm_to_cpr,
};
use pyo3::prelude::*;
use pyo3::types::PyModule;

use super::specs::PyOvernightCompoundingMethod;

/// Convert an annual Conditional Prepayment Rate (CPR) to a Single Monthly Mortality (SMM).
///
/// Raises `ValueError` if CPR is negative.
#[pyfunction]
#[pyo3(name = "cpr_to_smm", text_signature = "(cpr)")]
fn py_cpr_to_smm(cpr: f64) -> PyResult<f64> {
    cpr_to_smm(cpr).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
}

/// Convert a Single Monthly Mortality (SMM) to an annual Conditional Prepayment Rate (CPR).
#[pyfunction]
#[pyo3(name = "smm_to_cpr", text_signature = "(smm)")]
fn py_smm_to_cpr(smm: f64) -> f64 {
    smm_to_cpr(smm)
}

/// Compute a compounded rate from daily rate observations.
///
/// Args:
///     daily_rates: List of (rate, days) tuples where rate is the daily fixing and days is the
///                  number of calendar days that fixing applies.
///     total_days: Total number of days in the accrual period.
///     day_count_basis: Day count basis denominator (e.g. 360.0 for Act/360).
///
/// Returns:
///     Annualized compounded rate.
#[pyfunction]
#[pyo3(
    name = "compute_compounded_rate",
    text_signature = "(daily_rates, total_days, day_count_basis)"
)]
fn py_compute_compounded_rate(
    daily_rates: Vec<(f64, u32)>,
    total_days: u32,
    day_count_basis: f64,
) -> f64 {
    compute_compounded_rate(&daily_rates, total_days, day_count_basis)
}

/// Compute a simple average rate from daily rate observations.
///
/// Args:
///     daily_rates: List of (rate, days) tuples.
///     total_days: Total number of days in the accrual period.
///
/// Returns:
///     Simple weighted average rate.
#[pyfunction]
#[pyo3(
    name = "compute_simple_average_rate",
    text_signature = "(daily_rates, total_days)"
)]
fn py_compute_simple_average_rate(daily_rates: Vec<(f64, u32)>, total_days: u32) -> f64 {
    compute_simple_average_rate(&daily_rates, total_days)
}

/// Compute an overnight compounding rate using the specified method.
///
/// Args:
///     method: OvernightCompoundingMethod (e.g. COMPOUNDED_IN_ARREARS, SIMPLE_AVERAGE).
///     daily_rates: List of (rate, days) tuples.
///     total_days: Total number of days in the accrual period.
///     day_count_basis: Day count basis denominator (e.g. 360.0).
///
/// Returns:
///     Rate computed using the specified overnight compounding method.
#[pyfunction]
#[pyo3(
    name = "compute_overnight_rate",
    text_signature = "(method, daily_rates, total_days, day_count_basis)"
)]
fn py_compute_overnight_rate(
    method: PyOvernightCompoundingMethod,
    daily_rates: Vec<(f64, u32)>,
    total_days: u32,
    day_count_basis: f64,
) -> f64 {
    compute_overnight_rate(method.inner, &daily_rates, total_days, day_count_basis)
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_function(wrap_pyfunction!(py_cpr_to_smm, module)?)?;
    module.add_function(wrap_pyfunction!(py_smm_to_cpr, module)?)?;
    module.add_function(wrap_pyfunction!(py_compute_compounded_rate, module)?)?;
    module.add_function(wrap_pyfunction!(py_compute_simple_average_rate, module)?)?;
    module.add_function(wrap_pyfunction!(py_compute_overnight_rate, module)?)?;
    Ok(vec![
        "compute_compounded_rate",
        "compute_overnight_rate",
        "compute_simple_average_rate",
        "cpr_to_smm",
        "smm_to_cpr",
    ])
}
