//! Python bindings for financial instruments.
//!
//! This module provides Python-friendly wrappers around finstack's instrument
//! types, focusing on instruments commonly used by credit analysts.

use pyo3::prelude::*;

pub mod bond;
pub mod loan;
pub mod swap;

// Re-export main types
pub use bond::PyBond;
pub use loan::{
    PyDelayedDrawTermLoan, PyDrawEvent, PyExpectedFundingCurve, PyLoan, PyRevolvingCreditFacility,
};
pub use swap::{PyFixedLeg, PyFloatLeg, PyInterestRateSwap, PyPayReceive};

/// Register the instruments submodule with Python
pub fn register_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "instruments")?;

    // Register instrument classes
    m.add_class::<PyBond>()?;
    m.add_class::<PyInterestRateSwap>()?;
    m.add_class::<PyPayReceive>()?;
    m.add_class::<PyFixedLeg>()?;
    m.add_class::<PyFloatLeg>()?;

    // Register loan-related classes
    m.add_class::<PyLoan>()?;
    m.add_class::<PyDrawEvent>()?;
    m.add_class::<PyExpectedFundingCurve>()?;
    m.add_class::<PyDelayedDrawTermLoan>()?;
    m.add_class::<PyRevolvingCreditFacility>()?;

    // Add the submodule to parent
    parent.add_submodule(&m)?;
    parent
        .py()
        .import("sys")?
        .getattr("modules")?
        .set_item("finstack.instruments", &m)?;

    Ok(())
}
