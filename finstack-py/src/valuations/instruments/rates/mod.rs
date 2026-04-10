pub(crate) mod basis_swap;
pub(crate) mod cap_floor;
pub(crate) mod cms_option;
pub(crate) mod cms_swap;
pub(crate) mod common;
pub(crate) mod deposit;
pub(crate) mod fra;
pub(crate) mod inflation_cap_floor;
pub(crate) mod inflation_swap;
pub(crate) mod ir_future;
pub(crate) mod ir_future_option;
pub(crate) mod irs;
pub(crate) mod range_accrual;
pub(crate) mod repo;
pub(crate) mod swaption;
pub(crate) mod xccy_swap;

use pyo3::prelude::*;
use pyo3::types::PyModule;

pub(crate) fn register<'py>(
    py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let mut exports = Vec::new();

    let deposit_exports = deposit::register(py, module)?;
    exports.extend(deposit_exports.iter().copied());

    let basis_exports = basis_swap::register(py, module)?;
    exports.extend(basis_exports.iter().copied());

    let irs_exports = irs::register(py, module)?;
    exports.extend(irs_exports.iter().copied());

    let fra_exports = fra::register(py, module)?;
    exports.extend(fra_exports.iter().copied());

    let cap_floor_exports = cap_floor::register(py, module)?;
    exports.extend(cap_floor_exports.iter().copied());

    let ir_future_exports = ir_future::register(py, module)?;
    exports.extend(ir_future_exports.iter().copied());

    let ir_future_option_exports = ir_future_option::register(py, module)?;
    exports.extend(ir_future_option_exports.iter().copied());

    let swaption_exports = swaption::register(py, module)?;
    exports.extend(swaption_exports.iter().copied());

    let inflation_swap_exports = inflation_swap::register(py, module)?;
    exports.extend(inflation_swap_exports.iter().copied());

    let inflation_cap_floor_exports = inflation_cap_floor::register(py, module)?;
    exports.extend(inflation_cap_floor_exports.iter().copied());

    let repo_exports = repo::register(py, module)?;
    exports.extend(repo_exports.iter().copied());

    let xccy_swap_exports = xccy_swap::register(py, module)?;
    exports.extend(xccy_swap_exports.iter().copied());

    let cms_option_exports = cms_option::register(py, module)?;
    exports.extend(cms_option_exports.iter().copied());

    let cms_swap_exports = cms_swap::register(py, module)?;
    exports.extend(cms_swap_exports.iter().copied());

    let range_accrual_exports = range_accrual::register(py, module)?;
    exports.extend(range_accrual_exports.iter().copied());

    Ok(exports)
}
