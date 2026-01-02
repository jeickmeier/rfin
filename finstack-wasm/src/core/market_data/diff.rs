//! WASM bindings for market data comparison and shift measurement.
//!
//! Provides utilities for measuring market movements between two `MarketContext`
//! instances. Used for metrics-based P&L attribution, risk reporting, and
//! scenario analysis.

use crate::core::currency::JsCurrency;
use crate::core::error::js_error;
use crate::core::market_data::context::JsMarketContext;
use crate::core::utils::js_array_from_iter;
use finstack_core::market_data::diff::{
    measure_bucketed_discount_shift, measure_correlation_shift, measure_discount_curve_shift,
    measure_fx_shift, measure_hazard_curve_shift, measure_inflation_curve_shift,
    measure_scalar_shift, measure_vol_surface_shift, TenorSamplingMethod, ATM_MONEYNESS,
    DEFAULT_VOL_EXPIRY, STANDARD_TENORS,
};
use wasm_bindgen::prelude::*;

/// Method for selecting tenor points when measuring curve shifts.
#[wasm_bindgen(js_name = TenorSamplingMethod)]
pub struct JsTenorSamplingMethod {
    inner: TenorSamplingMethod,
}

#[wasm_bindgen(js_class = TenorSamplingMethod)]
impl JsTenorSamplingMethod {
    /// Standard swap market tenors (3M, 6M, 1Y, 2Y, 3Y, 5Y, 7Y, 10Y, 30Y).
    #[wasm_bindgen(js_name = standard)]
    pub fn standard() -> JsTenorSamplingMethod {
        JsTenorSamplingMethod {
            inner: TenorSamplingMethod::Standard,
        }
    }

    /// Use curve's own knot points dynamically.
    #[wasm_bindgen(js_name = dynamic)]
    pub fn dynamic() -> JsTenorSamplingMethod {
        JsTenorSamplingMethod {
            inner: TenorSamplingMethod::Dynamic,
        }
    }

    /// Custom tenor list specified by caller.
    #[wasm_bindgen(js_name = custom)]
    pub fn custom(tenors: Vec<f64>) -> JsTenorSamplingMethod {
        JsTenorSamplingMethod {
            inner: TenorSamplingMethod::Custom(tenors),
        }
    }

    /// Get the default sampling method (Standard).
    #[wasm_bindgen(js_name = default)]
    pub fn default_method() -> JsTenorSamplingMethod {
        JsTenorSamplingMethod {
            inner: TenorSamplingMethod::default(),
        }
    }
}

// -----------------------------------------------------------------------------
// Constants
// -----------------------------------------------------------------------------

/// Get the standard market tenors (in years).
#[wasm_bindgen(js_name = standardTenors)]
pub fn standard_tenors() -> js_sys::Array {
    let tenors = STANDARD_TENORS.iter().map(|&t| JsValue::from_f64(t));
    js_array_from_iter(tenors)
}

/// Get the ATM reference strike multiplier (1.0 = 100% of spot).
#[wasm_bindgen(js_name = atmMoneyness)]
pub fn atm_moneyness() -> f64 {
    ATM_MONEYNESS
}

/// Get the default volatility surface expiry for sampling (1 year).
#[wasm_bindgen(js_name = defaultVolExpiry)]
pub fn default_vol_expiry() -> f64 {
    DEFAULT_VOL_EXPIRY
}

// -----------------------------------------------------------------------------
// Curve Shift Measurement Functions
// -----------------------------------------------------------------------------

/// Measure average parallel rate shift in discount curve (basis points).
///
/// # Arguments
/// * `curve_id` - Curve identifier
/// * `market_t0` - Market context at T₀
/// * `market_t1` - Market context at T₁
/// * `method` - Optional sampling method (defaults to Standard)
#[wasm_bindgen(js_name = measureDiscountCurveShift)]
pub fn js_measure_discount_curve_shift(
    curve_id: &str,
    market_t0: &JsMarketContext,
    market_t1: &JsMarketContext,
    method: Option<JsTenorSamplingMethod>,
) -> Result<f64, JsValue> {
    let sampling = method
        .map(|m| m.inner.clone())
        .unwrap_or(TenorSamplingMethod::Standard);
    measure_discount_curve_shift(curve_id, market_t0.inner(), market_t1.inner(), sampling)
        .map_err(|e| js_error(e.to_string()))
}

/// Measure bucketed rate shifts for detailed attribution.
///
/// # Arguments
/// * `curve_id` - Curve identifier
/// * `market_t0` - Market context at T₀
/// * `market_t1` - Market context at T₁
/// * `tenors` - Tenor points to measure (in years)
///
/// # Returns
/// Array of [tenor, shift_bp] pairs.
#[wasm_bindgen(js_name = measureBucketedDiscountShift)]
pub fn js_measure_bucketed_discount_shift(
    curve_id: &str,
    market_t0: &JsMarketContext,
    market_t1: &JsMarketContext,
    tenors: Vec<f64>,
) -> Result<js_sys::Array, JsValue> {
    let shifts =
        measure_bucketed_discount_shift(curve_id, market_t0.inner(), market_t1.inner(), &tenors)
            .map_err(|e| js_error(e.to_string()))?;

    let result = js_sys::Array::new();
    for (tenor, shift) in shifts {
        let pair = js_sys::Array::new();
        pair.push(&JsValue::from_f64(tenor));
        pair.push(&JsValue::from_f64(shift));
        result.push(&pair);
    }
    Ok(result)
}

/// Measure average parallel spread shift in hazard curve (basis points).
///
/// # Arguments
/// * `curve_id` - Curve identifier
/// * `market_t0` - Market context at T₀
/// * `market_t1` - Market context at T₁
/// * `method` - Optional sampling method (defaults to Standard)
#[wasm_bindgen(js_name = measureHazardCurveShift)]
pub fn js_measure_hazard_curve_shift(
    curve_id: &str,
    market_t0: &JsMarketContext,
    market_t1: &JsMarketContext,
    method: Option<JsTenorSamplingMethod>,
) -> Result<f64, JsValue> {
    let sampling = method
        .map(|m| m.inner.clone())
        .unwrap_or(TenorSamplingMethod::Standard);
    measure_hazard_curve_shift(curve_id, market_t0.inner(), market_t1.inner(), sampling)
        .map_err(|e| js_error(e.to_string()))
}

/// Measure average inflation rate shift (basis points).
///
/// # Arguments
/// * `curve_id` - Curve identifier
/// * `market_t0` - Market context at T₀
/// * `market_t1` - Market context at T₁
#[wasm_bindgen(js_name = measureInflationCurveShift)]
pub fn js_measure_inflation_curve_shift(
    curve_id: &str,
    market_t0: &JsMarketContext,
    market_t1: &JsMarketContext,
) -> Result<f64, JsValue> {
    measure_inflation_curve_shift(curve_id, market_t0.inner(), market_t1.inner())
        .map_err(|e| js_error(e.to_string()))
}

/// Measure average correlation shift (percentage points).
///
/// # Arguments
/// * `curve_id` - Curve identifier
/// * `market_t0` - Market context at T₀
/// * `market_t1` - Market context at T₁
#[wasm_bindgen(js_name = measureCorrelationShift)]
pub fn js_measure_correlation_shift(
    curve_id: &str,
    market_t0: &JsMarketContext,
    market_t1: &JsMarketContext,
) -> Result<f64, JsValue> {
    measure_correlation_shift(curve_id, market_t0.inner(), market_t1.inner())
        .map_err(|e| js_error(e.to_string()))
}

/// Measure volatility surface shift (percentage points).
///
/// # Arguments
/// * `surface_id` - Volatility surface identifier
/// * `market_t0` - Market context at T₀
/// * `market_t1` - Market context at T₁
/// * `reference_expiry` - Optional expiry to measure (defaults to 1Y ATM)
/// * `reference_strike` - Optional strike to measure (defaults to ATM)
#[wasm_bindgen(js_name = measureVolSurfaceShift)]
pub fn js_measure_vol_surface_shift(
    surface_id: &str,
    market_t0: &JsMarketContext,
    market_t1: &JsMarketContext,
    reference_expiry: Option<f64>,
    reference_strike: Option<f64>,
) -> Result<f64, JsValue> {
    measure_vol_surface_shift(
        surface_id,
        market_t0.inner(),
        market_t1.inner(),
        reference_expiry,
        reference_strike,
    )
    .map_err(|e| js_error(e.to_string()))
}

/// Measure FX spot rate shift (percentage change).
///
/// # Arguments
/// * `base_ccy` - Base currency
/// * `quote_ccy` - Quote currency
/// * `market_t0` - Market context at T₀
/// * `market_t1` - Market context at T₁
/// * `as_of_t0` - Valuation date for T₀ (YYYY-MM-DD string)
/// * `as_of_t1` - Valuation date for T₁ (YYYY-MM-DD string)
#[wasm_bindgen(js_name = measureFxShift)]
pub fn js_measure_fx_shift(
    base_ccy: &JsCurrency,
    quote_ccy: &JsCurrency,
    market_t0: &JsMarketContext,
    market_t1: &JsMarketContext,
    as_of_t0: &str,
    as_of_t1: &str,
) -> Result<f64, JsValue> {
    let date_t0 = finstack_core::dates::Date::parse(
        as_of_t0,
        &time::format_description::well_known::Iso8601::DATE,
    )
    .map_err(|e| js_error(format!("Invalid date t0: {}", e)))?;
    let date_t1 = finstack_core::dates::Date::parse(
        as_of_t1,
        &time::format_description::well_known::Iso8601::DATE,
    )
    .map_err(|e| js_error(format!("Invalid date t1: {}", e)))?;

    measure_fx_shift(
        base_ccy.inner(),
        quote_ccy.inner(),
        market_t0.inner(),
        market_t1.inner(),
        date_t0,
        date_t1,
    )
    .map_err(|e| js_error(e.to_string()))
}

/// Measure market scalar shift (percentage change).
///
/// # Arguments
/// * `scalar_id` - Market scalar identifier
/// * `market_t0` - Market context at T₀
/// * `market_t1` - Market context at T₁
#[wasm_bindgen(js_name = measureScalarShift)]
pub fn js_measure_scalar_shift(
    scalar_id: &str,
    market_t0: &JsMarketContext,
    market_t1: &JsMarketContext,
) -> Result<f64, JsValue> {
    measure_scalar_shift(scalar_id, market_t0.inner(), market_t1.inner())
        .map_err(|e| js_error(e.to_string()))
}
