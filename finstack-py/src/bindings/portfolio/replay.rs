//! Python binding for portfolio historical replay.

use crate::errors::display_to_py;
use pyo3::prelude::*;

/// Replay a portfolio through dated market snapshots.
///
/// Parameters
/// ----------
/// spec_json : str
///     JSON-serialized ``PortfolioSpec``.
/// snapshots_json : str
///     JSON array of ``{"date": "YYYY-MM-DD", "market": {...}}`` objects.
///     Markets use the standard ``MarketContextState`` JSON format.
/// config_json : str
///     JSON-serialized ``ReplayConfig``.
///
/// Returns
/// -------
/// str
///     JSON-serialized ``ReplayResult``.
#[pyfunction]
fn replay_portfolio(spec_json: &str, snapshots_json: &str, config_json: &str) -> PyResult<String> {
    let spec: finstack_portfolio::PortfolioSpec =
        serde_json::from_str(spec_json).map_err(display_to_py)?;
    let portfolio = finstack_portfolio::Portfolio::from_spec(spec).map_err(display_to_py)?;

    let config: finstack_portfolio::ReplayConfig =
        serde_json::from_str(config_json).map_err(display_to_py)?;

    // Parse snapshots: [{"date": "YYYY-MM-DD", "market": {...}}, ...]
    let raw: Vec<serde_json::Value> =
        serde_json::from_str(snapshots_json).map_err(display_to_py)?;

    let format = time::format_description::well_known::Iso8601::DEFAULT;
    let mut snapshots = Vec::with_capacity(raw.len());
    for entry in &raw {
        let date_str = entry["date"]
            .as_str()
            .ok_or_else(|| display_to_py("each snapshot must have a 'date' string field"))?;
        let date = time::Date::parse(date_str, &format).map_err(display_to_py)?;
        let market: finstack_core::market_data::context::MarketContext =
            serde_json::from_value(entry["market"].clone()).map_err(display_to_py)?;
        snapshots.push((date, market));
    }

    let timeline = finstack_portfolio::ReplayTimeline::new(snapshots).map_err(display_to_py)?;
    let finstack_config = finstack_core::config::FinstackConfig::default();

    let result =
        finstack_portfolio::replay_portfolio(&portfolio, &timeline, &config, &finstack_config)
            .map_err(display_to_py)?;

    serde_json::to_string(&result).map_err(display_to_py)
}

/// Register replay functions on the portfolio submodule.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(pyo3::wrap_pyfunction!(replay_portfolio, m)?)?;
    Ok(())
}
