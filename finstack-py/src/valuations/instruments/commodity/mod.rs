pub(crate) mod commodity_asian_option;
pub(crate) mod commodity_forward;
pub(crate) mod commodity_option;
pub(crate) mod commodity_swap;

use pyo3::prelude::*;
use pyo3::types::PyModule;

pub(crate) fn register<'py>(
    py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let mut exports = Vec::new();

    commodity_forward::register_module(module)?;
    exports.push("CommodityForward");
    exports.push("CommodityForwardBuilder");

    commodity_option::register_module(module)?;
    exports.push("CommodityOption");
    exports.push("CommodityOptionBuilder");

    commodity_swap::register_module(module)?;
    exports.push("CommoditySwap");
    exports.push("CommoditySwapBuilder");

    let asian_exports = commodity_asian_option::register(py, module)?;
    exports.extend(asian_exports.iter().copied());

    Ok(exports)
}
