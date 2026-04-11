//! Instrument pricing pipeline: JSON instrument + market → ValuationResult.

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
/// market_json : str
///     JSON-serialized ``MarketContext``.
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
#[pyo3(signature = (instrument_json, market_json, as_of, model="discounting"))]
fn price_instrument(
    instrument_json: &str,
    market_json: &str,
    as_of: &str,
    model: &str,
) -> PyResult<String> {
    let inst: finstack_valuations::instruments::InstrumentJson =
        serde_json::from_str(instrument_json).map_err(val_to_py)?;
    let boxed = inst.into_boxed().map_err(val_to_py)?;

    let market: finstack_core::market_data::context::MarketContext =
        serde_json::from_str(market_json).map_err(val_to_py)?;

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
/// market_json : str
///     JSON-serialized ``MarketContext``.
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
#[pyo3(signature = (instrument_json, market_json, as_of, model="discounting", metrics=vec![]))]
fn price_instrument_with_metrics(
    instrument_json: &str,
    market_json: &str,
    as_of: &str,
    model: &str,
    metrics: Vec<String>,
) -> PyResult<String> {
    let inst: finstack_valuations::instruments::InstrumentJson =
        serde_json::from_str(instrument_json).map_err(val_to_py)?;
    let boxed = inst.into_boxed().map_err(val_to_py)?;

    let market: finstack_core::market_data::context::MarketContext =
        serde_json::from_str(market_json).map_err(val_to_py)?;

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

fn parse_model_key(s: &str) -> PyResult<finstack_valuations::pricer::ModelKey> {
    use finstack_valuations::pricer::ModelKey;
    match s {
        "discounting" => Ok(ModelKey::Discounting),
        "hazard_rate" => Ok(ModelKey::HazardRate),        
        "tree" => Ok(ModelKey::Tree),
        "black76" => Ok(ModelKey::Black76),
        "hull_white_1f" => Ok(ModelKey::HullWhite1F),
        "normal" => Ok(ModelKey::Normal),
        "monte_carlo_gbm" => Ok(ModelKey::MonteCarloGBM),
        other => Err(PyValueError::new_err(format!(
            "Unknown model key: '{other}'. Use one of: discounting, tree, black76, hull_white_1f, hazard_rate, normal, monte_carlo_gbm"
        ))),
    }
}

/// Register pricing functions on the valuations submodule.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(pyo3::wrap_pyfunction!(price_instrument, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(price_instrument_with_metrics, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(list_standard_metrics, m)?)?;
    Ok(())
}
