//! Shared pricing runner helpers for instrument-level golden fixtures.

use crate::golden::schema::GoldenFixture;
use finstack_core::market_data::context::MarketContext;
use finstack_valuations::calibration::api::engine;
use finstack_valuations::calibration::api::schema::CalibrationEnvelope;
use finstack_valuations::pricer::price_instrument_json_with_metrics;
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Deserialize)]
struct PricingInputs {
    valuation_date: String,
    model: String,
    metrics: Vec<String>,
    instrument_json: serde_json::Value,
    /// Materialized MarketContext JSON (snapshot form). Mutually exclusive with `market_envelope`.
    #[serde(default)]
    market: Option<MarketContext>,
    /// CalibrationEnvelope JSON (quote-driven form). Mutually exclusive with `market`.
    #[serde(default)]
    market_envelope: Option<CalibrationEnvelope>,
}

impl PricingInputs {
    fn resolve_market(&self) -> Result<MarketContext, String> {
        match (&self.market, &self.market_envelope) {
            (Some(_), Some(_)) => Err(
                "pricing fixture supplied both 'market' and 'market_envelope'; specify exactly one"
                    .to_string(),
            ),
            (Some(m), None) => Ok(m.clone()),
            (None, Some(env)) => {
                let result = engine::execute(env)
                    .map_err(|err| format!("calibrate market_envelope: {err}"))?;
                MarketContext::try_from(result.result.final_market.clone())
                    .map_err(|err| format!("rehydrate calibrated market: {err}"))
            }
            (None, None) => {
                Err("pricing fixture must supply either 'market' or 'market_envelope'".to_string())
            }
        }
    }
}

/// Price an instrument fixture that follows the common pricing input contract.
pub(crate) fn run_pricing_fixture(
    fixture: &GoldenFixture,
) -> Result<BTreeMap<String, f64>, String> {
    crate::golden::source_validation::validate_source_validation_fixture(
        "pricing runner",
        fixture,
    )?;

    let inputs: PricingInputs = serde_json::from_value(fixture.inputs.clone())
        .map_err(|err| format!("parse pricing inputs: {err}"))?;
    let market = inputs.resolve_market()?;
    let instrument_json = serde_json::to_string(&inputs.instrument_json)
        .map_err(|err| format!("serialize instrument_json: {err}"))?;

    let result = price_instrument_json_with_metrics(
        &instrument_json,
        &market,
        &inputs.valuation_date,
        &inputs.model,
        &inputs.metrics,
        None,
    )
    .map_err(|err| format!("price instrument JSON: {err}"))?;

    let mut actuals = BTreeMap::new();
    for metric in fixture.expected_outputs.keys() {
        let value = if metric == "npv" {
            result.value.amount()
        } else {
            *result
                .measures
                .get(metric.as_str())
                .ok_or_else(|| format!("result missing metric '{metric}'"))?
        };
        actuals.insert(metric.clone(), value);
    }
    Ok(actuals)
}

#[cfg(test)]
mod market_envelope_input_tests {
    use super::*;
    use serde_json::json;

    /// Minimal valid MarketContext JSON (empty curves, no surfaces).
    fn minimal_market() -> serde_json::Value {
        json!({
            "version": 2,
            "curves": [],
            "fx": null,
            "surfaces": [],
            "prices": {},
            "series": [],
            "inflation_indices": [],
            "dividends": [],
            "credit_indices": [],
            "fx_delta_vol_surfaces": [],
            "vol_cubes": [],
            "collateral": {}
        })
    }

    /// Minimal valid CalibrationEnvelope JSON (no steps, no initial market).
    fn minimal_envelope() -> serde_json::Value {
        json!({
            "schema": "finstack.calibration",
            "plan": {
                "id": "test_envelope",
                "quote_sets": {},
                "steps": [],
                "settings": {}
            }
        })
    }

    #[test]
    fn pricing_inputs_reject_when_both_market_and_market_envelope_supplied() {
        let inputs = json!({
            "valuation_date": "2026-04-30",
            "model": "discounting",
            "metrics": [],
            "instrument_json": {},
            "market": minimal_market(),
            "market_envelope": minimal_envelope()
        });
        // Either deserialization fails, OR PricingInputs::resolve_market returns the
        // expected mutually-exclusive error. Both are acceptable as long as the
        // runtime end result is a clear rejection mentioning both field names.
        let result: Result<PricingInputs, _> = serde_json::from_value(inputs);
        match result {
            Err(err) => {
                let msg = err.to_string();
                assert!(
                    msg.contains("market"),
                    "deserialize error should mention 'market', got: {msg}"
                );
            }
            Ok(parsed) => {
                let err = parsed
                    .resolve_market()
                    .err()
                    .expect("must reject fixtures that supply both 'market' and 'market_envelope'");
                assert!(
                    err.contains("market") && err.contains("market_envelope"),
                    "resolve_market error should name both fields, got: {err}"
                );
            }
        }
    }
}
