pub mod primitives;
pub mod xirr;

use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;

pub(crate) fn register<'py>(py: Python<'py>, parent: &Bound<'py, PyModule>) -> PyResult<()> {
    let module = PyModule::new(py, "cashflow")?;
    module.setattr(
        "__doc__",
        "Cash-flow primitives (cashflows, kinds) and analytics (XIRR) mirroring finstack-core.",
    )?;

    let mut exports = primitives::register(py, &module)?;
    let xirr_exports = xirr::register(py, &module)?;
    exports.extend(xirr_exports);
    
    module.setattr("__all__", PyList::new(py, &exports)?)?;

    parent.add_submodule(&module)?;
    Ok(())
}
