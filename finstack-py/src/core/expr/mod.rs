use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;

pub(crate) fn register<'py>(py: Python<'py>, parent: &Bound<'py, PyModule>) -> PyResult<()> {
    let module = PyModule::new(py, "expr")?;
    module.setattr(
        "__doc__",
        "Expression engine bindings (AST, compilation, evaluation).",
    )?;

    // TODO: Add Expr, Function, CompiledExpr bindings here
    // For now, we just register the module to reserve the namespace parity.

    let exports: [&str; 0] = [];
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;
    Ok(())
}

