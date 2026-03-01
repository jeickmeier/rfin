//! Python bindings for the performance analytics module.
//!
//! Maps to `finstack_core::analytics` in Rust, exposed as
//! `finstack.core.analytics` in Python (with a convenience alias at
//! `finstack.analytics` for ergonomic access).

mod expr_plugin;
mod performance;

use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;

/// Register the analytics module under the parent module.
pub fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(py, "analytics")?;
    m.setattr(
        "__doc__",
        "Performance analytics: returns, drawdowns, risk metrics, and benchmark-relative statistics.",
    )?;
    m.add_class::<performance::PyPerformance>()?;

    let exports = ["Performance"];
    m.setattr("__all__", PyList::new(py, exports)?)?;

    parent.add_submodule(&m)?;
    parent.setattr("analytics", &m)?;
    Ok(())
}
