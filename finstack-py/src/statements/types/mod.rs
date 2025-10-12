//! Type bindings for statements crate.

pub(crate) mod forecast;
pub(crate) mod model;
pub(crate) mod node;
pub(crate) mod value;

use pyo3::prelude::*;
use pyo3::types::PyModule;
use pyo3::Bound;

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "types")?;
    module.setattr("__doc__", "Core types for statement modeling.")?;

    // Register all type classes
    node::register(py, &module)?;
    forecast::register(py, &module)?;
    value::register(py, &module)?;
    model::register(py, &module)?;

    parent.add_submodule(&module)?;
    parent.setattr("types", &module)?;

    Ok(vec![
        "NodeType",
        "NodeSpec",
        "ForecastMethod",
        "ForecastSpec",
        "SeasonalMode",
        "AmountOrScalar",
        "FinancialModelSpec",
        "CapitalStructureSpec",
        "DebtInstrumentSpec",
    ])
}
