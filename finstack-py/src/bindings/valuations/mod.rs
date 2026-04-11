//! Python bindings for the `finstack-valuations` crate.

use pyo3::prelude::*;
use pyo3::types::PyList;

/// Register the `valuations` submodule on the parent module.
pub fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(py, "valuations")?;
    m.setattr("__doc__", "Bindings for the finstack-valuations crate.")?;

    let all = PyList::empty(py);
    m.setattr("__all__", all)?;
    parent.add_submodule(&m)?;
    Ok(())
}
