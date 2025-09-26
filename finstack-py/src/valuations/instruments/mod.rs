//! Python bindings for financial instruments.
//!
//! This module provides Python-friendly wrappers around finstack's instrument
//! types, focusing on instruments commonly used by credit analysts.

use pyo3::prelude::*;
use std::str::FromStr;

pub mod bond;
pub mod swap;

// Re-export main types
pub use bond::PyBond;
pub use swap::{PyFixedLeg, PyFloatLeg, PyInterestRateSwap, PyPayReceive};

// Shared helper to parse metric names into MetricId values
pub(crate) fn parse_metric_ids(
    metrics: &[String],
) -> Vec<finstack_valuations::metrics::MetricId> {
    use finstack_valuations::metrics::MetricId;

    metrics
        .iter()
        .map(|name| MetricId::from_str(name))
        .collect::<Result<_, _>>()
        .unwrap_or_else(|_| metrics.iter().map(MetricId::custom).collect())
}

/// Register the instruments submodule with Python
pub fn register_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "instruments")?;

    // Register instrument classes
    m.add_class::<PyBond>()?;
    m.add_class::<PyInterestRateSwap>()?;
    m.add_class::<PyPayReceive>()?;
    m.add_class::<PyFixedLeg>()?;
    m.add_class::<PyFloatLeg>()?;

    // Loan and revolver bindings removed

    // Add the submodule to parent
    parent.add_submodule(&m)?;
    parent
        .py()
        .import("sys")?
        .getattr("modules")?
        .set_item("finstack.instruments", &m)?;

    Ok(())
}
