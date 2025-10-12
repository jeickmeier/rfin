//! Python bindings for the finstack-statements crate.
//!
//! This module provides Python bindings for the financial statement modeling engine,
//! including types, builders, evaluators, extensions, and the metric registry system.

pub(crate) mod builder;
pub(crate) mod error;
pub(crate) mod evaluator;
pub(crate) mod extensions;
pub(crate) mod registry;
pub(crate) mod types;
pub(crate) mod utils;

use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;

/// Register the statements module and all its submodules.
pub(crate) fn register<'py>(py: Python<'py>, parent: &Bound<'py, PyModule>) -> PyResult<()> {
    let module = PyModule::new(py, "statements")?;
    module.setattr(
        "__doc__",
        concat!(
            "Financial statement modeling engine.\n\n",
            "This module provides tools for building, evaluating, and analyzing ",
            "financial statement models with deterministic evaluation, currency-safe ",
            "arithmetic, and support for forecasting methods, extensions, and dynamic ",
            "metric registries."
        ),
    )?;

    // Register submodules
    let types_exports = types::register(py, &module)?;
    let builder_exports = builder::register(py, &module)?;
    let evaluator_exports = evaluator::register(py, &module)?;
    let extensions_exports = extensions::register(py, &module)?;
    let registry_exports = registry::register(py, &module)?;

    // Collect all exports
    let mut all_exports = Vec::new();
    all_exports.extend(types_exports);
    all_exports.extend(builder_exports);
    all_exports.extend(evaluator_exports);
    all_exports.extend(extensions_exports);
    all_exports.extend(registry_exports);

    // Set __all__ for the module
    let all_list = PyList::new(py, &all_exports)?;
    module.setattr("__all__", all_list)?;

    parent.add_submodule(&module)?;
    parent.setattr("statements", &module)?;

    Ok(())
}
