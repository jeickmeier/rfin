pub(crate) mod merton;

use pyo3::prelude::*;
use pyo3::types::PyModule;

pub(crate) fn register<'py>(
    py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let mut exports = Vec::new();

    let merton_exports = merton::register(py, module)?;
    exports.extend(merton_exports.iter().copied());

    Ok(exports)
}
