pub(crate) mod bond;
pub(crate) mod bond_future;
pub(crate) mod cmo;
pub(crate) mod common;
pub(crate) mod convertible;
pub(crate) mod dollar_roll;
pub(crate) mod fi_trs;
pub(crate) mod inflation_linked_bond;
pub(crate) mod mbs_passthrough;
pub(crate) mod revolving_credit;
pub(crate) mod structured_credit;
pub(crate) mod tba;
pub(crate) mod term_loan;

use pyo3::prelude::*;
use pyo3::types::PyModule;

pub(crate) fn register<'py>(
    py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let mut exports = Vec::new();

    let mbs_passthrough_exports = mbs_passthrough::register(py, module)?;
    exports.extend(mbs_passthrough_exports.iter().copied());

    let tba_exports = tba::register(py, module)?;
    exports.extend(tba_exports.iter().copied());

    let dollar_roll_exports = dollar_roll::register(py, module)?;
    exports.extend(dollar_roll_exports.iter().copied());

    let cmo_exports = cmo::register(py, module)?;
    exports.extend(cmo_exports.iter().copied());

    let bond_exports = bond::register(py, module)?;
    exports.extend(bond_exports.iter().copied());

    let bond_future_exports = bond_future::register(py, module)?;
    exports.extend(bond_future_exports.iter().copied());

    let convertible_exports = convertible::register(py, module)?;
    exports.extend(convertible_exports.iter().copied());

    let ilb_exports = inflation_linked_bond::register(py, module)?;
    exports.extend(ilb_exports.iter().copied());

    let revolving_credit_exports = revolving_credit::register(py, module)?;
    exports.extend(revolving_credit_exports.iter().copied());

    let structured_credit_exports = structured_credit::register(py, module)?;
    exports.extend(structured_credit_exports.iter().copied());

    let term_loan_exports = term_loan::register(py, module)?;
    exports.extend(term_loan_exports.iter().copied());

    let fi_trs_exports = fi_trs::register(py, module)?;
    exports.extend(fi_trs_exports.iter().copied());

    Ok(exports)
}
