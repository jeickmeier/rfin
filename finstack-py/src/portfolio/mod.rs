//! Python bindings for the finstack-portfolio crate.
//!
//! This module provides Python bindings for portfolio management, aggregation,
//! valuation, and metrics calculation.

pub(crate) mod attribution;
pub(crate) mod book;
pub(crate) mod builder;
pub(crate) mod cashflows;
pub(crate) mod dataframe;
pub(crate) mod error;
pub(crate) mod grouping;
pub(crate) mod margin;
pub(crate) mod metrics;
pub(crate) mod optimization;
pub(crate) mod positions;
pub(crate) mod results;
pub(crate) mod types;
pub(crate) mod valuation;

pub(crate) mod scenarios;

use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;

/// Register a named submodule on `parent`, populate it via `register_fn`,
/// and promote all its exports onto the parent module.
fn register_submodule<'py, F>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
    name: &str,
    all_exports: &mut Vec<String>,
    register_fn: F,
) -> PyResult<()>
where
    F: FnOnce(Python<'py>, &Bound<'py, PyModule>) -> PyResult<Vec<String>>,
{
    let submod = PyModule::new(py, name)?;
    let exports = register_fn(py, &submod)?;
    submod.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&submod)?;
    parent.setattr(name, &submod)?;
    for export_name in &exports {
        if submod.hasattr(export_name.as_str())? {
            let attr = submod.getattr(export_name.as_str())?;
            parent.setattr(export_name.as_str(), attr)?;
        }
    }
    all_exports.extend(exports);
    Ok(())
}

/// Register the portfolio module and all its submodules.
pub(crate) fn register<'py>(py: Python<'py>, parent: &Bound<'py, PyModule>) -> PyResult<()> {
    let module = PyModule::new(py, "portfolio")?;
    module.setattr(
        "__doc__",
        concat!(
            "Portfolio management and aggregation for finstack.\n\n",
            "This module provides portfolio-level operations including entity and position ",
            "management, valuation aggregation, metrics calculation, attribute-based grouping, ",
            "and DataFrame exports for analysis."
        ),
    )?;

    let mut all_exports: Vec<String> = Vec::new();

    register_submodule(py, &module, "types", &mut all_exports, types::register)?;
    register_submodule(py, &module, "book", &mut all_exports, book::register)?;
    register_submodule(
        py,
        &module,
        "portfolio",
        &mut all_exports,
        positions::register,
    )?;
    register_submodule(py, &module, "builder", &mut all_exports, builder::register)?;
    register_submodule(
        py,
        &module,
        "valuation",
        &mut all_exports,
        valuation::register,
    )?;
    register_submodule(py, &module, "metrics", &mut all_exports, metrics::register)?;
    register_submodule(
        py,
        &module,
        "optimization",
        &mut all_exports,
        optimization::register,
    )?;
    register_submodule(py, &module, "results", &mut all_exports, results::register)?;
    register_submodule(
        py,
        &module,
        "grouping",
        &mut all_exports,
        grouping::register,
    )?;
    register_submodule(
        py,
        &module,
        "attribution",
        &mut all_exports,
        attribution::register,
    )?;
    register_submodule(
        py,
        &module,
        "cashflows",
        &mut all_exports,
        cashflows::register,
    )?;
    register_submodule(
        py,
        &module,
        "dataframe",
        &mut all_exports,
        dataframe::register,
    )?;
    register_submodule(py, &module, "margin", &mut all_exports, margin::register)?;
    register_submodule(
        py,
        &module,
        "scenarios",
        &mut all_exports,
        scenarios::register,
    )?;

    let all_list = PyList::new(py, &all_exports)?;
    module.setattr("__all__", all_list)?;

    parent.add_submodule(&module)?;
    parent.setattr("portfolio", &module)?;

    Ok(())
}
