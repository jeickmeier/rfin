//! Extended Internal Rate of Return (XIRR) for irregular cashflows.
//!
//! This module provides the XIRR calculation, which finds the annualized rate
//! of return for a series of cashflows with arbitrary timing. Unlike standard
//! IRR which assumes equal periods, XIRR handles real-world investment scenarios
//! with irregular payment dates.
//!
//! # Mathematical Foundation
//!
//! XIRR finds the rate `r` that satisfies:
//!
//! ```text
//! Σ CF_i / (1 + r)^((d_i - d_0) / 365) = 0
//! ```
//!
//! where:
//! - `CF_i` is the cashflow at date `d_i`
//! - `d_0` is the first cashflow date
//! - Time is measured in years using ACT/365
//!
//! # Algorithm
//!
//! The implementation uses Newton-Raphson iteration with:
//! - Automatic bracketing for difficult cases
//! - Convergence tolerance of 1e-10
//! - Maximum 100 iterations
//!
//! # References
//!
//! - Microsoft Excel XIRR function specification
//! - Numerical Recipes in C, 2nd ed., Press et al. (1992), Chapter 9
//!
//! # See Also
//!
//! - `finstack.core.cashflow.irr_periodic` for evenly-spaced cashflows
//! - `finstack.core.cashflow.npv` for present value calculations

use crate::core::dates::utils::py_to_date;
use crate::errors::{core_to_py, PyContext};
use finstack_core::cashflow::InternalRateOfReturn;
use pyo3::prelude::*;
use pyo3::types::PyModule;
use pyo3::Bound;

/// Calculate XIRR (Extended Internal Rate of Return) for irregular cashflows.
///
/// XIRR finds the annualized discount rate that makes the net present value of
/// all cashflows equal to zero. It's the standard method for measuring investment
/// performance when cashflows occur at arbitrary dates.
///
/// Parameters
/// ----------
/// cash_flows : list[tuple[datetime.date, float]]
///     List of (date, amount) pairs in any order (will be sorted internally).
///     Dates can be `date` objects or ISO-format strings (e.g., "2024-01-01").
///     Negative amounts represent outflows (investments), positive amounts
///     represent inflows (returns/distributions).
/// guess : float, optional
///     Initial guess for the IRR (default: 0.1 = 10%). Providing a reasonable
///     guess can help convergence for unusual return patterns.
///
/// Returns
/// -------
/// float
///     The XIRR as an annualized decimal (e.g., 0.15 for 15% annual return).
///
/// Raises
/// ------
/// ValueError
///     If less than 2 cashflows provided, or no sign change in cashflows
///     (i.e., all inflows or all outflows).
/// RuntimeError
///     If the solver cannot converge to a solution within 100 iterations.
///
/// Examples
/// --------
/// >>> from datetime import date
/// >>> from finstack.core.cashflow import xirr
///
/// >>> # Simple investment: one year holding period
/// >>> cash_flows = [
/// ...     (date(2024, 1, 1), -100000.0),   # Initial investment
/// ...     (date(2025, 1, 1), 110000.0)     # Final value
/// ... ]
/// >>> irr = xirr(cash_flows)
/// >>> print(f"IRR: {irr * 100:.2f}%")
/// IRR: 10.00%
///
/// >>> # Investment with irregular distributions
/// >>> cash_flows = [
/// ...     (date(2024, 1, 1), -100000.0),   # Initial investment
/// ...     (date(2024, 6, 15), 5000.0),     # Mid-year dividend
/// ...     (date(2024, 12, 31), 3000.0),    # Year-end dividend
/// ...     (date(2025, 1, 1), 110000.0)     # Final value
/// ... ]
/// >>> irr = xirr(cash_flows)
/// >>> print(f"IRR: {irr * 100:.2f}%")
///
/// >>> # Private equity style: multiple capital calls and distributions
/// >>> cash_flows = [
/// ...     (date(2020, 1, 15), -1000000.0),  # Initial capital call
/// ...     (date(2020, 6, 1), -500000.0),    # Second capital call
/// ...     (date(2021, 3, 15), 200000.0),    # Distribution
/// ...     (date(2022, 9, 30), 500000.0),    # Distribution
/// ...     (date(2024, 1, 1), 2500000.0)     # Final distribution/NAV
/// ... ]
/// >>> irr = xirr(cash_flows)
///
/// Notes
/// -----
/// - The XIRR is annualized regardless of the investment duration
/// - For investments less than one year, XIRR extrapolates the rate
/// - Time fractions use ACT/365 (actual days / 365)
/// - The solver uses Newton-Raphson with automatic bracketing
///
/// See Also
/// --------
/// irr_periodic : IRR for evenly-spaced periodic cashflows
/// npv : Net Present Value calculation
#[pyfunction(name = "xirr")]
#[pyo3(signature = (cash_flows, guess=None), text_signature = "(cash_flows, guess=None)")]
pub fn py_xirr(cash_flows: Vec<(Bound<'_, PyAny>, f64)>, guess: Option<f64>) -> PyResult<f64> {
    // Convert Python dates to Rust dates
    let mut flows: Vec<(finstack_core::dates::Date, f64)> = Vec::with_capacity(cash_flows.len());

    for (idx, (date, amount)) in cash_flows.into_iter().enumerate() {
        let field = format!("cash_flows[{idx}].date");
        let rust_date = py_to_date(&date).context(&field)?;
        flows.push((rust_date, amount));
    }

    // Call the core XIRR function
    flows.irr(guess).map_err(core_to_py)
}

/// Register XIRR function with the Python module.
pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_function(wrap_pyfunction!(py_xirr, module)?)?;
    Ok(vec!["xirr"])
}
