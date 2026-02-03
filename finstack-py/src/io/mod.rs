//! Python bindings for the finstack-io crate.
//!
//! This module provides Python bindings for the persistence layer, including
//! the `SqliteStore` for storing and retrieving market contexts, instruments,
//! portfolios, scenarios, statement models, and metric registries.

pub(crate) mod error;
pub(crate) mod store;
pub(crate) mod types;

use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;

/// Register the io module and all its exports.
pub(crate) fn register<'py>(py: Python<'py>, parent: &Bound<'py, PyModule>) -> PyResult<()> {
    let module = PyModule::new(py, "io")?;
    module.setattr(
        "__doc__",
        concat!(
            "Persistence layer for Finstack domain objects.\n\n",
            "This module provides a typed repository interface for storing and retrieving ",
            "market contexts, instruments, portfolios, scenarios, statement models, and ",
            "metric registries. The default implementation uses SQLite."
        ),
    )?;

    // Register exceptions
    error::register_exceptions(py, &module)?;

    // Register types
    let type_exports = types::register(py, &module)?;

    // Register store
    let store_exports = store::register(py, &module)?;

    // Collect all exports
    let mut all_exports = Vec::new();
    all_exports.extend(type_exports.iter().map(|s| s.as_str()));
    all_exports.extend(store_exports.iter().map(|s| s.as_str()));
    // Add exception names
    all_exports.push("IoError");
    all_exports.push("NotFoundError");
    all_exports.push("SchemaVersionError");

    // Set __all__ for the module
    let all_list = PyList::new(py, &all_exports)?;
    module.setattr("__all__", all_list)?;

    parent.add_submodule(&module)?;
    parent.setattr("io", &module)?;

    Ok(())
}
