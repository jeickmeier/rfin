//! Python bindings for `finstack_core::math`.

mod consecutive;
mod linalg;
mod special_functions;
mod stats;
mod summation;

use pyo3::prelude::*;
use pyo3::types::PyList;

/// Register the `math` submodule on the parent `core` module.
pub fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(py, "math")?;
    m.setattr(
        "__doc__",
        "Numerical helpers: linear algebra, statistics, special functions, summation.",
    )?;

    let pkg: String = match parent.getattr("__package__") {
        Ok(attr) => match attr.extract::<String>() {
            Ok(s) => s,
            Err(_) => "finstack.core".to_string(),
        },
        Err(_) => "finstack.core".to_string(),
    };
    let qual = format!("{pkg}.math");
    m.setattr("__package__", &qual)?;

    consecutive::register(py, &m)?;
    linalg::register(py, &m)?;
    stats::register(py, &m)?;
    special_functions::register(py, &m)?;
    summation::register(py, &m)?;

    let all = PyList::new(
        py,
        [
            "consecutive",
            "linalg",
            "stats",
            "special_functions",
            "summation",
        ],
    )?;
    m.setattr("__all__", all)?;
    parent.add_submodule(&m)?;

    let sys = PyModule::import(py, "sys")?;
    let modules = sys.getattr("modules")?;
    modules.set_item(&qual, &m)?;

    Ok(())
}
