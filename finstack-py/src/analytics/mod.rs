//! Python bindings for the performance analytics module.

mod expr_plugin;
mod performance;

use pyo3::prelude::*;
use pyo3::types::PyModule;
use pyo3::Bound;

/// Register the analytics module under the parent module.
pub fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(py, "analytics")?;
    m.setattr(
        "__doc__",
        "Performance analytics: returns, drawdowns, risk metrics, and benchmark-relative statistics.",
    )?;
    m.add_class::<performance::PyPerformance>()?;
    parent.add_submodule(&m)?;
    Ok(())
}
