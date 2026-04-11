//! Python bindings for the `finstack-core` crate.

use pyo3::prelude::*;
use pyo3::types::PyList;

/// Register the `core` submodule on the parent module.
pub fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(py, "core")?;
    m.setattr("__doc__", "Bindings for the finstack-core crate.")?;

    let all = PyList::empty(py);
    m.setattr("__all__", all)?;
    parent.add_submodule(&m)?;
    Ok(())
}
