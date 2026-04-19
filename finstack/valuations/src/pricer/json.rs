//! Shared JSON pricing helpers for host-language bindings.
//!
//! This module centralizes the tagged-instrument JSON pipeline used by the
//! Python and WASM bindings: parse instrument JSON, optionally merge metric
//! pricing overrides, parse the as-of date and model key, and dispatch through
//! the standard pricer registry.

use super::{standard_registry, ModelKey};
use crate::instruments::{Instrument, InstrumentJson, MetricPricingOverrides};
use crate::metrics::MetricId;
use crate::results::ValuationResult;
use finstack_core::market_data::context::MarketContext;
use finstack_core::Error;
use serde_json::{Map, Value};
use std::borrow::Cow;

/// Parse tagged instrument JSON into the canonical Rust enum.
pub fn parse_instrument_json(json: &str) -> finstack_core::Result<InstrumentJson> {
    serde_json::from_str(json)
        .map_err(|e| Error::Validation(format!("invalid instrument JSON: {e}")))
}

/// Parse tagged instrument JSON, optionally merge metric pricing overrides, and
/// box the concrete instrument for pricing dispatch.
pub fn parse_boxed_instrument_json(
    instrument_json: &str,
    pricing_options: Option<&str>,
) -> finstack_core::Result<Box<dyn Instrument>> {
    let effective_json = instrument_json_for_pricing(instrument_json, pricing_options)?;
    let parsed = parse_instrument_json(effective_json.as_ref())?;
    parsed.into_boxed()
}

/// Parse an ISO 8601 as-of date for JSON pricing helpers.
pub fn parse_as_of_date(as_of: &str) -> finstack_core::Result<time::Date> {
    let format = time::format_description::well_known::Iso8601::DEFAULT;
    time::Date::parse(as_of, &format)
        .map_err(|e| Error::Validation(format!("Invalid date '{as_of}': {e}")))
}

/// Parse a string model key used by the JSON pricing helpers.
pub fn parse_model_key(model: &str) -> finstack_core::Result<ModelKey> {
    model
        .parse::<ModelKey>()
        .map_err(|e| Error::Validation(format!("Unknown model key: '{model}'. {e}")))
}

/// Price a tagged instrument JSON payload using the shared standard registry.
pub fn price_instrument_json(
    instrument_json: &str,
    market: &MarketContext,
    as_of: &str,
    model: &str,
) -> finstack_core::Result<ValuationResult> {
    let instrument = parse_boxed_instrument_json(instrument_json, None)?;
    let as_of = parse_as_of_date(as_of)?;
    let model = parse_model_key(model)?;
    standard_registry()
        .price(instrument.as_ref(), model, market, as_of, None)
        .map_err(Into::into)
}

/// Price a tagged instrument JSON payload with explicit metric requests.
pub fn price_instrument_json_with_metrics(
    instrument_json: &str,
    market: &MarketContext,
    as_of: &str,
    model: &str,
    metrics: &[String],
    pricing_options: Option<&str>,
) -> finstack_core::Result<ValuationResult> {
    let instrument = parse_boxed_instrument_json(instrument_json, pricing_options)?;
    let as_of = parse_as_of_date(as_of)?;
    let model = parse_model_key(model)?;
    let metric_ids: Vec<MetricId> = metrics.iter().map(MetricId::custom).collect();
    standard_registry()
        .price_with_metrics(
            instrument.as_ref(),
            model,
            market,
            as_of,
            &metric_ids,
            Default::default(),
        )
        .map_err(Into::into)
}

fn instrument_json_for_pricing<'a>(
    instrument_json: &'a str,
    pricing_options: Option<&str>,
) -> finstack_core::Result<Cow<'a, str>> {
    let Some(pricing_options_json) = pricing_options else {
        return Ok(Cow::Borrowed(instrument_json));
    };

    let pricing_options: MetricPricingOverrides = serde_json::from_str(pricing_options_json)
        .map_err(|e| Error::Validation(format!("invalid pricing options JSON: {e}")))?;
    let mut document: Value = serde_json::from_str(instrument_json)
        .map_err(|e| Error::Validation(format!("invalid instrument JSON: {e}")))?;
    let pricing_patch = serde_json::to_value(&pricing_options)
        .map_err(|e| Error::Validation(format!("invalid pricing options JSON: {e}")))?;

    let patch = pricing_patch.as_object().cloned().ok_or_else(|| {
        Error::Validation("metric pricing overrides must serialize to an object".to_string())
    })?;
    let spec = document
        .get_mut("spec")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| Error::Validation("instrument JSON must contain an object spec".into()))?;
    let pricing_overrides = spec
        .entry("pricing_overrides".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    let pricing_overrides = pricing_overrides.as_object_mut().ok_or_else(|| {
        Error::Validation("instrument spec.pricing_overrides must be an object".to_string())
    })?;
    pricing_overrides.extend(patch);

    serde_json::to_string(&document)
        .map(Cow::Owned)
        .map_err(|e| Error::Validation(format!("invalid instrument JSON: {e}")))
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::instruments::fixed_income::bond::Bond;
    use finstack_core::currency::Currency;
    use finstack_core::market_data::term_structures::DiscountCurve;
    use finstack_core::money::Money;

    fn bond_instrument_json() -> String {
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

    fn market_context() -> MarketContext {
        let base = time::Date::from_calendar_date(2024, time::Month::January, 1).expect("date");
        let disc = DiscountCurve::builder("USD-OIS")
            .base_date(base)
            .knots([(0.5, 0.99), (1.0, 0.98), (5.0, 0.90), (10.0, 0.80)])
            .build()
            .expect("curve");
        MarketContext::new().insert(disc)
    }

    #[test]
    fn instrument_json_for_pricing_merges_metric_overrides() {
        let json = bond_instrument_json();
        let merged = instrument_json_for_pricing(
            &json,
            Some(
                r#"{"theta_period":"1D","breakeven_config":{"target":"z_spread","mode":"linear"}}"#,
            ),
        )
        .expect("merge");
        let parsed: Value = serde_json::from_str(merged.as_ref()).expect("json");
        assert_eq!(parsed["spec"]["pricing_overrides"]["theta_period"], "1D");
        assert_eq!(
            parsed["spec"]["pricing_overrides"]["breakeven_config"]["target"],
            "z_spread"
        );
    }

    #[test]
    fn parse_model_key_recognizes_standard_keys() {
        assert_eq!(
            parse_model_key("discounting").expect("ok"),
            ModelKey::Discounting
        );
        assert_eq!(parse_model_key("tree").expect("ok"), ModelKey::Tree);
        assert_eq!(parse_model_key("black76").expect("ok"), ModelKey::Black76);
        assert_eq!(
            parse_model_key("hull_white_1f").expect("ok"),
            ModelKey::HullWhite1F
        );
        assert_eq!(
            parse_model_key("hazard_rate").expect("ok"),
            ModelKey::HazardRate
        );
        assert_eq!(parse_model_key("normal").expect("ok"), ModelKey::Normal);
        assert_eq!(
            parse_model_key("monte_carlo_gbm").expect("ok"),
            ModelKey::MonteCarloGBM
        );
    }

    #[test]
    fn price_instrument_json_prices_bond() {
        let result = price_instrument_json(
            &bond_instrument_json(),
            &market_context(),
            "2024-01-01",
            "discounting",
        )
        .expect("price");
        assert_eq!(result.instrument_id, "TEST-BOND");
    }

    #[test]
    fn price_instrument_json_with_metrics_accepts_pricing_options() {
        let result = price_instrument_json_with_metrics(
            &bond_instrument_json(),
            &market_context(),
            "2024-01-01",
            "discounting",
            &["dirty_price".to_string()],
            Some(r#"{"theta_period":"1D"}"#),
        )
        .expect("price");
        assert_eq!(result.instrument_id, "TEST-BOND");
    }
}
