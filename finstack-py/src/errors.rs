//! Centralized error mapping from Rust crate errors to Python exceptions.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

/// Convert a `finstack_core::Error` into a Python exception.
pub fn core_to_py(e: finstack_core::Error) -> PyErr {
    PyValueError::new_err(e.to_string())
}

/// Convert any `Display`-able error into a Python `ValueError`.
pub fn display_to_py(e: impl std::fmt::Display) -> PyErr {
    PyValueError::new_err(e.to_string())
}
