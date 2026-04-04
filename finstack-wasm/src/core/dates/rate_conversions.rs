use crate::core::error::core_to_js;
use wasm_bindgen::prelude::*;

/// Convert a simple (linear) interest rate to a periodically compounded rate.
///
/// @param {number} simpleRate - Simple interest rate (e.g. 0.05 for 5%)
/// @param {number} yearFraction - Time period as year fraction
/// @param {number} periodsPerYear - Compounding frequency (e.g. 2 for semi-annual)
/// @returns {number} Equivalent periodic rate
#[wasm_bindgen(js_name = simpleToPeriodic)]
pub fn simple_to_periodic(
    simple_rate: f64,
    year_fraction: f64,
    periods_per_year: u32,
) -> Result<f64, JsValue> {
    finstack_core::dates::rate_conversions::simple_to_periodic(
        simple_rate,
        year_fraction,
        periods_per_year,
    )
    .map_err(core_to_js)
}

/// Convert a periodically compounded rate to a simple (linear) rate.
///
/// @param {number} periodicRate - Periodic rate (e.g. 0.05 for 5%)
/// @param {number} yearFraction - Time period as year fraction
/// @param {number} periodsPerYear - Compounding frequency
/// @returns {number} Equivalent simple rate
#[wasm_bindgen(js_name = periodicToSimple)]
pub fn periodic_to_simple(
    periodic_rate: f64,
    year_fraction: f64,
    periods_per_year: u32,
) -> Result<f64, JsValue> {
    finstack_core::dates::rate_conversions::periodic_to_simple(
        periodic_rate,
        year_fraction,
        periods_per_year,
    )
    .map_err(core_to_js)
}

/// Convert a periodically compounded rate to continuous compounding.
///
/// @param {number} periodicRate - Periodic rate
/// @param {number} periodsPerYear - Compounding frequency
/// @returns {number} Equivalent continuously compounded rate
#[wasm_bindgen(js_name = periodicToContinuous)]
pub fn periodic_to_continuous(periodic_rate: f64, periods_per_year: u32) -> Result<f64, JsValue> {
    finstack_core::dates::rate_conversions::periodic_to_continuous(periodic_rate, periods_per_year)
        .map_err(core_to_js)
}

/// Convert a continuously compounded rate to periodic compounding.
///
/// @param {number} continuousRate - Continuously compounded rate
/// @param {number} periodsPerYear - Target compounding frequency
/// @returns {number} Equivalent periodic rate
#[wasm_bindgen(js_name = continuousToPeriodic)]
pub fn continuous_to_periodic(continuous_rate: f64, periods_per_year: u32) -> Result<f64, JsValue> {
    finstack_core::dates::rate_conversions::continuous_to_periodic(
        continuous_rate,
        periods_per_year,
    )
    .map_err(core_to_js)
}

/// Convert a simple (linear) rate to continuous compounding.
///
/// @param {number} simpleRate - Simple rate
/// @param {number} yearFraction - Time period as year fraction
/// @returns {number} Equivalent continuously compounded rate
#[wasm_bindgen(js_name = simpleToContinuous)]
pub fn simple_to_continuous(simple_rate: f64, year_fraction: f64) -> Result<f64, JsValue> {
    finstack_core::dates::rate_conversions::simple_to_continuous(simple_rate, year_fraction)
        .map_err(core_to_js)
}

/// Convert a continuously compounded rate to simple (linear) rate.
///
/// @param {number} continuousRate - Continuously compounded rate
/// @param {number} yearFraction - Time period as year fraction
/// @returns {number} Equivalent simple rate
#[wasm_bindgen(js_name = continuousToSimple)]
pub fn continuous_to_simple(continuous_rate: f64, year_fraction: f64) -> Result<f64, JsValue> {
    finstack_core::dates::rate_conversions::continuous_to_simple(continuous_rate, year_fraction)
        .map_err(core_to_js)
}
