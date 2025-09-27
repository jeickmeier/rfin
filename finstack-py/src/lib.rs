#![allow(clippy::all)]

use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule, PyModuleMethods};
use pyo3::Bound;

mod config;
mod currency;
mod dates;
mod error;
mod money;
mod utils;

use config::{PyFinstackConfig, PyRoundingMode};
use currency::PyCurrency;
use dates::{PyBusinessDayConvention, PyCalendar};
use money::PyMoney;

/// Python bindings for the `finstack-core` crate.
#[pymodule]
fn finstack(py: Python<'_>, m: Bound<'_, PyModule>) -> PyResult<()> {
    m.setattr("__package__", "finstack")?;
    m.setattr(
        "__doc__",
        concat!(
            "High-level financial primitives from the Rust finstack core crate.\n\n",
            "These bindings surface currencies, configuration, money arithmetic, and ",
            "business-day calendars with Python-friendly docs and type hints. Downstream\n",
            "code should import from `finstack` directly.",
        ),
    )?;

    // Currency module and exports
    currency::register(py, &m)?;
    m.add_class::<PyCurrency>()?;

    // Config module and exports
    config::register(py, &m)?;
    m.add_class::<PyFinstackConfig>()?;
    m.add_class::<PyRoundingMode>()?;

    // Money module and exports
    money::register(py, &m)?;
    m.add_class::<PyMoney>()?;

    // Dates module and exports
    dates::register(py, &m)?;
    m.add_class::<PyBusinessDayConvention>()?;
    m.add_class::<PyCalendar>()?;

    // Re-export selected helpers at package root for convenience
    let dates_binding = m.getattr("dates")?;
    let dates_mod = dates_binding.downcast::<PyModule>()?;
    let adjust = dates_mod.getattr("adjust")?;
    m.setattr("adjust", adjust)?;
    for attr in [
        "available_calendars",
        "available_calendar_codes",
        "get_calendar",
    ] {
        if let Ok(value) = dates_mod.getattr(attr) {
            m.setattr(attr, value)?;
        }
    }

    let all = PyList::new(
        py,
        [
            "Currency",
            "Money",
            "FinstackConfig",
            "RoundingMode",
            "BusinessDayConvention",
            "Calendar",
            "adjust",
            "available_calendars",
            "available_calendar_codes",
            "get_calendar",
        ],
    )?;
    m.setattr("__all__", all)?;

    Ok(())
}
