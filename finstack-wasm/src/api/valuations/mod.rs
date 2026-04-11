//! WASM bindings for the `finstack-valuations` crate.
//!
//! Exposes JSON round-trip for valuation results and instrument validation.

use crate::utils::to_js_err;
use wasm_bindgen::prelude::*;

/// Deserialize a `ValuationResult` from JSON and return the canonical JSON.
///
/// Validates the input conforms to the `ValuationResult` schema.
#[wasm_bindgen(js_name = validateValuationResultJson)]
pub fn validate_valuation_result_json(json: &str) -> Result<String, JsValue> {
    let result: finstack_valuations::results::ValuationResult =
        serde_json::from_str(json).map_err(to_js_err)?;
    serde_json::to_string(&result).map_err(to_js_err)
}

/// Validate a tagged instrument JSON string.
///
/// Deserializes the input against the known instrument schema and
/// returns the canonical (re-serialized) JSON.
#[wasm_bindgen(js_name = validateInstrumentJson)]
pub fn validate_instrument_json(json: &str) -> Result<String, JsValue> {
    let parsed: finstack_valuations::instruments::InstrumentJson =
        serde_json::from_str(json).map_err(to_js_err)?;
    serde_json::to_string(&parsed).map_err(to_js_err)
}

/// Price an instrument from its tagged JSON and return a ValuationResult JSON.
#[wasm_bindgen(js_name = priceInstrument)]
pub fn price_instrument(
    instrument_json: &str,
    market_json: &str,
    as_of: &str,
    model: &str,
) -> Result<String, JsValue> {
    let inst: finstack_valuations::instruments::InstrumentJson =
        serde_json::from_str(instrument_json).map_err(to_js_err)?;
    let boxed = inst.into_boxed().map_err(to_js_err)?;
    let market: finstack_core::market_data::context::MarketContext =
        serde_json::from_str(market_json).map_err(to_js_err)?;
    let format = time::format_description::well_known::Iso8601::DEFAULT;
    let date = time::Date::parse(as_of, &format).map_err(to_js_err)?;
    let model_key = parse_model_key(model)?;
    let registry = finstack_valuations::pricer::standard_registry();
    let result = registry
        .price(boxed.as_ref(), model_key, &market, date, None)
        .map_err(to_js_err)?;
    serde_json::to_string_pretty(&result).map_err(to_js_err)
}

/// Price an instrument with explicit metric requests.
#[wasm_bindgen(js_name = priceInstrumentWithMetrics)]
pub fn price_instrument_with_metrics(
    instrument_json: &str,
    market_json: &str,
    as_of: &str,
    model: &str,
    metrics: JsValue,
) -> Result<String, JsValue> {
    let inst: finstack_valuations::instruments::InstrumentJson =
        serde_json::from_str(instrument_json).map_err(to_js_err)?;
    let boxed = inst.into_boxed().map_err(to_js_err)?;
    let market: finstack_core::market_data::context::MarketContext =
        serde_json::from_str(market_json).map_err(to_js_err)?;
    let format = time::format_description::well_known::Iso8601::DEFAULT;
    let date = time::Date::parse(as_of, &format).map_err(to_js_err)?;
    let model_key = parse_model_key(model)?;
    let metric_strs: Vec<String> = serde_wasm_bindgen::from_value(metrics).map_err(to_js_err)?;
    let metric_ids: Vec<finstack_valuations::metrics::MetricId> = metric_strs
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
        .map_err(to_js_err)?;
    serde_json::to_string_pretty(&result).map_err(to_js_err)
}

/// List all metric IDs in the standard metric registry.
#[wasm_bindgen(js_name = listStandardMetrics)]
pub fn list_standard_metrics() -> Result<JsValue, JsValue> {
    let ids: Vec<String> = finstack_valuations::metrics::standard_registry()
        .available_metrics()
        .into_iter()
        .map(|id| id.to_string())
        .collect();
    serde_wasm_bindgen::to_value(&ids).map_err(to_js_err)
}

fn parse_model_key(s: &str) -> Result<finstack_valuations::pricer::ModelKey, JsValue> {
    use finstack_valuations::pricer::ModelKey;
    match s {
        "discounting" => Ok(ModelKey::Discounting),
        "tree" => Ok(ModelKey::Tree),
        "black76" => Ok(ModelKey::Black76),
        "hull_white_1f" => Ok(ModelKey::HullWhite1F),
        "hazard_rate" => Ok(ModelKey::HazardRate),
        "normal" => Ok(ModelKey::Normal),
        "monte_carlo_gbm" => Ok(ModelKey::MonteCarloGBM),
        other => Err(to_js_err(format!(
            "Unknown model key: '{other}'. Use one of: discounting, tree, black76, hull_white_1f, hazard_rate, normal, monte_carlo_gbm"
        ))),
    }
}
