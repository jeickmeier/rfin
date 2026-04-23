//! WASM bindings for instrument pricing and metric introspection.

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
    finstack_valuations::pricer::validate_instrument_json(json).map_err(to_js_err)
}

/// Price an instrument from its tagged JSON and return a ValuationResult JSON.
#[wasm_bindgen(js_name = priceInstrument)]
pub fn price_instrument(
    instrument_json: &str,
    market_json: &str,
    as_of: &str,
    model: &str,
) -> Result<String, JsValue> {
    let market: finstack_core::market_data::context::MarketContext =
        serde_json::from_str(market_json).map_err(to_js_err)?;
    let result =
        finstack_valuations::pricer::price_instrument_json(instrument_json, &market, as_of, model)
            .map_err(to_js_err)?;
    serde_json::to_string(&result).map_err(to_js_err)
}

/// Price an instrument with explicit metric requests.
#[wasm_bindgen(js_name = priceInstrumentWithMetrics)]
pub fn price_instrument_with_metrics(
    instrument_json: &str,
    market_json: &str,
    as_of: &str,
    model: &str,
    metrics: JsValue,
    pricing_options: Option<String>,
) -> Result<String, JsValue> {
    let market: finstack_core::market_data::context::MarketContext =
        serde_json::from_str(market_json).map_err(to_js_err)?;
    let metric_strs: Vec<String> = serde_wasm_bindgen::from_value(metrics).map_err(to_js_err)?;
    let result = finstack_valuations::pricer::price_instrument_json_with_metrics(
        instrument_json,
        &market,
        as_of,
        model,
        &metric_strs,
        pricing_options.as_deref(),
    )
    .map_err(to_js_err)?;
    serde_json::to_string(&result).map_err(to_js_err)
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

/// List all standard metrics organized by group.
///
/// Returns a JSON object `{ group_name: [metric_id, ...], ... }` where
/// each key is a human-readable group name (e.g. "Pricing", "Greeks",
/// "Sensitivity") and the value is a sorted array of metric ID strings.
#[wasm_bindgen(js_name = listStandardMetricsGrouped)]
pub fn list_standard_metrics_grouped() -> Result<JsValue, JsValue> {
    let grouped: Vec<(String, Vec<String>)> = finstack_valuations::metrics::standard_registry()
        .available_metrics_grouped()
        .into_iter()
        .map(|(group, metrics)| {
            (
                group.display_name().to_string(),
                metrics.into_iter().map(|m| m.to_string()).collect(),
            )
        })
        .collect();
    let map: std::collections::BTreeMap<String, Vec<String>> = grouped.into_iter().collect();
    serde_wasm_bindgen::to_value(&map).map_err(to_js_err)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_model_key_recognizes_standard_keys() {
        assert_eq!(
            finstack_valuations::pricer::parse_model_key("discounting").expect("ok"),
            finstack_valuations::pricer::ModelKey::Discounting
        );
        assert_eq!(
            finstack_valuations::pricer::parse_model_key("tree").expect("ok"),
            finstack_valuations::pricer::ModelKey::Tree
        );
        assert_eq!(
            finstack_valuations::pricer::parse_model_key("black76").expect("ok"),
            finstack_valuations::pricer::ModelKey::Black76
        );
        assert_eq!(
            finstack_valuations::pricer::parse_model_key("hull_white_1f").expect("ok"),
            finstack_valuations::pricer::ModelKey::HullWhite1F
        );
        assert_eq!(
            finstack_valuations::pricer::parse_model_key("hazard_rate").expect("ok"),
            finstack_valuations::pricer::ModelKey::HazardRate
        );
        assert_eq!(
            finstack_valuations::pricer::parse_model_key("normal").expect("ok"),
            finstack_valuations::pricer::ModelKey::Normal
        );
        assert_eq!(
            finstack_valuations::pricer::parse_model_key("monte_carlo_gbm").expect("ok"),
            finstack_valuations::pricer::ModelKey::MonteCarloGBM
        );
    }

    pub(crate) fn bond_instrument_json() -> String {
        use finstack_core::currency::Currency;
        use finstack_core::money::Money;
        use finstack_valuations::instruments::fixed_income::bond::Bond;
        use finstack_valuations::instruments::InstrumentJson;

        let bond = Bond::fixed(
            "TEST-BOND",
            Money::new(1_000_000.0, Currency::USD),
            0.05,
            time::Date::from_calendar_date(2024, time::Month::January, 1).expect("date"),
            time::Date::from_calendar_date(2034, time::Month::January, 1).expect("date"),
            "USD-OIS",
        )
        .expect("bond");
        serde_json::to_string(&InstrumentJson::Bond(bond)).expect("serialize")
    }

    pub(crate) fn market_context_json() -> String {
        use finstack_core::market_data::context::MarketContext;
        use finstack_core::market_data::term_structures::DiscountCurve;
        let base = time::Date::from_calendar_date(2024, time::Month::January, 1).expect("date");
        let disc = DiscountCurve::builder("USD-OIS")
            .base_date(base)
            .knots([(0.5, 0.99), (1.0, 0.98), (5.0, 0.90), (10.0, 0.80)])
            .build()
            .expect("curve");
        let ctx = MarketContext::new().insert(disc);
        serde_json::to_string(&ctx).expect("serialize")
    }

    #[test]
    fn validate_instrument_json_bond() {
        let json = bond_instrument_json();
        let canonical = validate_instrument_json(&json).expect("validate");
        assert!(!canonical.is_empty());
    }

    #[test]
    fn price_instrument_bond() {
        let inst = bond_instrument_json();
        let mkt = market_context_json();
        let result = price_instrument(&inst, &mkt, "2024-01-01", "discounting").expect("price");
        let parsed: serde_json::Value = serde_json::from_str(&result).expect("json");
        assert!(parsed.is_object());
    }

    #[test]
    fn validate_valuation_result_json_roundtrip() {
        let inst = bond_instrument_json();
        let mkt = market_context_json();
        let result_json =
            price_instrument(&inst, &mkt, "2024-01-01", "discounting").expect("price");
        let canonical = validate_valuation_result_json(&result_json).expect("validate");
        assert!(!canonical.is_empty());
        let parsed: serde_json::Value = serde_json::from_str(&canonical).expect("json");
        assert!(parsed.is_object());
    }
}
