use crate::core::market_data::volatility as md_vol;
use pyo3::prelude::*;
use pyo3::types::PyModule;
use pyo3::Bound;

/// Register volatility bindings at the core namespace.
///
/// This forwards to the shared market-data volatility bindings so the module
/// appears at both `finstack.core.volatility` and `finstack.core.market_data.volatility`.
pub(crate) fn register<'py>(py: Python<'py>, parent: &Bound<'py, PyModule>) -> PyResult<()> {
    // Reuse the existing module registration to avoid divergent behaviour.
    md_vol::register(py, parent).map(|_| ())
}
