//! Python bindings for primitives module.

pub mod currency;
pub mod money;

pub use currency::PyCurrency;
pub use money::PyMoney;

use pyo3::prelude::*;

/// Register primitives module components
pub fn register_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Add classes to the main primitives module
    m.add_class::<PyCurrency>()?;
    m.add_class::<PyMoney>()?;

    // Add common currency constants
    m.add(
        "USD",
        PyCurrency::from_inner(rfin_core::primitives::currency::Currency::USD),
    )?;
    m.add(
        "EUR",
        PyCurrency::from_inner(rfin_core::primitives::currency::Currency::EUR),
    )?;
    m.add(
        "GBP",
        PyCurrency::from_inner(rfin_core::primitives::currency::Currency::GBP),
    )?;
    m.add(
        "JPY",
        PyCurrency::from_inner(rfin_core::primitives::currency::Currency::JPY),
    )?;
    m.add(
        "CHF",
        PyCurrency::from_inner(rfin_core::primitives::currency::Currency::CHF),
    )?;
    m.add(
        "AUD",
        PyCurrency::from_inner(rfin_core::primitives::currency::Currency::AUD),
    )?;
    m.add(
        "CAD",
        PyCurrency::from_inner(rfin_core::primitives::currency::Currency::CAD),
    )?;

    m.add(
        "__doc__",
        "Core financial primitives including Currency and Money types",
    )?;
    Ok(())
}
