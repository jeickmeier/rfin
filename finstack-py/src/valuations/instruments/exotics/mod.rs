pub(crate) mod asian_option;
pub(crate) mod barrier_option;
pub(crate) mod basket;
pub(crate) mod lookback_option;

use pyo3::prelude::*;
use pyo3::types::PyModule;

pub(crate) fn register<'py>(
    py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let mut exports = Vec::new();

    let asian_exports = asian_option::register(py, module)?;
    exports.extend(asian_exports.iter().copied());

    let barrier_exports = barrier_option::register(py, module)?;
    exports.extend(barrier_exports.iter().copied());

    let basket_exports = basket::register(py, module)?;
    exports.extend(basket_exports.iter().copied());

    let lookback_exports = lookback_option::register(py, module)?;
    exports.extend(lookback_exports.iter().copied());

    Ok(exports)
}
