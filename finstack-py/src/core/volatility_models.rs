use crate::core::math::volatility::models;
use pyo3::prelude::*;
use pyo3::types::PyModule;
use pyo3::Bound;

/// Register stochastic volatility model parameter bindings at the core namespace.
///
/// This is a compatibility shim: the canonical implementation lives under
/// `crate::core::math::volatility::models`, mirroring `finstack-core`’s layout.
pub(crate) fn register<'py>(py: Python<'py>, parent: &Bound<'py, PyModule>) -> PyResult<()> {
    models::register(py, parent)
}
