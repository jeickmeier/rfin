//! Error conversion for statements crate.

use pyo3::exceptions::PyRuntimeError;
use pyo3::PyErr;

/// Convert finstack-statements Error to PyErr
pub(crate) fn stmt_to_py(err: finstack_statements::Error) -> PyErr {
    PyRuntimeError::new_err(err.to_string())
}
