//! Standalone analytics functions mirroring the `finstack_analytics` crate root.

mod aggregation;
mod benchmark;
mod drawdown;
mod returns;
mod risk_metrics;

use pyo3::prelude::*;

pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    aggregation::register(m)?;
    benchmark::register(m)?;
    drawdown::register(m)?;
    returns::register(m)?;
    risk_metrics::register(m)?;
    Ok(())
}
