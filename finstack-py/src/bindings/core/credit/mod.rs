//! Python bindings for `finstack_core::credit`.

mod pd;
mod scoring;

use pyo3::prelude::*;
use pyo3::types::PyList;

/// Register the `credit` submodule on the parent `core` module.
pub fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(py, "credit")?;
    m.setattr(
        "__doc__",
        "Credit risk models: academic scoring (Altman, Ohlson, Zmijewski) and PD calibration.",
    )?;

    let pkg: String = match parent.getattr("__package__") {
        Ok(attr) => match attr.extract::<String>() {
            Ok(s) => s,
            Err(_) => "finstack.core".to_string(),
        },
        Err(_) => "finstack.core".to_string(),
    };
    let qual = format!("{pkg}.credit");
    m.setattr("__package__", &qual)?;

    scoring::register(py, &m)?;
    pd::register(py, &m)?;

    let all = PyList::new(py, ["scoring", "pd"])?;
    m.setattr("__all__", all)?;
    parent.add_submodule(&m)?;

    let sys = PyModule::import(py, "sys")?;
    let modules = sys.getattr("modules")?;
    modules.set_item(&qual, &m)?;

    Ok(())
}
