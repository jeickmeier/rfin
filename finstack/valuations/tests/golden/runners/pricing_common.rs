//! Shared pricing runner helpers for instrument-level golden fixtures.

use crate::golden::runner::DomainRunner;
use crate::golden::schema::GoldenFixture;
use finstack_core::market_data::context::MarketContext;
use finstack_valuations::pricer::price_instrument_json_with_metrics;
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Deserialize)]
struct PricingInputs {
    valuation_date: String,
    model: String,
    metrics: Vec<String>,
    instrument_json: serde_json::Value,
    market: MarketContext,
}

/// Shared runner for fixture domains that use the common pricing input contract.
pub struct PricingRunner;

impl DomainRunner for PricingRunner {
    fn run(&self, fixture: &GoldenFixture) -> Result<BTreeMap<String, f64>, String> {
        run_pricing_fixture(fixture)
    }
}

/// Price an instrument fixture that follows the common pricing input contract.
pub(crate) fn run_pricing_fixture(
    fixture: &GoldenFixture,
) -> Result<BTreeMap<String, f64>, String> {
    crate::golden::runners::validate_source_validation_fixture("pricing runner", fixture)?;

    let inputs: PricingInputs = serde_json::from_value(fixture.inputs.clone())
        .map_err(|err| format!("parse pricing inputs: {err}"))?;
    let instrument_json = serde_json::to_string(&inputs.instrument_json)
        .map_err(|err| format!("serialize instrument_json: {err}"))?;

    let result = price_instrument_json_with_metrics(
        &instrument_json,
        &inputs.market,
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
