pub mod builder;
pub(crate) mod dataframe;
pub(crate) mod performance;
pub mod specs;
pub mod utils;

use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &pyo3::Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "cashflow")?;
    module.setattr(
        "__doc__",
        "Valuations cash-flow builder exposing complex coupon windows, PIK splits, and amortization.",
    )?;

    let mut exports: Vec<&str> = Vec::new();

    let builder_exports = builder::register(py, &module)?;
    exports.extend(builder_exports.iter().copied());

    let specs_exports = specs::register(py, &module)?;
    exports.extend(specs_exports.iter().copied());

    let utils_exports = utils::register(py, &module)?;
    exports.extend(utils_exports.iter().copied());

    exports.sort_unstable();
    exports.dedup();
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports)
}
