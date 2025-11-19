use crate::errors::core_to_py;
use crate::core::utils::py_to_date;
use finstack_core::cashflow::xirr as core_xirr;
use pyo3::prelude::*;
use pyo3::types::PyModule;
use pyo3::Bound;

/// Calculate XIRR (Extended Internal Rate of Return) for irregular cash flows.
///
/// XIRR finds the discount rate that makes the net present value of all cash flows
/// equal to zero. It's particularly useful for investments with irregular timing.
///
/// Parameters
/// ----------
/// cash_flows : list[tuple[datetime.date, float]]
///     List of (date, amount) pairs. Negative amounts represent outflows (investments),
///     positive amounts represent inflows (returns).
/// guess : float, optional
///     Initial guess for the IRR (default: 0.1 = 10%). Providing a good guess can
///     help convergence for difficult cases.
///
/// Returns
/// -------
/// float
///     The XIRR as a decimal (e.g., 0.15 for 15% annual return).
///
/// Raises
/// ------
/// ValueError
///     If less than 2 cash flows provided, or no sign change in cash flows.
/// RuntimeError
///     If the solver cannot converge to a solution.
///
/// Examples
/// --------
/// >>> from datetime import date
/// >>> from finstack.core.cashflow import xirr
/// >>>
/// >>> # Investment with irregular cash flows
/// >>> cash_flows = [
/// ...     (date(2024, 1, 1), -100000.0),   # Initial investment
/// ...     (date(2024, 6, 15), 5000.0),     # Mid-year dividend
/// ...     (date(2025, 1, 1), 110000.0)     # Final value
/// ... ]
/// >>> irr = xirr(cash_flows)
/// >>> print(f"IRR: {irr * 100:.2f}%")
/// IRR: 15.23%
///
/// See Also
/// --------
/// finstack.core.math.solver : Root-finding algorithms used by XIRR
#[pyfunction(name = "xirr")]
#[pyo3(signature = (cash_flows, guess=None), text_signature = "(cash_flows, guess=None)")]
pub fn py_xirr(cash_flows: Vec<(Bound<'_, PyAny>, f64)>, guess: Option<f64>) -> PyResult<f64> {
    // Convert Python dates to Rust dates
    let mut flows: Vec<(finstack_core::dates::Date, f64)> = Vec::with_capacity(cash_flows.len());

    for (date, amount) in cash_flows {
        let rust_date = py_to_date(&date)?;
        flows.push((rust_date, amount));
    }

    // Call the core XIRR function
    core_xirr(&flows, guess).map_err(core_to_py)
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_function(wrap_pyfunction!(py_xirr, module)?)?;
    Ok(vec!["xirr"])
}
