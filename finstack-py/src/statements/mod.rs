//! Python bindings for the finstack-statements crate.
//!
//! This module provides Python bindings for the financial statement modeling engine,
//! including types, builders, evaluators, extensions, and the metric registry system.

pub(crate) mod analysis;
pub(crate) mod builder;
pub(crate) mod error;
pub(crate) mod evaluator;
pub(crate) mod explain;
pub(crate) mod extensions;
pub(crate) mod registry;
pub(crate) mod reports;
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
    promote_exports(&module, "types", &types_exports)?;
    let builder_exports = builder::register(py, &module)?;
    promote_exports(&module, "builder", &builder_exports)?;
    let evaluator_exports = evaluator::register(py, &module)?;
    promote_exports(&module, "evaluator", &evaluator_exports)?;
    let extensions_exports = extensions::register(py, &module)?;
    promote_exports(&module, "extensions", &extensions_exports)?;
    let registry_exports = registry::register(py, &module)?;
    promote_exports(&module, "registry", &registry_exports)?;
    let analysis_exports = analysis::register(py, &module)?;
    promote_exports(&module, "analysis", &analysis_exports)?;
    let explain_exports = explain::register(py, &module)?;
    promote_exports(&module, "explain", &explain_exports)?;
    let reports_exports = reports::register(py, &module)?;
    promote_exports(&module, "reports", &reports_exports)?;

    // Collect all exports
    let mut all_exports = Vec::new();
    all_exports.extend(types_exports);
    all_exports.extend(builder_exports);
    all_exports.extend(evaluator_exports);
    all_exports.extend(extensions_exports);
    all_exports.extend(registry_exports);
    all_exports.extend(analysis_exports);
    all_exports.extend(explain_exports);
    all_exports.extend(reports_exports);

    // Set __all__ for the module
    let all_list = PyList::new(py, &all_exports)?;
    module.setattr("__all__", all_list)?;

    parent.add_submodule(&module)?;
    parent.setattr("statements", &module)?;

    Ok(())
}

fn promote_exports<'py>(
    parent: &Bound<'py, PyModule>,
    submodule_name: &str,
    exports: &[&str],
) -> PyResult<()> {
    if exports.is_empty() {
        return Ok(());
    }
    let submodule_any = parent.getattr(submodule_name)?;
    let submodule = submodule_any.downcast::<PyModule>()?;
    for &name in exports {
        if submodule.hasattr(name)? {
            let attr = submodule.getattr(name)?;
            parent.setattr(name, attr)?;
        }
    }
    Ok(())
}
