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

use crate::core::common::args::parse_day_count;
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

/// Calculate XIRR with an explicit day-count convention for irregular cashflows.
///
/// Similar to :func:`xirr`, but allows specifying the day-count convention
/// used to compute year fractions (e.g., Act/360, Act/365F, 30/360).
///
/// Parameters
/// ----------
/// cash_flows : list[tuple[datetime.date, float]]
///     List of (date, amount) pairs in any order (will be sorted internally).
/// day_count : DayCount or str
///     Day-count convention for computing year fractions. Accepts a DayCount
///     object or string identifier (e.g., "act365f", "act360", "30/360").
/// guess : float, optional
///     Initial guess for the IRR (default: 0.1 = 10%).
///
/// Returns
/// -------
/// float
///     The XIRR as an annualized decimal.
///
/// Raises
/// ------
/// ValueError
///     If less than 2 cashflows provided, or no sign change in cashflows.
/// RuntimeError
///     If the solver cannot converge.
///
/// Examples
/// --------
/// >>> from datetime import date
/// >>> from finstack.core.cashflow import xirr_with_daycount
///
/// >>> cash_flows = [
/// ...     (date(2024, 1, 1), -100000.0),
/// ...     (date(2025, 1, 1), 110000.0)
/// ... ]
/// >>> irr = xirr_with_daycount(cash_flows, "act360")
///
/// See Also
/// --------
/// xirr : XIRR with default Act/365F day count
/// irr_periodic_with_daycount : Periodic IRR (day count is ignored for periodic flows)
#[pyfunction(name = "xirr_with_daycount")]
#[pyo3(
    signature = (cash_flows, day_count, guess=None),
    text_signature = "(cash_flows, day_count, guess=None)"
)]
pub fn py_xirr_with_daycount(
    cash_flows: Vec<(Bound<'_, PyAny>, f64)>,
    day_count: Bound<'_, PyAny>,
    guess: Option<f64>,
) -> PyResult<f64> {
    let dc = parse_day_count(&day_count)?;
    let mut flows: Vec<(finstack_core::dates::Date, f64)> = Vec::with_capacity(cash_flows.len());
    for (idx, (date, amount)) in cash_flows.into_iter().enumerate() {
        let field = format!("cash_flows[{idx}].date");
        let rust_date = py_to_date(&date).context(&field)?;
        flows.push((rust_date, amount));
    }
    flows.irr_with_daycount(dc, guess).map_err(core_to_py)
}

/// Calculate periodic IRR with an explicit day-count convention.
///
/// For periodic (evenly-spaced) cashflows, the day count is ignored since periods
/// are unitless integers. This function is provided for API symmetry with
/// :func:`xirr_with_daycount`.
///
/// Parameters
/// ----------
/// amounts : list[float]
///     List of cashflow amounts in chronological order.
/// day_count : DayCount or str
///     Day-count convention (ignored for periodic cashflows but accepted for
///     API consistency).
/// guess : float, optional
///     Initial guess for the IRR (default: 0.1 = 10%).
///
/// Returns
/// -------
/// float
///     The IRR as a decimal per period.
///
/// Raises
/// ------
/// ValueError
///     If less than 2 cashflows provided or no sign change.
///
/// See Also
/// --------
/// irr_periodic : Periodic IRR without day-count parameter
/// xirr_with_daycount : XIRR with explicit day-count for dated cashflows
#[pyfunction(name = "irr_periodic_with_daycount")]
#[pyo3(
    signature = (amounts, day_count, guess=None),
    text_signature = "(amounts, day_count, guess=None)"
)]
pub fn py_irr_periodic_with_daycount(
    amounts: Vec<f64>,
    day_count: Bound<'_, PyAny>,
    guess: Option<f64>,
) -> PyResult<f64> {
    let dc = parse_day_count(&day_count)?;
    amounts.irr_with_daycount(dc, guess).map_err(core_to_py)
}

/// Count sign changes in a numeric sequence.
///
/// Returns the number of times the sign changes between consecutive non-zero
/// values. By Descartes' rule of signs, this bounds the number of positive
/// real roots of the NPV polynomial, indicating potential multiple IRR solutions.
///
/// Parameters
/// ----------
/// values : list[float]
///     Numeric sequence to analyze. Zero values are skipped.
///
/// Returns
/// -------
/// int
///     Number of sign changes in the sequence.
///
/// Examples
/// --------
/// >>> from finstack.core.cashflow import count_sign_changes
///
/// >>> count_sign_changes([-100, 50, 50])
/// 1
/// >>> count_sign_changes([-100, 200, -150])
/// 2
/// >>> count_sign_changes([0, 0, 100])
/// 0
///
/// Notes
/// -----
/// When the count exceeds 1, the cashflow pattern may admit multiple IRR
/// solutions. Use :func:`irr_detailed` or :func:`xirr_detailed` to obtain
/// root-ambiguity metadata alongside the computed rate.
///
/// See Also
/// --------
/// irr_detailed : IRR with root-ambiguity metadata
/// xirr_detailed : XIRR with root-ambiguity metadata
#[pyfunction(name = "count_sign_changes")]
#[pyo3(signature = (values,), text_signature = "(values)")]
pub fn py_count_sign_changes(values: Vec<f64>) -> usize {
    finstack_core::cashflow::count_sign_changes(values.into_iter())
}

/// Extended result from IRR calculation with root-ambiguity metadata.
///
/// Contains the computed rate along with information about whether the
/// cashflow pattern may admit multiple IRR solutions, based on the number
/// of sign changes in the cashflow sequence (Descartes' rule of signs).
///
/// Attributes
/// ----------
/// rate : float
///     The computed internal rate of return.
/// sign_changes : int
///     Number of sign changes in the cashflow sequence.
/// multiple_roots_possible : bool
///     Whether multiple roots are possible (sign_changes > 1).
#[pyclass(
    name = "IrrResult",
    module = "finstack.core.cashflow",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyIrrResult {
    inner: finstack_core::cashflow::IrrResult,
}

#[pymethods]
impl PyIrrResult {
    #[getter]
    /// The computed internal rate of return.
    fn rate(&self) -> f64 {
        self.inner.rate
    }

    #[getter]
    /// Number of sign changes in the cashflow sequence.
    fn sign_changes(&self) -> usize {
        self.inner.sign_changes
    }

    #[getter]
    /// Whether multiple roots are possible (sign_changes > 1).
    fn multiple_roots_possible(&self) -> bool {
        self.inner.multiple_roots_possible
    }

    fn __repr__(&self) -> String {
        format!(
            "IrrResult(rate={:.6}, sign_changes={}, multiple_roots_possible={})",
            self.inner.rate, self.inner.sign_changes, self.inner.multiple_roots_possible
        )
    }
}

/// Calculate IRR for periodic cashflows with root-ambiguity metadata.
///
/// Returns the IRR alongside information about the number of sign changes
/// in the cashflow sequence, which bounds the number of possible roots
/// by Descartes' rule of signs.
///
/// Parameters
/// ----------
/// amounts : list[float]
///     List of cashflow amounts in chronological order.
/// guess : float, optional
///     Initial guess for the IRR (default: 0.1 = 10%).
///
/// Returns
/// -------
/// IrrResult
///     Result containing the rate, sign change count, and multiple-roots flag.
///
/// Raises
/// ------
/// ValueError
///     If less than 2 cashflows provided or no sign change.
///
/// Examples
/// --------
/// >>> from finstack.core.cashflow import irr_detailed
///
/// >>> result = irr_detailed([-100, 230, -132, 5])
/// >>> result.rate  # the computed IRR
/// >>> result.sign_changes  # >= 3 for this pattern
/// >>> result.multiple_roots_possible  # True
///
/// See Also
/// --------
/// irr_periodic : Simple periodic IRR (rate only)
/// xirr_detailed : XIRR with root-ambiguity metadata for dated flows
#[pyfunction(name = "irr_detailed")]
#[pyo3(signature = (amounts, guess=None), text_signature = "(amounts, guess=None)")]
pub fn py_irr_detailed(amounts: Vec<f64>, guess: Option<f64>) -> PyResult<PyIrrResult> {
    finstack_core::cashflow::irr_detailed(&amounts, guess)
        .map(|inner| PyIrrResult { inner })
        .map_err(core_to_py)
}

/// Calculate XIRR for dated cashflows with root-ambiguity metadata.
///
/// Returns the XIRR alongside information about the number of sign changes
/// in the cashflow sequence, which bounds the number of possible roots
/// by Descartes' rule of signs.
///
/// Parameters
/// ----------
/// cash_flows : list[tuple[datetime.date, float]]
///     List of (date, amount) pairs in any order (will be sorted internally).
/// day_count : DayCount or str
///     Day-count convention for computing year fractions.
/// guess : float, optional
///     Initial guess for the IRR (default: 0.1 = 10%).
///
/// Returns
/// -------
/// IrrResult
///     Result containing the rate, sign change count, and multiple-roots flag.
///
/// Raises
/// ------
/// ValueError
///     If less than 2 cashflows provided, no sign change, or day count error.
///
/// Examples
/// --------
/// >>> from datetime import date
/// >>> from finstack.core.cashflow import xirr_detailed
///
/// >>> cash_flows = [
/// ...     (date(2024, 1, 1), -100000.0),
/// ...     (date(2025, 1, 1), 110000.0)
/// ... ]
/// >>> result = xirr_detailed(cash_flows, "act365f")
/// >>> result.rate
/// >>> result.sign_changes
/// 1
/// >>> result.multiple_roots_possible
/// False
///
/// See Also
/// --------
/// xirr : Simple XIRR (rate only)
/// irr_detailed : Periodic IRR with root-ambiguity metadata
#[pyfunction(name = "xirr_detailed")]
#[pyo3(
    signature = (cash_flows, day_count, guess=None),
    text_signature = "(cash_flows, day_count, guess=None)"
)]
pub fn py_xirr_detailed(
    cash_flows: Vec<(Bound<'_, PyAny>, f64)>,
    day_count: Bound<'_, PyAny>,
    guess: Option<f64>,
) -> PyResult<PyIrrResult> {
    let dc = parse_day_count(&day_count)?;
    let mut flows: Vec<(finstack_core::dates::Date, f64)> = Vec::with_capacity(cash_flows.len());
    for (idx, (date, amount)) in cash_flows.into_iter().enumerate() {
        let field = format!("cash_flows[{idx}].date");
        let rust_date = py_to_date(&date).context(&field)?;
        flows.push((rust_date, amount));
    }
    finstack_core::cashflow::xirr_detailed(&flows, dc, guess)
        .map(|inner| PyIrrResult { inner })
        .map_err(core_to_py)
}

/// Register XIRR functions with the Python module.
pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_function(wrap_pyfunction!(py_xirr, module)?)?;
    module.add_function(wrap_pyfunction!(py_xirr_with_daycount, module)?)?;
    module.add_function(wrap_pyfunction!(py_irr_periodic_with_daycount, module)?)?;
    module.add_function(wrap_pyfunction!(py_count_sign_changes, module)?)?;
    module.add_function(wrap_pyfunction!(py_irr_detailed, module)?)?;
    module.add_function(wrap_pyfunction!(py_xirr_detailed, module)?)?;
    module.add_class::<PyIrrResult>()?;
    Ok(vec![
        "xirr",
        "xirr_with_daycount",
        "irr_periodic_with_daycount",
        "count_sign_changes",
        "irr_detailed",
        "xirr_detailed",
        "IrrResult",
    ])
}
