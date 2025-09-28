mod primitives;

use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;

pub(crate) fn register<'py>(py: Python<'py>, parent: &Bound<'py, PyModule>) -> PyResult<()> {
    let module = PyModule::new(py, "cashflow")?;
    module.setattr(
        "__doc__",
        "Cash-flow primitives (cashflows, kinds) mirroring finstack-core.",
    )?;

    let exports = primitives::register(py, &module)?;
    module.setattr("__all__", PyList::new(py, &exports)?)?;

    parent.add_submodule(&module)?;
    Ok(())
}
