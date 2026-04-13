//! Instrument pricing pipeline: JSON instrument + market → ValuationResult.

use crate::bindings::extract::extract_market;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

fn val_to_py(e: impl std::fmt::Display) -> PyErr {
    PyValueError::new_err(e.to_string())
}

/// Price an instrument from its tagged JSON and return a ``ValuationResult`` JSON.
///
/// Parameters
/// ----------
/// instrument_json : str
///     Tagged instrument JSON (``{"type": "bond", ...}``).
/// market : MarketContext | str
///     A ``MarketContext`` object or a JSON string.
/// as_of : str
///     Valuation date in ISO 8601 format (``"YYYY-MM-DD"``).
/// model : str
///     Model key: ``"discounting"`` (default), ``"black76"``, ``"hazard_rate"``,
///     ``"hull_white_1f"``, ``"tree"``, ``"normal"``, ``"monte_carlo_gbm"``, etc.
///
/// Returns
/// -------
/// str
///     JSON-serialized ``ValuationResult``.
#[pyfunction]
#[pyo3(signature = (instrument_json, market, as_of, model="discounting"))]
fn price_instrument(
    instrument_json: &str,
    market: &Bound<'_, PyAny>,
    as_of: &str,
    model: &str,
) -> PyResult<String> {
    let inst: finstack_valuations::instruments::InstrumentJson =
        serde_json::from_str(instrument_json).map_err(val_to_py)?;
    let boxed = inst.into_boxed().map_err(val_to_py)?;

    let market = extract_market(market)?;

    let date = super::parse_date(as_of)?;
    let model_key = parse_model_key(model)?;

    let registry = finstack_valuations::pricer::standard_registry();
    let result = registry
        .price(boxed.as_ref(), model_key, &market, date, None)
        .map_err(val_to_py)?;

    serde_json::to_string_pretty(&result).map_err(val_to_py)
}

/// Price an instrument with explicit metric requests.
///
/// Parameters
/// ----------
/// instrument_json : str
///     Tagged instrument JSON.
/// market : MarketContext | str
///     A ``MarketContext`` object or a JSON string.
/// as_of : str
///     Valuation date.
/// model : str
///     Model key string.
/// metrics : list[str]
///     Metric identifiers to compute (e.g. ``["ytm", "dv01", "modified_duration"]``).
///
/// Returns
/// -------
/// str
///     JSON-serialized ``ValuationResult`` including requested metrics.
#[pyfunction]
#[pyo3(signature = (instrument_json, market, as_of, model="discounting", metrics=vec![]))]
fn price_instrument_with_metrics(
    instrument_json: &str,
    market: &Bound<'_, PyAny>,
    as_of: &str,
    model: &str,
    metrics: Vec<String>,
) -> PyResult<String> {
    let inst: finstack_valuations::instruments::InstrumentJson =
        serde_json::from_str(instrument_json).map_err(val_to_py)?;
    let boxed = inst.into_boxed().map_err(val_to_py)?;

    let market = extract_market(market)?;

    let date = super::parse_date(as_of)?;
    let model_key = parse_model_key(model)?;
    let metric_ids: Vec<finstack_valuations::metrics::MetricId> = metrics
        .iter()
        .map(|m| finstack_valuations::metrics::MetricId::custom(m.as_str()))
        .collect();

    let registry = finstack_valuations::pricer::standard_registry();
    let result = registry
        .price_with_metrics(
            boxed.as_ref(),
            model_key,
            &market,
            date,
            &metric_ids,
            Default::default(),
        )
        .map_err(val_to_py)?;

    serde_json::to_string_pretty(&result).map_err(val_to_py)
}

/// List all metric IDs in the standard metric registry.
///
/// Returns
/// -------
/// list[str]
///     All registered metric identifiers (sorted alphabetically).
#[pyfunction]
fn list_standard_metrics() -> Vec<String> {
    finstack_valuations::metrics::standard_registry()
        .available_metrics()
        .into_iter()
        .map(|id| id.to_string())
        .collect()
}

/// List all standard metrics organized by group.
///
/// Returns a dict `{ group_name: [metric_id, ...], ... }` where each key
/// is a human-readable group name (e.g. "Pricing", "Greeks", "Sensitivity")
/// and the value is a sorted list of metric ID strings.
///
/// Returns
/// -------
/// dict[str, list[str]]
///     Metrics grouped by category.
#[pyfunction]
fn list_standard_metrics_grouped() -> std::collections::HashMap<String, Vec<String>> {
    finstack_valuations::metrics::standard_registry()
        .available_metrics_grouped()
        .into_iter()
        .map(|(group, metrics)| {
            (
                group.display_name().to_string(),
                metrics.into_iter().map(|m| m.to_string()).collect(),
            )
        })
        .collect()
}

fn parse_model_key(s: &str) -> PyResult<finstack_valuations::pricer::ModelKey> {
    s.parse::<finstack_valuations::pricer::ModelKey>()
        .map_err(|e| PyValueError::new_err(format!("Unknown model key: '{s}'. {e}")))
}

/// Register pricing functions on the valuations submodule.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(pyo3::wrap_pyfunction!(price_instrument, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(price_instrument_with_metrics, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(list_standard_metrics, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(list_standard_metrics_grouped, m)?)?;
    Ok(())
}
