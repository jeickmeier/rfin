//! Performance measurement utilities: IRR and NPV calculations.
//!
//! This module provides time-value-of-money analytics for investment performance
//! measurement, including Net Present Value (NPV) and Internal Rate of Return (IRR)
//! calculations.
//!
//! # Features
//!
//! - **NPV**: Net Present Value with configurable day-count conventions
//! - **IRR**: Internal Rate of Return for evenly-spaced periodic cashflows
//! - **Flexible dates**: Accept `date` objects or ISO-format strings
//!
//! # Mathematical Foundation
//!
//! ## Net Present Value (NPV)
//!
//! ```text
//! NPV = Σ CF_i / (1 + r)^(t_i)
//! ```
//!
//! where `CF_i` is the cashflow at time `t_i` and `r` is the discount rate.
//!
//! ## Internal Rate of Return (IRR)
//!
//! The IRR is the rate `r` that satisfies:
//!
//! ```text
//! Σ CF_i / (1 + r)^i = 0
//! ```
//!
//! where `i` is the period number (0, 1, 2, ...).
//!
//! # See Also
//!
//! - `finstack.core.cashflow.xirr` for irregular cashflows
//! - `finstack.core.cashflow.npv_static` for curve-based discounting

use crate::core::common::args::DayCountArg;
use crate::core::dates::utils::py_to_date;
use crate::core::dates::PyDayCount;
use crate::errors::{core_to_py, PyContext};
use finstack_core::cashflow::xirr::InternalRateOfReturn;
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::PyModule;
use pyo3::Bound;

/// Parse a day-count convention from Python input.
///
/// Accepts either a `DayCount` object or a string identifier.
fn parse_day_count(dc: Bound<'_, PyAny>) -> PyResult<finstack_core::dates::DayCount> {
    if let Ok(py_dc) = dc.extract::<PyRef<PyDayCount>>() {
        return Ok(py_dc.inner);
    }
    if let Ok(DayCountArg(inner)) = dc.extract::<DayCountArg>() {
        return Ok(inner);
    }
    Err(PyTypeError::new_err(
        "day_count must be a DayCount or string identifier",
    ))
}

/// Calculate NPV (Net Present Value) for a series of cashflows at a given discount rate.
///
/// Computes the present value of future cashflows discounted at a constant annual
/// rate. This is useful for simple investment analysis and comparing alternatives.
///
/// Parameters
/// ----------
/// cash_flows : list[tuple[date, float]]
///     List of (date, amount) pairs. Dates can be `date` objects or ISO-format
///     strings (e.g., "2024-01-01"). Negative amounts represent outflows
///     (investments), positive amounts represent inflows (returns).
/// discount_rate : float
///     Annual discount rate as a decimal (e.g., 0.10 for 10%).
/// base_date : date or str, optional
///     Valuation date from which time fractions are measured. Defaults to the
///     first cashflow date if not provided.
/// day_count : DayCount or str, optional
///     Day-count convention for computing year fractions. Accepts a DayCount
///     object or string identifier (e.g., "act365f", "act360", "30/360").
///
/// Returns
/// -------
/// float
///     The net present value of the cashflows.
///
/// Raises
/// ------
/// ValueError
///     If cashflows are empty or date calculations fail.
/// TypeError
///     If `day_count` is not a DayCount or string identifier.
///
/// Examples
/// --------
/// >>> from datetime import date
/// >>> from finstack.core.cashflow import npv
///
/// >>> # Simple investment: pay $100k today, receive $110k in one year
/// >>> cash_flows = [
/// ...     (date(2024, 1, 1), -100000),
/// ...     (date(2025, 1, 1), 110000)
/// ... ]
/// >>> npv(cash_flows, 0.05)
/// 4761.90...
///
/// >>> # Multi-year investment with interim cashflows
/// >>> cash_flows = [
/// ...     (date(2024, 1, 1), -100000),
/// ...     (date(2024, 7, 1), 5000),
/// ...     (date(2025, 1, 1), 5000),
/// ...     (date(2025, 7, 1), 105000)
/// ... ]
/// >>> npv(cash_flows, 0.08, day_count="act360")
///
/// Notes
/// -----
/// - For curve-based discounting, use `npv_static` or `npv_using_curve_dc`
/// - For irregular cashflows requiring XIRR, use `xirr`
///
/// See Also
/// --------
/// xirr : Extended IRR for irregular cashflows
/// irr_periodic : IRR for evenly-spaced cashflows
/// npv_static : Curve-based NPV with explicit day-count
#[pyfunction(name = "npv")]
#[pyo3(
    signature = (cash_flows, discount_rate, base_date=None, day_count=None),
    text_signature = "(cash_flows, discount_rate, base_date=None, day_count=None)"
)]
pub fn py_npv(
    cash_flows: Vec<(Bound<'_, PyAny>, f64)>,
    discount_rate: f64,
    base_date: Option<Bound<'_, PyAny>>,
    day_count: Option<Bound<'_, PyAny>>,
) -> PyResult<f64> {
    // Convert Python dates to Rust dates and amounts to Money
    let mut money_flows: Vec<(finstack_core::dates::Date, finstack_core::money::Money)> =
        Vec::with_capacity(cash_flows.len());

    for (idx, (date, amount)) in cash_flows.into_iter().enumerate() {
        let field = format!("cash_flows[{idx}].date");
        let rust_date = py_to_date(&date).context(&field)?;
        let money =
            finstack_core::money::Money::new(amount, finstack_core::currency::Currency::USD);
        money_flows.push((rust_date, money));
    }

    // Default base date logic matches old behavior (first flow date)
    let base = base_date
        .map(|d| py_to_date(&d).context("base_date"))
        .transpose()?
        .or_else(|| money_flows.first().map(|(d, _)| *d))
        .ok_or_else(|| {
            pyo3::exceptions::PyValueError::new_err("No cashflows provided to determine base date")
        })?;

    // Parse day count from input if provided, otherwise use Act365F
    let dc = match day_count {
        Some(dc_any) => parse_day_count(dc_any)?,
        None => finstack_core::dates::DayCount::Act365F,
    };

    finstack_core::cashflow::discounting::npv_constant(&money_flows, discount_rate, base, dc)
        .map(|m| m.amount())
        .map_err(core_to_py)
}

/// Calculate IRR (Internal Rate of Return) for evenly-spaced periodic cashflows.
///
/// Finds the discount rate that makes the net present value of all periodic
/// cashflows equal to zero. This is a simplified version of XIRR for cashflows
/// that occur at regular intervals (e.g., monthly, quarterly, or annual).
///
/// Parameters
/// ----------
/// amounts : list[float]
///     List of cashflow amounts in chronological order. Negative amounts
///     represent outflows (investments), positive amounts represent inflows
///     (returns). Must have at least 2 cashflows with a sign change.
/// guess : float, optional
///     Initial guess for the IRR (default: 0.1 = 10%). Providing a reasonable
///     guess can help convergence for difficult cases.
///
/// Returns
/// -------
/// float
///     The IRR as a decimal **per period**. For annual periods, this is the
///     annual IRR. For quarterly periods, this is the quarterly rate.
///
/// Raises
/// ------
/// ValueError
///     If less than 2 cashflows provided, no sign change in cashflows,
///     or the solver cannot converge to a solution.
///
/// Examples
/// --------
/// >>> from finstack.core.cashflow import irr_periodic
///
/// >>> # Annual cashflows: invest $100k, receive $110k after 1 year
/// >>> amounts = [-100000, 110000]
/// >>> irr = irr_periodic(amounts)
/// >>> print(f"Annual IRR: {irr * 100:.2f}%")
/// Annual IRR: 10.00%
///
/// >>> # Quarterly cashflows over 2 years
/// >>> amounts = [-100000, 3000, 3000, 3000, 3000, 3000, 3000, 3000, 90000]
/// >>> quarterly_irr = irr_periodic(amounts)
/// >>> # Convert to annual: (1 + quarterly_irr)^4 - 1
/// >>> annual_irr = (1 + quarterly_irr) ** 4 - 1
/// >>> print(f"Annualized IRR: {annual_irr * 100:.2f}%")
///
/// >>> # Monthly cashflows (e.g., lease payments)
/// >>> amounts = [-50000] + [1000] * 59 + [1000]  # 5-year lease
/// >>> monthly_irr = irr_periodic(amounts)
/// >>> annual_irr = (1 + monthly_irr) ** 12 - 1
///
/// Notes
/// -----
/// - The IRR is computed using Newton-Raphson iteration
/// - For irregular cashflows, use `xirr` instead
/// - To convert periodic IRR to annual: `annual = (1 + periodic)^n - 1`
///   where `n` is the number of periods per year
///
/// See Also
/// --------
/// xirr : Extended IRR for irregular (non-periodic) cashflows
/// npv : Net Present Value calculation
#[pyfunction(name = "irr_periodic")]
#[pyo3(
    signature = (amounts, guess=None),
    text_signature = "(amounts, guess=None)"
)]
pub fn py_irr_periodic(amounts: Vec<f64>, guess: Option<f64>) -> PyResult<f64> {
    amounts.irr(guess).map_err(core_to_py)
}

/// Register performance functions with the Python module.
pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_function(wrap_pyfunction!(py_npv, module)?)?;
    module.add_function(wrap_pyfunction!(py_irr_periodic, module)?)?;
    Ok(vec!["npv", "irr_periodic"])
}
