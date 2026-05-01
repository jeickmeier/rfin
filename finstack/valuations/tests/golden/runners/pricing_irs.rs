//! Domain runner for `rates.irs` golden fixtures.

use crate::golden::runner::DomainRunner;
use crate::golden::schema::GoldenFixture;
use finstack_core::dates::DayCount;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::{DiscountCurve, ForwardCurve};
use finstack_core::math::interp::InterpStyle;
use finstack_valuations::pricer::{parse_as_of_date, price_instrument_json_with_metrics};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::str::FromStr;

#[derive(Debug, Deserialize)]
struct IrsInputs {
    valuation_date: String,
    model: String,
    metrics: Vec<String>,
    instrument_json: serde_json::Value,
    curves: CurveInputs,
}

#[derive(Debug, Deserialize)]
struct CurveInputs {
    discount: Vec<DiscountCurveSpec>,
    #[serde(default)]
    forward: Vec<ForwardCurveSpec>,
}

#[derive(Debug, Deserialize)]
struct DiscountCurveSpec {
    id: String,
    base_date: String,
    day_count: Option<String>,
    interp: Option<String>,
    knots: Vec<[f64; 2]>,
}

#[derive(Debug, Deserialize)]
struct ForwardCurveSpec {
    id: String,
    tenor: f64,
    base_date: String,
    day_count: Option<String>,
    interp: Option<String>,
    knots: Vec<[f64; 2]>,
}

/// Interest rate swap golden runner skeleton.
pub struct IrsRunner;

impl DomainRunner for IrsRunner {
    fn run(&self, fixture: &GoldenFixture) -> Result<BTreeMap<String, f64>, String> {
        let inputs: IrsInputs = serde_json::from_value(fixture.inputs.clone())
            .map_err(|err| format!("parse IRS inputs: {err}"))?;
        let market = build_market(&inputs.curves)?;
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
}

fn build_market(curves: &CurveInputs) -> Result<MarketContext, String> {
    let mut market = MarketContext::new();
    for curve in &curves.discount {
        market = market.insert(build_discount_curve(curve)?);
    }
    for curve in &curves.forward {
        market = market.insert(build_forward_curve(curve)?);
    }
    Ok(market)
}

fn build_discount_curve(spec: &DiscountCurveSpec) -> Result<DiscountCurve, String> {
    let mut builder = DiscountCurve::builder(spec.id.as_str())
        .base_date(parse_as_of_date(&spec.base_date).map_err(|err| err.to_string())?)
        .knots(to_knots(&spec.knots))
        .interp(parse_interp(spec.interp.as_deref())?);
    if let Some(day_count) = spec.day_count.as_deref() {
        builder = builder.day_count(parse_day_count(day_count)?);
    }
    builder
        .build()
        .map_err(|err| format!("build discount curve '{}': {err}", spec.id))
}

fn build_forward_curve(spec: &ForwardCurveSpec) -> Result<ForwardCurve, String> {
    let mut builder = ForwardCurve::builder(spec.id.as_str(), spec.tenor)
        .base_date(parse_as_of_date(&spec.base_date).map_err(|err| err.to_string())?)
        .knots(to_knots(&spec.knots))
        .interp(parse_interp(spec.interp.as_deref())?);
    if let Some(day_count) = spec.day_count.as_deref() {
        builder = builder.day_count(parse_day_count(day_count)?);
    }
    builder
        .build()
        .map_err(|err| format!("build forward curve '{}': {err}", spec.id))
}

fn to_knots(knots: &[[f64; 2]]) -> Vec<(f64, f64)> {
    knots.iter().map(|knot| (knot[0], knot[1])).collect()
}

fn parse_day_count(raw: &str) -> Result<DayCount, String> {
    DayCount::from_str(raw).map_err(|err| format!("invalid day_count '{raw}': {err}"))
}

fn parse_interp(raw: Option<&str>) -> Result<InterpStyle, String> {
    raw.unwrap_or("linear")
        .parse::<InterpStyle>()
        .map_err(|err| format!("invalid curve interpolation: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_bloomberg_swpm_fixture() {
        let fixture: GoldenFixture = serde_json::from_str(include_str!(
            "../data/pricing/irs/usd_sofr_5y_receive_fixed_swpm.json"
        ))
        .expect("fixture parses");

        let actuals = IrsRunner.run(&fixture).expect("runner prices fixture");

        assert!(actuals.contains_key("npv"));
        assert!(actuals.contains_key("par_rate"));
        assert!(actuals.contains_key("dv01"));
    }
}
