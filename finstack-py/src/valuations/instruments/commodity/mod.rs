pub(crate) mod commodity_asian_option;
pub(crate) mod commodity_forward;
pub(crate) mod commodity_option;
pub(crate) mod commodity_spread_option;
pub(crate) mod commodity_swap;
pub(crate) mod commodity_swaption;

use pyo3::prelude::*;
use pyo3::types::PyModule;

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let mut exports = Vec::new();

    commodity_forward::register(module)?;
    exports.push("CommodityForward");
    exports.push("CommodityForwardBuilder");

    commodity_option::register(module)?;
    exports.push("CommodityOption");
    exports.push("CommodityOptionBuilder");

    commodity_swap::register(module)?;
    exports.push("CommoditySwap");
    exports.push("CommoditySwapBuilder");

    commodity_spread_option::register(module)?;
    exports.push("CommoditySpreadOption");
    exports.push("CommoditySpreadOptionBuilder");

    commodity_swaption::register(module)?;
    exports.push("CommoditySwaption");
    exports.push("CommoditySwaptionBuilder");

    commodity_asian_option::register(module)?;
    exports.push("CommodityAsianOption");
    exports.push("CommodityAsianOptionBuilder");

    Ok(exports)
}
