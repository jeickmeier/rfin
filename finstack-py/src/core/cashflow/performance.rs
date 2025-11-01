//! Python bindings for performance measurement utilities: IRR, XIRR, and NPV.

use crate::core::error::core_to_py;
use crate::core::utils::py_to_date;
use finstack_core::cashflow::{irr_periodic, npv_performance};
use pyo3::prelude::*;
use pyo3::types::PyModule;
use pyo3::Bound;

/// Calculate NPV (Net Present Value) for a series of cash flows at a given discount rate.
///
/// Args:
///     cash_flows: List of (date, amount) tuples. Dates can be date objects or ISO strings.
///     discount_rate: Annual discount rate as a decimal (e.g., 0.1 for 10%).
///     base_date: Optional base date for discounting (defaults to first cash flow date).
///     day_count: Optional day count convention string (defaults to 'act365f').
///
/// Returns:
///     float: The net present value
///
/// Raises:
///     ValueError: If cash flows are empty or date calculations fail.
///
/// Examples:
///     >>> from finstack.core import dates
///     >>> from datetime import date
///     >>> flows = [
///     ...     (date(2024, 1, 1), -100000),
///     ...     (date(2025, 1, 1), 110000)
///     ... ]
///     >>> npv(flows, 0.05)
///     4761.90...
#[pyfunction(name = "npv")]
#[pyo3(
    signature = (cash_flows, discount_rate, base_date=None, day_count=None),
    text_signature = "(cash_flows, discount_rate, base_date=None, day_count=None)"
)]
pub fn py_npv(
    cash_flows: Vec<(Bound<'_, PyAny>, f64)>,
    discount_rate: f64,
    base_date: Option<Bound<'_, PyAny>>,
    day_count: Option<&str>,
) -> PyResult<f64> {
    // Convert Python dates to Rust dates
    let mut flows: Vec<(finstack_core::dates::Date, f64)> = Vec::with_capacity(cash_flows.len());

    for (date, amount) in cash_flows {
        let rust_date = py_to_date(&date)?;
        flows.push((rust_date, amount));
    }

    let base = base_date
        .map(|d| py_to_date(&d))
        .transpose()?
        .or_else(|| flows.first().map(|(d, _)| *d));

    // Parse day count from string if provided, otherwise use Act365F
    let dc = if let Some(dc_str) = day_count {
        // Try to parse common day count names
        match dc_str.to_lowercase().as_str() {
            "act365f" | "365f" => finstack_core::dates::DayCount::Act365F,
            "act360" | "360" => finstack_core::dates::DayCount::Act360,
            "thirty360" | "30/360" | "30_360" => finstack_core::dates::DayCount::Thirty360,
            _ => finstack_core::dates::DayCount::Act365F, // Default
        }
    } else {
        finstack_core::dates::DayCount::Act365F
    };

    npv_performance(&flows, discount_rate, base, Some(dc)).map_err(core_to_py)
}

/// Calculate IRR (Internal Rate of Return) for evenly-spaced periodic cash flows.
///
/// This is a simplified version of XIRR for cash flows that occur at regular intervals
/// (e.g., monthly, quarterly, or annual).
///
/// Args:
///     amounts: List of cash flow amounts (negative for outflows, positive for inflows).
///     guess: Optional initial guess for the IRR (defaults to 0.1 = 10%).
///
/// Returns:
///     float: The IRR as a decimal per period
///
/// Raises:
///     ValueError: If less than 2 cash flows, no sign change, or cannot converge.
///
/// Examples:
///     >>> # Quarterly cash flows over 2 years
///     >>> amounts = [-100000, 3000, 3000, 3000, 3000, 3000, 3000, 3000, 90000]
///     >>> quarterly_irr = irr_periodic(amounts, None)
///     >>> # Annual IRR = (1 + quarterly_irr)^4 - 1
///     >>> annual_irr = (1 + quarterly_irr) ** 4 - 1
#[pyfunction(name = "irr_periodic")]
#[pyo3(
    signature = (amounts, guess=None),
    text_signature = "(amounts, guess=None)"
)]
pub fn py_irr_periodic(amounts: Vec<f64>, guess: Option<f64>) -> PyResult<f64> {
    irr_periodic(&amounts, guess).map_err(core_to_py)
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_function(wrap_pyfunction!(py_npv, module)?)?;
    module.add_function(wrap_pyfunction!(py_irr_periodic, module)?)?;
    Ok(vec!["npv", "irr_periodic"])
}
