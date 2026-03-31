//! Python bindings for generic validation helpers.
//!
//! Wraps `finstack_core::validation::{require, require_or, require_with}`
//! so Python callers can reuse the same structural-invariant checks that
//! the Rust core provides.

use crate::errors::core_to_py;
use pyo3::prelude::*;
use pyo3::types::PyModule;

/// Assert that a condition is true, raising ``ValidationError`` otherwise.
///
/// Parameters
/// ----------
/// condition : bool
///     The invariant that must hold.
/// message : str
///     Error message when the condition is violated.
///
/// Raises
/// ------
/// ValidationError
///     If ``condition`` is ``False``.
#[pyfunction]
#[pyo3(text_signature = "(condition, message)")]
fn require(condition: bool, message: String) -> PyResult<()> {
    finstack_core::validation::require(condition, message).map_err(core_to_py)
}

/// Assert that a condition is true, raising the provided error otherwise.
///
/// Parameters
/// ----------
/// condition : bool
///     The invariant that must hold.
/// message : str
///     Error message when the condition is violated.
///
/// Raises
/// ------
/// ValidationError
///     If ``condition`` is ``False``.
#[pyfunction]
#[pyo3(text_signature = "(condition, message)")]
fn require_or(condition: bool, message: String) -> PyResult<()> {
    finstack_core::validation::require(condition, message).map_err(core_to_py)
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_function(wrap_pyfunction!(require, module)?)?;
    module.add_function(wrap_pyfunction!(require_or, module)?)?;
    Ok(vec!["require", "require_or"])
}
