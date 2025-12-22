pub mod config;
pub mod quote;
pub mod report;
pub mod sabr;
pub mod v2;
pub mod validation;

use finstack_core::collections::HashSet;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "calibration")?;
    module.setattr(
        "__doc__",
        "Calibration helpers mirroring finstack-valuations calibration interfaces.",
    )?;

    let mut exports: Vec<&str> = Vec::new();

    let config_exports = config::register(py, &module)?;
    exports.extend(config_exports.iter().copied());

    let quote_exports = quote::register(py, &module)?;
    exports.extend(quote_exports.iter().copied());

    let report_exports = report::register(py, &module)?;
    exports.extend(report_exports.iter().copied());

    let v2_exports = v2::register(py, &module)?;
    exports.extend(v2_exports.iter().copied());

    let validation_exports = validation::register(py, &module)?;
    exports.extend(validation_exports.iter().copied());

    let sabr_exports = sabr::register(py, &module)?;
    exports.extend(sabr_exports.iter().copied());

    let mut uniq = HashSet::default();
    exports.retain(|item| uniq.insert(*item));
    exports.sort_unstable();
    module.setattr("__all__", PyList::new(py, &exports)?)?;

    parent.add_submodule(&module)?;
    parent.setattr("calibration", &module)?;
    Ok(exports)
}
