//! Shared pricing runner helpers for instrument-level golden fixtures.

use crate::golden::schema::GoldenFixture;
use finstack_core::currency::Currency;
use finstack_core::dates::DayCount;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::scalars::MarketScalar;
use finstack_core::market_data::surfaces::VolSurface;
use finstack_core::market_data::term_structures::{
    BaseCorrelationCurve, CreditIndexData, DiscountCurve, ForwardCurve, HazardCurve, InflationCurve,
};
use finstack_core::math::interp::InterpStyle;
use finstack_core::money::fx::{FxMatrix, SimpleFxProvider};
use finstack_core::money::Money;
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
    prices: Vec<MarketScalarSpec>,
    #[serde(default)]
    credit_indices: Vec<CreditIndexSpec>,
    #[serde(default)]
    surfaces: SurfaceInputs,
}

#[derive(Debug, Deserialize)]
struct CurveInputs {
    discount: Vec<DiscountCurveSpec>,
    #[serde(default)]
    forward: Vec<ForwardCurveSpec>,
    #[serde(default)]
    hazard: Vec<HazardCurveSpec>,
    #[serde(default)]
    inflation: Vec<InflationCurveSpec>,
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
struct HazardCurveSpec {
    id: String,
    base_date: String,
    recovery_rate: Option<f64>,
    day_count: Option<String>,
    knots: Vec<[f64; 2]>,
}

#[derive(Debug, Deserialize)]
struct InflationCurveSpec {
    id: String,
    base_date: String,
    base_cpi: f64,
    day_count: Option<String>,
    indexation_lag_months: Option<u32>,
    interp: Option<String>,
    knots: Vec<[f64; 2]>,
}

#[derive(Debug, Deserialize)]
struct BaseCorrelationCurveSpec {
    id: String,
    knots: Vec<[f64; 2]>,
}

#[derive(Debug, Deserialize)]
struct CreditIndexSpec {
    id: String,
    num_constituents: u16,
    recovery_rate: f64,
    index_credit_curve_id: String,
    base_correlation_curve: BaseCorrelationCurveSpec,
}

#[derive(Debug, Deserialize)]
struct MarketScalarSpec {
    id: String,
    value: f64,
    currency: Option<String>,
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
    let market = build_market(
        &inputs.curves,
        &inputs.fx,
        &inputs.prices,
        &inputs.credit_indices,
        &inputs.surfaces,
    )?;
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
    prices: &[MarketScalarSpec],
    credit_indices: &[CreditIndexSpec],
    surfaces: &SurfaceInputs,
) -> Result<MarketContext, String> {
    let mut market = MarketContext::new();
    for curve in &curves.discount {
        market = market.insert(build_discount_curve(curve)?);
    }
    for curve in &curves.forward {
        market = market.insert(build_forward_curve(curve)?);
    }
    for curve in &curves.hazard {
        market = market.insert(build_hazard_curve(curve)?);
    }
    for curve in &curves.inflation {
        market = market.insert(build_inflation_curve(curve)?);
    }
    for surface in &surfaces.vol {
        market = market.insert_surface(build_vol_surface(surface)?);
    }
    for price in prices {
        market = market.insert_price(price.id.as_str(), build_market_scalar(price)?);
    }
    for credit_index in credit_indices {
        let data = build_credit_index(credit_index, &market)?;
        market = market.insert_credit_index(credit_index.id.as_str(), data);
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

fn build_hazard_curve(spec: &HazardCurveSpec) -> Result<HazardCurve, String> {
    let mut builder = HazardCurve::builder(spec.id.as_str())
        .base_date(parse_as_of_date(&spec.base_date).map_err(|err| err.to_string())?)
        .knots(to_knots(&spec.knots));
    if let Some(recovery_rate) = spec.recovery_rate {
        builder = builder.recovery_rate(recovery_rate);
    }
    if let Some(day_count) = spec.day_count.as_deref() {
        builder = builder.day_count(parse_day_count(day_count)?);
    }
    builder
        .build()
        .map_err(|err| format!("build hazard curve '{}': {err}", spec.id))
}

fn build_inflation_curve(spec: &InflationCurveSpec) -> Result<InflationCurve, String> {
    let mut builder = InflationCurve::builder(spec.id.as_str())
        .base_date(parse_as_of_date(&spec.base_date).map_err(|err| err.to_string())?)
        .base_cpi(spec.base_cpi)
        .knots(to_knots(&spec.knots))
        .interp(parse_interp(spec.interp.as_deref())?);
    if let Some(day_count) = spec.day_count.as_deref() {
        builder = builder.day_count(parse_day_count(day_count)?);
    }
    if let Some(lag) = spec.indexation_lag_months {
        builder = builder.indexation_lag_months(lag);
    }
    builder
        .build()
        .map_err(|err| format!("build inflation curve '{}': {err}", spec.id))
}

fn build_market_scalar(spec: &MarketScalarSpec) -> Result<MarketScalar, String> {
    if let Some(currency) = spec.currency.as_deref() {
        Ok(MarketScalar::Price(Money::new(
            spec.value,
            parse_currency(currency)?,
        )))
    } else {
        Ok(MarketScalar::Unitless(spec.value))
    }
}

fn build_base_correlation_curve(
    spec: &BaseCorrelationCurveSpec,
) -> Result<BaseCorrelationCurve, String> {
    BaseCorrelationCurve::builder(spec.id.as_str())
        .knots(to_knots(&spec.knots))
        .build()
        .map_err(|err| format!("build base correlation curve '{}': {err}", spec.id))
}

fn build_credit_index(
    spec: &CreditIndexSpec,
    market: &MarketContext,
) -> Result<CreditIndexData, String> {
    let index_curve = market
        .get_hazard(spec.index_credit_curve_id.as_str())
        .map_err(|err| {
            format!(
                "get credit index hazard curve '{}': {err}",
                spec.index_credit_curve_id
            )
        })?;
    let base_correlation = build_base_correlation_curve(&spec.base_correlation_curve)?;
    CreditIndexData::builder()
        .num_constituents(spec.num_constituents)
        .recovery_rate(spec.recovery_rate)
        .index_credit_curve(index_curve)
        .base_correlation_curve(Arc::new(base_correlation))
        .build()
        .map_err(|err| format!("build credit index '{}': {err}", spec.id))
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
