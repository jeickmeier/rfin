//! Cash-flow primitives and analytics for Python bindings.
//!
//! This module provides Python bindings for finstack-core's cashflow functionality,
//! including cashflow types, classification, and time-value-of-money analytics.
//!
//! # Features
//!
//! - **Primitives**: [`CashFlow`] and [`CFKind`] for representing and classifying cashflows
//! - **XIRR**: Extended Internal Rate of Return for irregular cashflows
//! - **IRR**: Internal Rate of Return for periodic cashflows
//! - **NPV**: Net Present Value calculations with various day-count conventions
//! - **Discounting**: Present-value calculations using discount curves
//!
//! # Submodules
//!
//! - [`primitives`]: Core cashflow types and classification
//! - [`xirr`]: XIRR calculation for irregular cashflows
//! - [`performance`]: NPV and IRR calculations
//! - [`discounting`]: Curve-based present value calculations
//!
//! # See Also
//!
//! - `finstack_core::cashflow` for the underlying Rust implementation
//! - `finstack.core.market_data.term_structures` for discount curves

pub mod discounting;
pub mod performance;
pub mod primitives;
pub mod xirr;

use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;

/// Register the cashflow module and all submodules with Python.
pub(crate) fn register<'py>(py: Python<'py>, parent: &Bound<'py, PyModule>) -> PyResult<()> {
    let module = PyModule::new(py, "cashflow")?;
    module.setattr(
        "__doc__",
        "Cash-flow primitives and analytics for financial computations.

This module provides:
- CashFlow and CFKind: Core types for representing cashflows
- xirr: Extended Internal Rate of Return for irregular cashflows
- irr_periodic: IRR for evenly-spaced periodic cashflows
- npv: Net Present Value with configurable day-count conventions
- npv_static: Curve-based discounting with explicit day-count
- npv_using_curve_dc: Curve-based discounting using curve's day-count

Examples
--------
>>> from datetime import date
>>> from finstack.core.cashflow import CashFlow, CFKind, xirr, npv

>>> # Calculate XIRR for an investment
>>> cash_flows = [
...     (date(2024, 1, 1), -100000.0),
...     (date(2025, 1, 1), 115000.0)
... ]
>>> irr = xirr(cash_flows)

>>> # Create a structured cashflow
>>> cf = CashFlow(
...     date=date(2025, 6, 15),
...     amount=Money(2500, 'USD'),
...     kind=CFKind.FIXED,
...     accrual_factor=0.25
... )
",
    )?;

    let mut exports = primitives::register(py, &module)?;
    let xirr_exports = xirr::register(py, &module)?;
    exports.extend(xirr_exports);
    let perf_exports = performance::register(py, &module)?;
    exports.extend(perf_exports);
    let disc_exports = discounting::register(py, &module)?;
    exports.extend(disc_exports);

    module.setattr("__all__", PyList::new(py, &exports)?)?;

    parent.add_submodule(&module)?;
    Ok(())
}
