pub(crate) mod cds;
pub(crate) mod cds_index;
pub(crate) mod cds_option;
pub(crate) mod cds_tranche;

use pyo3::prelude::*;
use pyo3::types::PyModule;

pub(crate) fn register<'py>(
    py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let mut exports = Vec::new();

    let cds_exports = cds::register(py, module)?;
    exports.extend(cds_exports.iter().copied());

    let cds_index_exports = cds_index::register(py, module)?;
    exports.extend(cds_index_exports.iter().copied());

    let cds_option_exports = cds_option::register(py, module)?;
    exports.extend(cds_option_exports.iter().copied());

    let cds_tranche_exports = cds_tranche::register(py, module)?;
    exports.extend(cds_tranche_exports.iter().copied());

    Ok(exports)
}
