//! Python bindings for the `finstack-correlation` crate.
//!
//! Exposes copula models, factor models, recovery models, and correlation
//! utilities for credit portfolio modeling.

use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};

mod copulas;
mod factor_models;
mod recovery;
mod utils;

/// Register the correlation module and all its submodules.
pub(crate) fn register<'py>(py: Python<'py>, parent: &Bound<'py, PyModule>) -> PyResult<()> {
    let module = PyModule::new(py, "correlation")?;
    module.setattr(
        "__doc__",
        concat!(
            "Correlation infrastructure for credit portfolio modeling.\n\n",
            "Copula models, factor models, recovery models, and linear-algebra ",
            "utilities used across CDS tranche pricing, structured credit, and ",
            "portfolio credit risk."
        ),
    )?;

    let mut all_exports: Vec<&'static str> = Vec::new();

    register_submodule(py, &module, "copulas", &mut all_exports, copulas::register)?;
    register_submodule(
        py,
        &module,
        "factor_models",
        &mut all_exports,
        factor_models::register,
    )?;
    register_submodule(
        py,
        &module,
        "recovery",
        &mut all_exports,
        recovery::register,
    )?;
    register_submodule(py, &module, "utils", &mut all_exports, utils::register)?;

    let all_list = PyList::new(py, &all_exports)?;
    module.setattr("__all__", all_list)?;

    parent.add_submodule(&module)?;
    parent.setattr("correlation", &module)?;

    Ok(())
}

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
