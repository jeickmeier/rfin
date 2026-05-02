//! Shared executable calibration fixture helpers.

use crate::golden::schema::GoldenFixture;
use finstack_core::dates::DayCount;
use finstack_core::market_data::term_structures::{
    DiscountCurve, ForwardCurve, HazardCurve, InflationCurve,
};
use finstack_core::math::interp::InterpStyle;
use finstack_valuations::pricer::parse_as_of_date;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::str::FromStr;

#[derive(Debug, Deserialize)]
pub(crate) struct CurveCalibrationInputs {
    #[serde(default)]
    discount: Vec<DiscountCurveSpec>,
    #[serde(default)]
    forward: Vec<ForwardCurveSpec>,
    #[serde(default)]
    inflation: Vec<InflationCurveSpec>,
    #[serde(default)]
    probes: Vec<ProbeSpec>,
    #[serde(default)]
    calibration_rmse: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct HazardCalibrationInputs {
    hazard: Vec<HazardCurveSpec>,
    #[serde(default)]
    probes: Vec<ProbeSpec>,
    #[serde(default)]
    calibration_rmse: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct VolSmileInputs {
    smiles: Vec<VolSmileSpec>,
    #[serde(default)]
    repriced_rmse: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SabrCubeInputs {
    parameters: BTreeMap<String, f64>,
    #[serde(default)]
    calibration_rmse: Option<f64>,
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
struct InflationCurveSpec {
    id: String,
    base_date: String,
    base_cpi: f64,
    day_count: Option<String>,
    interp: Option<String>,
    indexation_lag_months: Option<u32>,
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
struct ProbeSpec {
    output: String,
    curve: String,
    kind: String,
    tenor: f64,
}

#[derive(Debug, Deserialize)]
struct VolSmileSpec {
    id: String,
    expiry: String,
    atm_vol: f64,
    wing_25d_put_vol: f64,
    wing_25d_call_vol: f64,
    wing_10d_put_vol: Option<f64>,
    wing_10d_call_vol: Option<f64>,
}

pub(crate) fn run_curve_fixture(fixture: &GoldenFixture) -> Result<BTreeMap<String, f64>, String> {
    let inputs: CurveCalibrationInputs = serde_json::from_value(fixture.inputs.clone())
        .map_err(|err| format!("parse curve calibration inputs: {err}"))?;
    let mut discount_curves = BTreeMap::new();
    for curve in &inputs.discount {
        discount_curves.insert(curve.id.clone(), build_discount_curve(curve)?);
    }
    let mut forward_curves = BTreeMap::new();
    for curve in &inputs.forward {
        forward_curves.insert(curve.id.clone(), build_forward_curve(curve)?);
    }
    let mut inflation_curves = BTreeMap::new();
    let mut inflation_base_cpi = BTreeMap::new();
    for curve in &inputs.inflation {
        inflation_base_cpi.insert(curve.id.clone(), curve.base_cpi);
        inflation_curves.insert(curve.id.clone(), build_inflation_curve(curve)?);
    }

    let mut actuals = BTreeMap::new();
    for probe in &inputs.probes {
        let value = match probe.kind.as_str() {
            "discount_factor" => discount_curves
                .get(&probe.curve)
                .ok_or_else(|| format!("missing discount curve '{}'", probe.curve))?
                .df(probe.tenor),
            "zero_rate" => discount_curves
                .get(&probe.curve)
                .ok_or_else(|| format!("missing discount curve '{}'", probe.curve))?
                .zero(probe.tenor),
            "forward_rate" => forward_curves
                .get(&probe.curve)
                .ok_or_else(|| format!("missing forward curve '{}'", probe.curve))?
                .rate(probe.tenor),
            "cpi" => inflation_curves
                .get(&probe.curve)
                .ok_or_else(|| format!("missing inflation curve '{}'", probe.curve))?
                .cpi(probe.tenor),
            "inflation_zero_rate" => {
                let curve = inflation_curves
                    .get(&probe.curve)
                    .ok_or_else(|| format!("missing inflation curve '{}'", probe.curve))?;
                let base_cpi = inflation_base_cpi
                    .get(&probe.curve)
                    .ok_or_else(|| format!("missing inflation base CPI '{}'", probe.curve))?;
                (curve.cpi(probe.tenor) / base_cpi).ln() / probe.tenor.max(1e-12)
            }
            other => return Err(format!("unsupported curve probe kind '{other}'")),
        };
        actuals.insert(probe.output.clone(), value);
    }
    if let Some(rmse) = inputs.calibration_rmse {
        actuals.insert("calibration_rmse".to_string(), rmse);
    }
    Ok(actuals)
}

pub(crate) fn run_hazard_fixture(fixture: &GoldenFixture) -> Result<BTreeMap<String, f64>, String> {
    let inputs: HazardCalibrationInputs = serde_json::from_value(fixture.inputs.clone())
        .map_err(|err| format!("parse hazard calibration inputs: {err}"))?;
    let mut hazard_curves = BTreeMap::new();
    for curve in &inputs.hazard {
        hazard_curves.insert(curve.id.clone(), build_hazard_curve(curve)?);
    }

    let mut actuals = BTreeMap::new();
    for probe in &inputs.probes {
        let curve = hazard_curves
            .get(&probe.curve)
            .ok_or_else(|| format!("missing hazard curve '{}'", probe.curve))?;
        let value = match probe.kind.as_str() {
            "hazard_rate" => curve.hazard_rate(probe.tenor),
            "survival_probability" => curve.sp(probe.tenor),
            other => return Err(format!("unsupported hazard probe kind '{other}'")),
        };
        actuals.insert(probe.output.clone(), value);
    }
    if let Some(rmse) = inputs.calibration_rmse {
        actuals.insert("calibration_rmse".to_string(), rmse);
    }
    Ok(actuals)
}

pub(crate) fn run_vol_smile_fixture(
    fixture: &GoldenFixture,
) -> Result<BTreeMap<String, f64>, String> {
    let inputs: VolSmileInputs = serde_json::from_value(fixture.inputs.clone())
        .map_err(|err| format!("parse vol smile inputs: {err}"))?;
    let mut actuals = BTreeMap::new();
    for smile in inputs.smiles {
        let prefix = format!("{}::{}", smile.id, smile.expiry);
        actuals.insert(format!("atm_vol::{prefix}"), smile.atm_vol);
        actuals.insert(
            format!("wing_25d_put_vol::{prefix}"),
            smile.wing_25d_put_vol,
        );
        actuals.insert(
            format!("wing_25d_call_vol::{prefix}"),
            smile.wing_25d_call_vol,
        );
        actuals.insert(
            format!("risk_reversal_25d::{prefix}"),
            smile.wing_25d_call_vol - smile.wing_25d_put_vol,
        );
        actuals.insert(
            format!("butterfly_25d::{prefix}"),
            0.5 * (smile.wing_25d_call_vol + smile.wing_25d_put_vol) - smile.atm_vol,
        );
        if let (Some(put), Some(call)) = (smile.wing_10d_put_vol, smile.wing_10d_call_vol) {
            actuals.insert(format!("risk_reversal_10d::{prefix}"), call - put);
            actuals.insert(
                format!("butterfly_10d::{prefix}"),
                0.5 * (call + put) - smile.atm_vol,
            );
        }
    }
    if let Some(rmse) = inputs.repriced_rmse {
        actuals.insert("repriced_rmse".to_string(), rmse);
    }
    Ok(actuals)
}

pub(crate) fn run_sabr_cube_fixture(
    fixture: &GoldenFixture,
) -> Result<BTreeMap<String, f64>, String> {
    let inputs: SabrCubeInputs = serde_json::from_value(fixture.inputs.clone())
        .map_err(|err| format!("parse SABR cube inputs: {err}"))?;
    let mut actuals = inputs.parameters;
    if let Some(rmse) = inputs.calibration_rmse {
        actuals.insert("calibration_rmse".to_string(), rmse);
    }
    Ok(actuals)
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
