#[allow(clippy::module_inception)]
pub(crate) mod fx;
pub(crate) mod fx_barrier_option;
pub(crate) mod fx_digital_option;
pub(crate) mod fx_forward;
pub(crate) mod fx_touch_option;
pub(crate) mod fx_variance_swap;
pub(crate) mod ndf;
pub(crate) mod quanto_option;

use pyo3::prelude::*;
use pyo3::types::PyModule;

pub(crate) fn register<'py>(
    py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let mut exports = Vec::new();

    let fx_exports = fx::register(py, module)?;
    exports.extend(fx_exports.iter().copied());

    let fx_forward_exports = fx_forward::register(py, module)?;
    exports.extend(fx_forward_exports.iter().copied());

    let ndf_exports = ndf::register(py, module)?;
    exports.extend(ndf_exports.iter().copied());

    let fx_digital_exports = fx_digital_option::register(py, module)?;
    exports.extend(fx_digital_exports.iter().copied());

    let fx_touch_exports = fx_touch_option::register(py, module)?;
    exports.extend(fx_touch_exports.iter().copied());

    let fx_variance_exports = fx_variance_swap::register(py, module)?;
    exports.extend(fx_variance_exports.iter().copied());

    let fx_barrier_exports = fx_barrier_option::register(py, module)?;
    exports.extend(fx_barrier_exports.iter().copied());

    let quanto_exports = quanto_option::register(py, module)?;
    exports.extend(quanto_exports.iter().copied());

    Ok(exports)
}
