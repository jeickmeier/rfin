//! Python bindings for the performance analytics module.
//!
//! Maps to `finstack_analytics` in Rust, exposed as
//! `finstack.core.analytics` in Python (with a convenience alias at
//! `finstack.analytics` for ergonomic access).
//!
//! Standalone functions are registered directly on the module for
//! lightweight, slice-based analytics without constructing a `Performance`
//! object.

mod aggregation;
mod benchmark;
mod consecutive;
mod drawdown;
mod expr_plugin;
mod lookback;
mod performance;
mod returns;
mod risk_metrics;

use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;

/// Register the analytics module under the parent module.
pub fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(py, "analytics")?;
    m.setattr(
        "__doc__",
        "Performance analytics: returns, drawdowns, risk metrics, and benchmark-relative statistics.",
    )?;
    m.add_class::<performance::PyPerformance>()?;

    let mut exports: Vec<&'static str> = vec!["Performance"];

    exports.extend(returns::register(&m)?);
    exports.extend(drawdown::register(&m)?);
    exports.extend(risk_metrics::register(&m)?);
    exports.extend(benchmark::register(&m)?);
    exports.extend(lookback::register(&m)?);
    exports.extend(consecutive::register(&m)?);
    exports.extend(aggregation::register(&m)?);

    m.setattr("__all__", PyList::new(py, &exports)?)?;

    parent.add_submodule(&m)?;
    parent.setattr("analytics", &m)?;
    Ok(())
}
