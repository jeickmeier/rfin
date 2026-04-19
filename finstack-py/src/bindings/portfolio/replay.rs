//! Python binding for portfolio historical replay.

use crate::bindings::extract::extract_portfolio_ref;
use crate::errors::display_to_py;
use pyo3::prelude::*;
use serde::Deserialize;

/// Typed snapshot used during single-pass deserialization.
///
/// Parses each snapshot's `market` directly into a `MarketContext`, avoiding
/// the intermediate `serde_json::Value` tree and the per-entry subtree clone
/// the previous implementation performed.
#[derive(Deserialize)]
struct RawSnapshot {
    date: String,
    market: finstack_core::market_data::context::MarketContext,
}

/// Replay a portfolio through dated market snapshots.
///
/// Parameters
/// ----------
/// portfolio : Portfolio | str
///     A :class:`Portfolio` object (fast path) or a JSON-serialized
///     ``PortfolioSpec`` string.
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
fn replay_portfolio(
    portfolio: &Bound<'_, PyAny>,
    snapshots_json: &str,
    config_json: &str,
) -> PyResult<String> {
    let portfolio = extract_portfolio_ref(portfolio)?;

    let config: finstack_portfolio::replay::ReplayConfig =
        serde_json::from_str(config_json).map_err(display_to_py)?;

    let raw: Vec<RawSnapshot> = serde_json::from_str(snapshots_json).map_err(display_to_py)?;

    let format = time::format_description::well_known::Iso8601::DEFAULT;
    let mut snapshots = Vec::with_capacity(raw.len());
    for entry in raw {
        let date = time::Date::parse(&entry.date, &format).map_err(display_to_py)?;
        snapshots.push((date, entry.market));
    }

    let timeline =
        finstack_portfolio::replay::ReplayTimeline::new(snapshots).map_err(display_to_py)?;
    let finstack_config = finstack_core::config::FinstackConfig::default();

    let result = finstack_portfolio::replay::replay_portfolio(
        &portfolio,
        &timeline,
        &config,
        &finstack_config,
    )
    .map_err(display_to_py)?;

    serde_json::to_string(&result).map_err(display_to_py)
}

/// Register replay functions on the portfolio submodule.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(pyo3::wrap_pyfunction!(replay_portfolio, m)?)?;
    Ok(())
}
