//! Python bindings for the `finstack-portfolio` crate.
//!
//! Portfolio contains `Arc<dyn Instrument>` which cannot be directly wrapped,
//! so this module exposes JSON-based construction via [`PortfolioSpec`],
//! result extraction via serde round-trips, and end-to-end pipeline functions
//! that build the runtime portfolio internally.

mod optimization;
mod pipeline;
mod replay;
mod spec;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyList;

/// Parse an ISO 8601 date string into a `time::Date`.
fn parse_date(s: &str) -> PyResult<time::Date> {
    let format = time::format_description::well_known::Iso8601::DEFAULT;
    time::Date::parse(s, &format)
        .map_err(|e| PyValueError::new_err(format!("Invalid date '{s}': {e}")))
}

/// Register the `portfolio` submodule on the parent module.
pub fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(py, "portfolio")?;
    m.setattr(
        "__doc__",
        "Portfolio construction, valuation, cashflows, scenarios, and metrics.",
    )?;

    spec::register(py, &m)?;
    pipeline::register(py, &m)?;
    optimization::register(py, &m)?;
    replay::register(py, &m)?;

    let exports = vec![
        "parse_portfolio_spec",
        "build_portfolio_from_spec",
        "portfolio_result_total_value",
        "portfolio_result_get_metric",
        "aggregate_metrics",
        "value_portfolio",
        "aggregate_cashflows",
        "apply_scenario_and_revalue",
        "optimize_portfolio",
        "replay_portfolio",
    ];

    let all = PyList::new(py, exports)?;
    m.setattr("__all__", all)?;
    parent.add_submodule(&m)?;

    let parent_name: String = match parent.getattr("__name__") {
        Ok(attr) => match attr.extract::<String>() {
            Ok(s) => s,
            Err(_) => "finstack.finstack".to_string(),
        },
        Err(_) => "finstack.finstack".to_string(),
    };
    let qual = format!("{parent_name}.portfolio");
    m.setattr("__package__", &qual)?;
    let sys = PyModule::import(py, "sys")?;
    let modules = sys.getattr("modules")?;
    modules.set_item(&qual, &m)?;

    Ok(())
}
