pub(crate) mod autocallable;
pub(crate) mod cliquet_option;
pub(crate) mod dcf;
#[allow(clippy::module_inception)]
pub(crate) mod equity;
pub(crate) mod equity_index_future;
pub(crate) mod equity_option;
pub(crate) mod levered_real_estate_equity;
pub(crate) mod private_markets_fund;
pub(crate) mod real_estate;
pub(crate) mod trs;
pub(crate) mod variance_swap;
pub(crate) mod vol_index_future;
pub(crate) mod vol_index_option;

use pyo3::prelude::*;
use pyo3::types::PyModule;

pub(crate) fn register<'py>(
    py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let mut exports = Vec::new();

    let equity_exports = equity::register(py, module)?;
    exports.extend(equity_exports.iter().copied());

    let equity_index_future_exports = equity_index_future::register(py, module)?;
    exports.extend(equity_index_future_exports.iter().copied());

    let equity_option_exports = equity_option::register(py, module)?;
    exports.extend(equity_option_exports.iter().copied());

    let convertible_exports = autocallable::register(py, module)?;
    exports.extend(convertible_exports.iter().copied());

    let cliquet_exports = cliquet_option::register(py, module)?;
    exports.extend(cliquet_exports.iter().copied());

    let trs_exports = trs::register(py, module)?;
    exports.extend(trs_exports.iter().copied());

    let variance_exports = variance_swap::register(py, module)?;
    exports.extend(variance_exports.iter().copied());

    let vol_future_exports = vol_index_future::register(py, module)?;
    exports.extend(vol_future_exports.iter().copied());

    let vol_option_exports = vol_index_option::register(py, module)?;
    exports.extend(vol_option_exports.iter().copied());

    let pmf_exports = private_markets_fund::register(py, module)?;
    exports.extend(pmf_exports.iter().copied());

    real_estate::register_module(module)?;
    exports.push("RealEstateAsset");

    levered_real_estate_equity::register_module(module)?;
    exports.push("LeveredRealEstateEquity");

    let dcf_exports = dcf::register(py, module)?;
    exports.extend(dcf_exports.iter().copied());

    Ok(exports)
}
