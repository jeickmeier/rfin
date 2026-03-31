//! WASM bindings for scenario utility functions.
//!
//! Tenor parsing, interpolation weights, and day-count helpers.

use crate::core::error::js_error;
use wasm_bindgen::prelude::*;

/// Parse a tenor string to a fractional number of years using simple approximations.
///
/// Uses fixed approximations: 1D = 1/365, 1W = 7/365, 1M = 1/12, 1Y = 1.
///
/// # Arguments
/// * `tenor` - Tenor string (e.g. "1D", "3M", "5Y")
///
/// # Returns
/// Fractional year count
#[wasm_bindgen(js_name = parseTenorToYears)]
pub fn parse_tenor_to_years(tenor: &str) -> Result<f64, JsValue> {
    finstack_scenarios::utils::parse_tenor_to_years(tenor)
        .map_err(|e| js_error(format!("Invalid tenor '{tenor}': {e}")))
}

/// Parse a period string to an integer number of days.
///
/// Uses consistent approximations: 1D→1, 1W→7, 1M→30 (365/12 rounded), 1Y→365.
///
/// # Arguments
/// * `period` - Period string (e.g. "1D", "1W", "3M", "1Y")
///
/// # Returns
/// Number of days
#[wasm_bindgen(js_name = parsePeriodToDays)]
pub fn parse_period_to_days(period: &str) -> Result<i64, JsValue> {
    finstack_scenarios::utils::parse_period_to_days(period)
        .map_err(|e| js_error(format!("Invalid period '{period}': {e}")))
}

/// Calculate interpolation weights for distributing a bump at a target tenor
/// onto adjacent curve pillars.
///
/// # Arguments
/// * `target` - Time in years where the shock is applied
/// * `knots` - Sorted array of knot times in years
///
/// # Returns
/// Array of `[index, weight]` pairs
#[wasm_bindgen(js_name = calculateInterpolationWeights)]
pub fn calculate_interpolation_weights(target: f64, knots: Vec<f64>) -> JsValue {
    let weights = finstack_scenarios::utils::calculate_interpolation_weights(target, &knots);
    let array = js_sys::Array::new();
    for (index, weight) in weights {
        let pair = js_sys::Array::new();
        pair.push(&JsValue::from_f64(index as f64));
        pair.push(&JsValue::from_f64(weight));
        array.push(&pair);
    }
    JsValue::from(array)
}

/// Calculate interpolation weights with detailed extrapolation information.
///
/// # Arguments
/// * `target` - Time in years where the shock is applied
/// * `knots` - Sorted array of knot times in years
///
/// # Returns
/// Object with `weights` (array of [index, weight] pairs),
/// `isExtrapolation` (boolean), and `extrapolationDistance` (number or null)
#[wasm_bindgen(js_name = calculateInterpolationWeightsWithInfo)]
pub fn calculate_interpolation_weights_with_info(target: f64, knots: Vec<f64>) -> Result<JsValue, JsValue> {
    let result =
        finstack_scenarios::utils::calculate_interpolation_weights_with_info(target, &knots);

    let obj = js_sys::Object::new();

    let weights_array = js_sys::Array::new();
    for (index, weight) in &result.weights {
        let pair = js_sys::Array::new();
        pair.push(&JsValue::from_f64(*index as f64));
        pair.push(&JsValue::from_f64(*weight));
        weights_array.push(&pair);
    }
    js_sys::Reflect::set(&obj, &"weights".into(), &weights_array)?;
    js_sys::Reflect::set(
        &obj,
        &"isExtrapolation".into(),
        &JsValue::from_bool(result.is_extrapolation),
    )?;
    js_sys::Reflect::set(
        &obj,
        &"extrapolationDistance".into(),
        &match result.extrapolation_distance {
            Some(d) => JsValue::from_f64(d),
            None => JsValue::NULL,
        },
    )?;

    Ok(JsValue::from(obj))
}
