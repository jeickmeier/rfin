//! Shared pricing runner helpers for instrument-level golden fixtures.

use crate::golden::schema::GoldenFixture;
use finstack_core::currency::Currency;
use finstack_core::dates::DayCount;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::surfaces::VolSurface;
use finstack_core::market_data::term_structures::{DiscountCurve, ForwardCurve};
use finstack_core::math::interp::InterpStyle;
use finstack_core::money::fx::{FxMatrix, SimpleFxProvider};
use finstack_valuations::pricer::{parse_as_of_date, price_instrument_json_with_metrics};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::str::FromStr;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
struct PricingInputs {
    valuation_date: String,
    model: String,
    metrics: Vec<String>,
    instrument_json: serde_json::Value,
    curves: CurveInputs,
    #[serde(default)]
    fx: Vec<FxQuoteSpec>,
    #[serde(default)]
    surfaces: SurfaceInputs,
}

#[derive(Debug, Deserialize)]
struct CurveInputs {
    discount: Vec<DiscountCurveSpec>,
    #[serde(default)]
    forward: Vec<ForwardCurveSpec>,
}

#[derive(Debug, Default, Deserialize)]
struct SurfaceInputs {
    #[serde(default)]
    vol: Vec<VolSurfaceSpec>,
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

#[derive(Debug, Deserialize)]
struct FxQuoteSpec {
    base: String,
    quote: String,
    rate: f64,
}

#[derive(Debug, Deserialize)]
struct VolSurfaceSpec {
    id: String,
    expiries: Vec<f64>,
    strikes: Vec<f64>,
    vols_row_major: Vec<f64>,
}

/// Price an instrument fixture that follows the common pricing input contract.
pub(crate) fn run_pricing_fixture(
    fixture: &GoldenFixture,
) -> Result<BTreeMap<String, f64>, String> {
    let inputs: PricingInputs = serde_json::from_value(fixture.inputs.clone())
        .map_err(|err| format!("parse pricing inputs: {err}"))?;
    let market = build_market(&inputs.curves, &inputs.fx, &inputs.surfaces)?;
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

fn build_market(
    curves: &CurveInputs,
    fx_quotes: &[FxQuoteSpec],
    surfaces: &SurfaceInputs,
) -> Result<MarketContext, String> {
    let mut market = MarketContext::new();
    for curve in &curves.discount {
        market = market.insert(build_discount_curve(curve)?);
    }
    for curve in &curves.forward {
        market = market.insert(build_forward_curve(curve)?);
    }
    for surface in &surfaces.vol {
        market = market.insert_surface(build_vol_surface(surface)?);
    }
    if !fx_quotes.is_empty() {
        market = market.insert_fx(build_fx_matrix(fx_quotes)?);
    }
    Ok(market)
}

fn build_vol_surface(spec: &VolSurfaceSpec) -> Result<VolSurface, String> {
    VolSurface::from_grid(
        spec.id.as_str(),
        &spec.expiries,
        &spec.strikes,
        &spec.vols_row_major,
    )
    .map_err(|err| format!("build vol surface '{}': {err}", spec.id))
}

fn build_fx_matrix(quotes: &[FxQuoteSpec]) -> Result<FxMatrix, String> {
    let provider = Arc::new(SimpleFxProvider::new());
    for quote in quotes {
        provider
            .set_quote(
                parse_currency(&quote.base)?,
                parse_currency(&quote.quote)?,
                quote.rate,
            )
            .map_err(|err| format!("set FX quote {}/{}: {err}", quote.base, quote.quote))?;
    }
    Ok(FxMatrix::new(provider))
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

fn parse_currency(raw: &str) -> Result<Currency, String> {
    Currency::from_str(raw).map_err(|err| format!("invalid currency '{raw}': {err}"))
}

fn parse_interp(raw: Option<&str>) -> Result<InterpStyle, String> {
    raw.unwrap_or("linear")
        .parse::<InterpStyle>()
        .map_err(|err| format!("invalid curve interpolation: {err}"))
}
