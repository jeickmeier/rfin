pub mod discounting;
pub mod performance;
pub mod primitives;
pub mod xirr;

use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;

pub(crate) fn register<'py>(py: Python<'py>, parent: &Bound<'py, PyModule>) -> PyResult<()> {
    let module = PyModule::new(py, "cashflow")?;
    module.setattr(
        "__doc__",
        "Cash-flow primitives (cashflows, kinds) and analytics (XIRR, IRR, NPV) mirroring finstack-core.",
    )?;

    let mut exports = primitives::register(py, &module)?;
    let xirr_exports = xirr::register(py, &module)?;
    exports.extend(xirr_exports);
    let perf_exports = performance::register(py, &module)?;
    exports.extend(perf_exports);
    let disc_exports = discounting::register(py, &module)?;
    exports.extend(disc_exports);

    module.setattr("__all__", PyList::new(py, &exports)?)?;

    parent.add_submodule(&module)?;
    Ok(())
}
