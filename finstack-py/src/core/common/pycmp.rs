//! Shared helpers for implementing Python rich comparisons.

use pyo3::{basic::CompareOp, exceptions::PyValueError, prelude::*, IntoPyObjectExt};

#[inline]
pub fn richcmp_eq_ne<T: PartialEq>(
    py: Python<'_>,
    lhs: &T,
    rhs: Option<T>,
    op: CompareOp,
) -> PyResult<PyObject> {
    let ok = match op {
        CompareOp::Eq => rhs.map(|r| r == *lhs).unwrap_or(false),
        CompareOp::Ne => rhs.map(|r| r != *lhs).unwrap_or(true),
        _ => return Err(PyValueError::new_err("Unsupported comparison")),
    };
    let py_bool = ok.into_bound_py_any(py)?;
    Ok(py_bool.into())
}
