//! WASM bindings for P&L attribution across multiple methodologies.

use crate::utils::to_js_err;
use wasm_bindgen::prelude::*;

/// Run P&L attribution for a single instrument.
///
/// Accepts the instrument JSON, two market snapshots, dates, and a
/// method descriptor.  Returns the `PnlAttribution` result as JSON.
#[wasm_bindgen(js_name = attributePnl)]
pub fn attribute_pnl(
    instrument_json: &str,
    market_t0_json: &str,
    market_t1_json: &str,
    as_of_t0: &str,
    as_of_t1: &str,
    method_json: &str,
    config_json: Option<String>,
) -> Result<String, JsValue> {
    let spec = build_attribution_spec(
        instrument_json,
        market_t0_json,
        market_t1_json,
        as_of_t0,
        as_of_t1,
        method_json,
        config_json.as_deref(),
    )?;
    let result = spec.execute().map_err(to_js_err)?;
    serde_json::to_string(&result.attribution).map_err(to_js_err)
}

/// Run attribution from a full JSON `AttributionEnvelope` and return JSON.
///
/// Power-user variant for full envelope round-trip workflows.
#[wasm_bindgen(js_name = attributePnlFromSpec)]
pub fn attribute_pnl_from_spec(spec_json: &str) -> Result<String, JsValue> {
    let envelope: finstack_valuations::attribution::AttributionEnvelope =
        serde_json::from_str(spec_json).map_err(to_js_err)?;
    let result_envelope = envelope.execute().map_err(to_js_err)?;
    serde_json::to_string(&result_envelope).map_err(to_js_err)
}

/// Validate an attribution specification JSON.
///
/// Deserializes against the `AttributionEnvelope` schema and returns
/// the canonical JSON.
#[wasm_bindgen(js_name = validateAttributionJson)]
pub fn validate_attribution_json(json: &str) -> Result<String, JsValue> {
    let envelope: finstack_valuations::attribution::AttributionEnvelope =
        serde_json::from_str(json).map_err(to_js_err)?;
    serde_json::to_string(&envelope).map_err(to_js_err)
}

/// Return the default waterfall factor ordering as a JSON array.
#[wasm_bindgen(js_name = defaultWaterfallOrder)]
pub fn default_waterfall_order() -> Result<JsValue, JsValue> {
    let factors: Vec<String> = finstack_valuations::attribution::default_waterfall_order()
        .into_iter()
        .map(|f| f.to_string())
        .collect();
    serde_wasm_bindgen::to_value(&factors).map_err(to_js_err)
}

/// Return the default metric IDs used by metrics-based attribution.
#[wasm_bindgen(js_name = defaultAttributionMetrics)]
pub fn default_attribution_metrics() -> Result<JsValue, JsValue> {
    let metrics: Vec<String> = finstack_valuations::attribution::default_attribution_metrics()
        .into_iter()
        .map(|m| m.to_string())
        .collect();
    serde_wasm_bindgen::to_value(&metrics).map_err(to_js_err)
}

fn build_attribution_spec(
    instrument_json: &str,
    market_t0_json: &str,
    market_t1_json: &str,
    as_of_t0: &str,
    as_of_t1: &str,
    method_json: &str,
    config_json: Option<&str>,
) -> Result<finstack_valuations::attribution::AttributionSpec, JsValue> {
    let instrument: finstack_valuations::instruments::InstrumentJson =
        serde_json::from_str(instrument_json).map_err(to_js_err)?;
    let market_t0: finstack_core::market_data::context::MarketContextState =
        serde_json::from_str(market_t0_json).map_err(to_js_err)?;
    let market_t1: finstack_core::market_data::context::MarketContextState =
        serde_json::from_str(market_t1_json).map_err(to_js_err)?;
    let format = time::format_description::well_known::Iso8601::DEFAULT;
    let t0 = time::Date::parse(as_of_t0, &format).map_err(to_js_err)?;
    let t1 = time::Date::parse(as_of_t1, &format).map_err(to_js_err)?;
    let method: finstack_valuations::attribution::AttributionMethod =
        serde_json::from_str(method_json).map_err(to_js_err)?;
    let config: Option<finstack_valuations::attribution::AttributionConfig> = match config_json {
        Some(json) => Some(serde_json::from_str(json).map_err(to_js_err)?),
        None => None,
    };
    Ok(finstack_valuations::attribution::AttributionSpec {
        instrument,
        market_t0,
        market_t1,
        as_of_t0: t0,
        as_of_t1: t1,
        method,
        model_params_t0: None,
        config,
    })
}
