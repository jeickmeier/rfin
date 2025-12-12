//! Curve-based discounting utilities for cashflow present value calculations.
//!
//! This module provides functions to compute the Net Present Value (NPV) of a
//! series of cashflows using discount curves. It supports both explicit day-count
//! conventions and using the curve's internal day-count.
//!
//! # Features
//!
//! - **Curve-based NPV**: Use term structure discount curves for accurate present values
//! - **Flexible day-count**: Support for explicit or curve-derived day-count conventions
//! - **Currency-safe**: Returns `Money` with the same currency as input cashflows
//!
//! # Mathematical Foundation
//!
//! The net present value is calculated as:
//!
//! ```text
//! NPV = Σ CF_i × DF(t_i)
//! ```
//!
//! where:
//! - `CF_i` is the cashflow amount at time `t_i`
//! - `DF(t_i)` is the discount factor from the curve at time `t_i`
//! - Time fractions are computed using the specified day-count convention
//!
//! # See Also
//!
//! - `finstack.core.market_data.term_structures.DiscountCurve` for curve construction
//! - `finstack.core.cashflow.npv` for rate-based NPV calculations

use crate::core::common::args::DayCountArg;
use crate::core::dates::utils::py_to_date;
use crate::core::dates::PyDayCount;
use crate::core::market_data::term_structures::PyDiscountCurve;
use crate::core::money::{extract_money, PyMoney};
use crate::errors::{core_to_py, PyContext};
use finstack_core::cashflow::discounting::{npv_static, npv_using_curve_dc};
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
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
    Err(pyo3::exceptions::PyTypeError::new_err(
        "day_count must be a DayCount or string identifier",
    ))
}

/// Parse a list of (date, amount) tuples into typed cashflows.
fn parse_flows(
    flows: Vec<(Bound<'_, PyAny>, Bound<'_, PyAny>)>,
) -> PyResult<Vec<(finstack_core::dates::Date, finstack_core::money::Money)>> {
    let mut out = Vec::with_capacity(flows.len());
    for (date_any, amount_any) in flows {
        let date = py_to_date(&date_any).context("flows.date")?;
        let amount = extract_money(&amount_any).context("flows.amount")?;
        out.push((date, amount));
    }
    Ok(out)
}

/// Compute NPV of cashflows using a discount curve with explicit day-count.
///
/// Calculates the present value of a series of dated cashflows by interpolating
/// discount factors from the provided curve and applying the specified day-count
/// convention to compute time fractions.
///
/// Parameters
/// ----------
/// curve : DiscountCurve
///     The discount curve providing discount factors for each tenor.
/// base_date : date or str
///     The valuation date from which time fractions are measured.
/// day_count : DayCount or str
///     Day-count convention for computing year fractions. Accepts a DayCount
///     object or string identifier (e.g., "act365f", "act360", "30/360").
/// cash_flows : list[tuple[date, Money]]
///     List of (payment_date, amount) pairs. All amounts must share the same
///     currency for currency-safe computation.
///
/// Returns
/// -------
/// Money
///     The net present value in the same currency as the input cashflows.
///
/// Raises
/// ------
/// ValueError
///     If cashflows have mismatched currencies or dates are invalid.
/// RuntimeError
///     If curve interpolation fails.
///
/// Examples
/// --------
/// >>> from datetime import date
/// >>> from finstack import Money
/// >>> from finstack.core.cashflow import npv_static
/// >>> from finstack.core.market_data.term_structures import DiscountCurve
///
/// >>> # Create a discount curve
/// >>> curve = DiscountCurve.from_discount_factors(
/// ...     "USD-OIS",
/// ...     date(2024, 1, 1),
/// ...     [(0.0, 1.0), (1.0, 0.95), (2.0, 0.90)],
/// ...     "act365f"
/// ... )
///
/// >>> # Define cashflows
/// >>> cash_flows = [
/// ...     (date(2024, 6, 1), Money(50000, "USD")),
/// ...     (date(2025, 1, 1), Money(1050000, "USD"))
/// ... ]
///
/// >>> # Calculate NPV
/// >>> npv = npv_static(curve, date(2024, 1, 1), "act365f", cash_flows)
///
/// See Also
/// --------
/// npv_using_curve_dc : Use curve's internal day-count convention
/// finstack.core.cashflow.npv : Rate-based NPV for simple calculations
#[pyfunction(
    name = "npv_static",
    signature = (curve, base_date, day_count, cash_flows),
    text_signature = "(curve, base_date, day_count, cash_flows)"
)]
pub fn py_npv_static(
    curve: PyRef<PyDiscountCurve>,
    base_date: Bound<'_, PyAny>,
    day_count: Bound<'_, PyAny>,
    cash_flows: Vec<(Bound<'_, PyAny>, Bound<'_, PyAny>)>,
) -> PyResult<PyMoney> {
    let base = py_to_date(&base_date).context("base_date")?;
    let dc = parse_day_count(day_count)?;
    let flows = parse_flows(cash_flows)?;
    npv_static(curve.inner.as_ref(), base, dc, &flows)
        .map(PyMoney::new)
        .map_err(core_to_py)
}

/// Compute NPV of cashflows using the curve's internal day-count convention.
///
/// A convenience function that uses the day-count convention stored in the
/// discount curve itself, ensuring consistency between curve construction
/// and NPV calculation.
///
/// Parameters
/// ----------
/// curve : DiscountCurve
///     The discount curve providing discount factors. The curve's internal
///     day-count convention will be used for time fraction calculations.
/// base_date : date or str
///     The valuation date from which time fractions are measured.
/// cash_flows : list[tuple[date, Money]]
///     List of (payment_date, amount) pairs. All amounts must share the same
///     currency for currency-safe computation.
///
/// Returns
/// -------
/// Money
///     The net present value in the same currency as the input cashflows.
///
/// Raises
/// ------
/// ValueError
///     If cashflows have mismatched currencies or dates are invalid.
/// RuntimeError
///     If curve interpolation fails.
///
/// Examples
/// --------
/// >>> from datetime import date
/// >>> from finstack import Money
/// >>> from finstack.core.cashflow import npv_using_curve_dc
/// >>> from finstack.core.market_data.term_structures import DiscountCurve
///
/// >>> # Create a discount curve (stores day-count internally)
/// >>> curve = DiscountCurve.from_discount_factors(
/// ...     "USD-OIS",
/// ...     date(2024, 1, 1),
/// ...     [(0.0, 1.0), (1.0, 0.95)],
/// ...     "act365f"
/// ... )
///
/// >>> # Define cashflows
/// >>> cash_flows = [
/// ...     (date(2025, 1, 1), Money(105000, "USD"))
/// ... ]
///
/// >>> # Calculate NPV using curve's day-count
/// >>> npv = npv_using_curve_dc(curve, date(2024, 1, 1), cash_flows)
///
/// Notes
/// -----
/// This function is preferred over `npv_static` when you want to ensure the
/// same day-count convention is used for both curve calibration and NPV
/// calculation, avoiding potential inconsistencies.
///
/// See Also
/// --------
/// npv_static : Explicit day-count convention
#[pyfunction(
    name = "npv_using_curve_dc",
    signature = (curve, base_date, cash_flows),
    text_signature = "(curve, base_date, cash_flows)"
)]
pub fn py_npv_using_curve_dc(
    curve: PyRef<PyDiscountCurve>,
    base_date: Bound<'_, PyAny>,
    cash_flows: Vec<(Bound<'_, PyAny>, Bound<'_, PyAny>)>,
) -> PyResult<PyMoney> {
    let base = py_to_date(&base_date).context("base_date")?;
    let flows = parse_flows(cash_flows)?;
    npv_using_curve_dc(curve.inner.as_ref(), base, &flows)
        .map(PyMoney::new)
        .map_err(core_to_py)
}

/// Register discounting functions with the Python module.
pub(crate) fn register<'py>(
    py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.setattr(
        "__doc__",
        "Curve-based discounting utilities for cashflow present value calculations.

This module provides functions to compute NPV using discount curves:

- npv_static: NPV with explicit day-count convention
- npv_using_curve_dc: NPV using the curve's internal day-count

Mathematical Foundation
-----------------------
NPV = Σ CF_i × DF(t_i)

where CF_i is the cashflow at time t_i and DF(t_i) is the discount factor.

See Also
--------
finstack.core.cashflow.npv : Rate-based NPV calculations
finstack.core.market_data.term_structures : Discount curve construction
",
    )?;
    module.add_function(wrap_pyfunction!(py_npv_static, module)?)?;
    module.add_function(wrap_pyfunction!(py_npv_using_curve_dc, module)?)?;
    let exports = ["npv_static", "npv_using_curve_dc"];
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    Ok(exports.to_vec())
}
