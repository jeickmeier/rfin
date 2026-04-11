//! Python bindings for the `finstack-core` crate.

mod config;
pub(crate) mod currency;
pub mod dates;
pub mod market_data;
mod math;
pub(crate) mod money;
mod types;

use pyo3::prelude::*;
use pyo3::types::PyList;

/// Register the `core` submodule on the parent module.
pub fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(py, "core")?;
    m.setattr("__doc__", "Bindings for the finstack-core crate.")?;
    m.setattr("__package__", "finstack.core")?;

    config::register(py, &m)?;
    types::register(py, &m)?;
    currency::register(py, &m)?;
    money::register(py, &m)?;
    math::register(py, &m)?;
    dates::register(py, &m)?;
    market_data::register(py, &m)?;

    let all = PyList::new(
        py,
        [
            "config",
            "types",
            "currency",
            "money",
            "math",
            "dates",
            "market_data",
        ],
    )?;
    m.setattr("__all__", all)?;
    parent.add_submodule(&m)?;
    Ok(())
}
