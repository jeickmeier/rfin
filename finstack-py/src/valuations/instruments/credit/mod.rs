pub(crate) mod dynamic_recovery;
pub(crate) mod endogenous_hazard;
pub(crate) mod mc_config;
pub(crate) mod merton;
pub(crate) mod toggle_exercise;

use pyo3::prelude::*;
use pyo3::types::PyModule;

pub(crate) fn register<'py>(
    py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let mut exports = Vec::new();

    let merton_exports = merton::register(py, module)?;
    exports.extend(merton_exports.iter().copied());

    let endogenous_exports = endogenous_hazard::register(py, module)?;
    exports.extend(endogenous_exports.iter().copied());

    let recovery_exports = dynamic_recovery::register(py, module)?;
    exports.extend(recovery_exports.iter().copied());

    let toggle_exports = toggle_exercise::register(py, module)?;
    exports.extend(toggle_exports.iter().copied());

    let mc_config_exports = mc_config::register(py, module)?;
    exports.extend(mc_config_exports.iter().copied());

    Ok(exports)
}
