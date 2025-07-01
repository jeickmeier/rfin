//! Python bindings for primitives module.

use pyo3::prelude::*;

/// Register primitives module components
pub fn register_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // TODO: Add Money class binding
    // TODO: Add Currency class binding

    m.add("__doc__", "Core financial primitives")?;
    Ok(())
}
