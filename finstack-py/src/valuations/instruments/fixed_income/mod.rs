pub(crate) mod agency_mbs;
pub(crate) mod bond;
pub(crate) mod bond_future;
pub(crate) mod convertible;
pub(crate) mod inflation_linked_bond;
pub(crate) mod revolving_credit;
pub(crate) mod structured_credit;
pub(crate) mod term_loan;

use pyo3::prelude::*;
use pyo3::types::PyModule;

pub(crate) fn register<'py>(
    py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let mut exports = Vec::new();

    let agency_mbs_exports = agency_mbs::register(py, module)?;
    exports.extend(agency_mbs_exports.iter().copied());

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

    Ok(exports)
}
