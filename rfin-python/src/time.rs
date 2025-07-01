//! Python bindings for dates module.

use pyo3::prelude::*;

/// Register dates module components
pub fn register_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // TODO: Add Date class binding
    // TODO: Add Calendar class binding
    // TODO: Add DayCount functions

    m.add(
        "__doc__",
        "Date and time handling for financial calculations",
    )?;
    Ok(())
}
