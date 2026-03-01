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

/// Register a named submodule on `parent`, populate it via `register_fn`,
/// and promote all its exports onto the parent module.
fn register_submodule<'py, F>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
    name: &str,
    all_exports: &mut Vec<&'static str>,
    register_fn: F,
) -> PyResult<()>
where
    F: FnOnce(Python<'py>, &Bound<'py, PyModule>) -> PyResult<Vec<&'static str>>,
{
    let submod = PyModule::new(py, name)?;
    let exports = register_fn(py, &submod)?;
    submod.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&submod)?;
    parent.setattr(name, &submod)?;
    for &export_name in &exports {
        if submod.hasattr(export_name)? {
            let attr = submod.getattr(export_name)?;
            parent.setattr(export_name, attr)?;
        }
    }
    all_exports.extend(exports);
    Ok(())
}

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

    let mut all_exports: Vec<&'static str> = Vec::new();

    register_submodule(py, &module, "enums", &mut all_exports, enums::register)?;
    register_submodule(py, &module, "spec", &mut all_exports, spec::register)?;
    register_submodule(py, &module, "reports", &mut all_exports, reports::register)?;
    register_submodule(py, &module, "engine", &mut all_exports, engine::register)?;

    let all_list = PyList::new(py, &all_exports)?;
    module.setattr("__all__", all_list)?;

    parent.add_submodule(&module)?;
    parent.setattr("scenarios", &module)?;

    Ok(())
}
