//! WASM bindings for forecast functions.
//!
//! Exposes the low-level forecast functions from `finstack_statements::forecast`
//! for direct use without a full model evaluation.

use crate::core::error::js_error;
use crate::statements::types::JsForecastSpec;
use finstack_core::dates::PeriodId;
use std::str::FromStr;
use wasm_bindgen::prelude::*;

fn parse_period_ids(ids: Vec<String>) -> Result<Vec<PeriodId>, JsValue> {
    ids.iter()
        .map(|s| {
            PeriodId::from_str(s).map_err(|e| js_error(format!("Invalid period ID '{s}': {e}")))
        })
        .collect()
}

fn results_to_js(results: indexmap::IndexMap<PeriodId, f64>) -> Result<JsValue, JsValue> {
    let obj = js_sys::Object::new();
    for (pid, value) in results {
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str(&pid.to_string()),
            &JsValue::from_f64(value),
        )?;
    }
    Ok(JsValue::from(obj))
}

/// Apply a forecast specification to generate values for forecast periods.
///
/// # Arguments
/// * `spec` - Forecast specification
/// * `base_value` - Starting value (typically last actual value)
/// * `forecast_periods` - Array of period ID strings
///
/// # Returns
/// Object mapping period IDs to forecasted values
#[wasm_bindgen(js_name = applyForecast)]
pub fn apply_forecast(
    spec: &JsForecastSpec,
    base_value: f64,
    forecast_periods: Vec<String>,
) -> Result<JsValue, JsValue> {
    let periods = parse_period_ids(forecast_periods)?;
    let result = finstack_statements::forecast::apply_forecast(&spec.inner, base_value, &periods)
        .map_err(|e| js_error(format!("Forecast failed: {e}")))?;
    results_to_js(result)
}

/// Forward-fill forecast: carry forward the base value unchanged.
///
/// # Arguments
/// * `base_value` - Value to carry forward
/// * `forecast_periods` - Array of period ID strings
///
/// # Returns
/// Object mapping period IDs to the base value
#[wasm_bindgen(js_name = forwardFill)]
pub fn forward_fill(base_value: f64, forecast_periods: Vec<String>) -> Result<JsValue, JsValue> {
    let periods = parse_period_ids(forecast_periods)?;
    let result = finstack_statements::forecast::forward_fill(base_value, &periods)
        .map_err(|e| js_error(format!("Forward fill failed: {e}")))?;
    results_to_js(result)
}

/// Growth percentage forecast: apply constant compound growth.
///
/// # Arguments
/// * `base_value` - Starting value
/// * `forecast_periods` - Array of period ID strings
/// * `params` - JSON object with `rate` field (e.g. `{ "rate": 0.05 }`)
///
/// # Returns
/// Object mapping period IDs to compounded values
#[wasm_bindgen(js_name = growthPct)]
pub fn growth_pct(
    base_value: f64,
    forecast_periods: Vec<String>,
    params: JsValue,
) -> Result<JsValue, JsValue> {
    let periods = parse_period_ids(forecast_periods)?;
    let params_map: indexmap::IndexMap<String, serde_json::Value> =
        serde_wasm_bindgen::from_value(params)
            .map_err(|e| js_error(format!("Invalid params: {e}")))?;
    let result = finstack_statements::forecast::growth_pct(base_value, &periods, &params_map)
        .map_err(|e| js_error(format!("Growth pct failed: {e}")))?;
    results_to_js(result)
}

/// Curve percentage forecast: apply period-specific growth rates.
///
/// # Arguments
/// * `base_value` - Starting value
/// * `forecast_periods` - Array of period ID strings
/// * `params` - JSON object with `curve` field (array of rates)
///
/// # Returns
/// Object mapping period IDs to values
#[wasm_bindgen(js_name = curvePct)]
pub fn curve_pct(
    base_value: f64,
    forecast_periods: Vec<String>,
    params: JsValue,
) -> Result<JsValue, JsValue> {
    let periods = parse_period_ids(forecast_periods)?;
    let params_map: indexmap::IndexMap<String, serde_json::Value> =
        serde_wasm_bindgen::from_value(params)
            .map_err(|e| js_error(format!("Invalid params: {e}")))?;
    let result = finstack_statements::forecast::curve_pct(base_value, &periods, &params_map)
        .map_err(|e| js_error(format!("Curve pct failed: {e}")))?;
    results_to_js(result)
}

/// Time series forecast.
///
/// # Arguments
/// * `base_value` - Starting value
/// * `forecast_periods` - Array of period ID strings
/// * `params` - JSON object with time series parameters
///
/// # Returns
/// Object mapping period IDs to values
#[wasm_bindgen(js_name = timeseriesForecast)]
pub fn timeseries_forecast(
    base_value: f64,
    forecast_periods: Vec<String>,
    params: JsValue,
) -> Result<JsValue, JsValue> {
    let periods = parse_period_ids(forecast_periods)?;
    let params_map: indexmap::IndexMap<String, serde_json::Value> =
        serde_wasm_bindgen::from_value(params)
            .map_err(|e| js_error(format!("Invalid params: {e}")))?;
    let result =
        finstack_statements::forecast::timeseries_forecast(base_value, &periods, &params_map)
            .map_err(|e| js_error(format!("Timeseries forecast failed: {e}")))?;
    results_to_js(result)
}

/// Seasonal forecast with optional growth.
///
/// # Arguments
/// * `base_value` - Starting value
/// * `forecast_periods` - Array of period ID strings
/// * `params` - JSON object with seasonal pattern and optional growth
///
/// # Returns
/// Object mapping period IDs to values
#[wasm_bindgen(js_name = seasonalForecast)]
pub fn seasonal_forecast(
    base_value: f64,
    forecast_periods: Vec<String>,
    params: JsValue,
) -> Result<JsValue, JsValue> {
    let periods = parse_period_ids(forecast_periods)?;
    let params_map: indexmap::IndexMap<String, serde_json::Value> =
        serde_wasm_bindgen::from_value(params)
            .map_err(|e| js_error(format!("Invalid params: {e}")))?;
    let result =
        finstack_statements::forecast::seasonal_forecast(base_value, &periods, &params_map)
            .map_err(|e| js_error(format!("Seasonal forecast failed: {e}")))?;
    results_to_js(result)
}

/// Normal distribution forecast with deterministic seeding.
///
/// # Arguments
/// * `base_value` - Mean baseline value
/// * `forecast_periods` - Array of period ID strings
/// * `params` - JSON object with `mean`, `std_dev`, `seed` fields
///
/// # Returns
/// Object mapping period IDs to sampled values
#[wasm_bindgen(js_name = normalForecast)]
pub fn normal_forecast(
    base_value: f64,
    forecast_periods: Vec<String>,
    params: JsValue,
) -> Result<JsValue, JsValue> {
    let periods = parse_period_ids(forecast_periods)?;
    let params_map: indexmap::IndexMap<String, serde_json::Value> =
        serde_wasm_bindgen::from_value(params)
            .map_err(|e| js_error(format!("Invalid params: {e}")))?;
    let result = finstack_statements::forecast::normal_forecast(base_value, &periods, &params_map)
        .map_err(|e| js_error(format!("Normal forecast failed: {e}")))?;
    results_to_js(result)
}

/// Log-normal distribution forecast (always positive values).
///
/// # Arguments
/// * `base_value` - Baseline value
/// * `forecast_periods` - Array of period ID strings
/// * `params` - JSON object with `mean`, `std_dev`, `seed` fields
///
/// # Returns
/// Object mapping period IDs to sampled values
#[wasm_bindgen(js_name = lognormalForecast)]
pub fn lognormal_forecast(
    base_value: f64,
    forecast_periods: Vec<String>,
    params: JsValue,
) -> Result<JsValue, JsValue> {
    let periods = parse_period_ids(forecast_periods)?;
    let params_map: indexmap::IndexMap<String, serde_json::Value> =
        serde_wasm_bindgen::from_value(params)
            .map_err(|e| js_error(format!("Invalid params: {e}")))?;
    let result =
        finstack_statements::forecast::lognormal_forecast(base_value, &periods, &params_map)
            .map_err(|e| js_error(format!("Lognormal forecast failed: {e}")))?;
    results_to_js(result)
}

/// Override forecast: sparse period values for specific overrides.
///
/// # Arguments
/// * `base_value` - Default value for periods without overrides
/// * `forecast_periods` - Array of period ID strings
/// * `params` - JSON object with period-to-value overrides
///
/// # Returns
/// Object mapping period IDs to values
#[wasm_bindgen(js_name = applyOverride)]
pub fn apply_override(
    base_value: f64,
    forecast_periods: Vec<String>,
    params: JsValue,
) -> Result<JsValue, JsValue> {
    let periods = parse_period_ids(forecast_periods)?;
    let params_map: indexmap::IndexMap<String, serde_json::Value> =
        serde_wasm_bindgen::from_value(params)
            .map_err(|e| js_error(format!("Invalid params: {e}")))?;
    let result = finstack_statements::forecast::apply_override(base_value, &periods, &params_map)
        .map_err(|e| js_error(format!("Override forecast failed: {e}")))?;
    results_to_js(result)
}
