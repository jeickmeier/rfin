//! Python bindings for the finstack-scenarios crate.
//!
//! This module provides Python bindings for the scenario engine, including
//! types for scenario specs, operations, execution context, and reports.

pub(crate) mod engine;
pub(crate) mod enums;
pub(crate) mod error;
pub(crate) mod reports;
pub(crate) mod spec;

use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;

/// Register the scenarios module and all its submodules.
pub(crate) fn register<'py>(py: Python<'py>, parent: &Bound<'py, PyModule>) -> PyResult<()> {
    let module = PyModule::new(py, "scenarios")?;
    module.setattr(
        "__doc__",
        concat!(
            "Deterministic scenario capability for stress testing and what-if analysis.\n\n",
            "This module provides tools for applying shocks to market data and financial ",
            "statement forecasts, enabling deterministic scenario analysis with stable ",
            "composition and priority-based conflict resolution."
        ),
    )?;

    // Register submodules and collect exports
    let enum_exports = enums::register(py, &module)?;
    let spec_exports = spec::register(py, &module)?;
    let reports_exports = reports::register(py, &module)?;
    let engine_exports = engine::register(py, &module)?;

    // Collect all exports
    let mut all_exports = Vec::new();
    all_exports.extend(enum_exports);
    all_exports.extend(spec_exports);
    all_exports.extend(reports_exports);
    all_exports.extend(engine_exports);

    // Set __all__ for the module
    let all_list = PyList::new(py, &all_exports)?;
    module.setattr("__all__", all_list)?;

    parent.add_submodule(&module)?;
    parent.setattr("scenarios", &module)?;

    Ok(())
}

