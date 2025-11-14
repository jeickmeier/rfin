//! WASM bindings for risk ladder calculations (KRD, CS01).
//!
//! Provides JavaScript-friendly risk analysis functions.

use crate::core::dates::date::JsDate;
use crate::core::market_data::context::JsMarketContext;
use crate::valuations::instruments::JsBond;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::metrics::standard_ir_dv01_buckets;
use wasm_bindgen::prelude::*;

/// Compute Key Rate Duration (KRD) DV01 ladder for a bond.
///
/// Returns a JavaScript object with `bucket` and `dv01` arrays that can be
/// easily converted to a table or chart.
///
/// @param {Bond} bond - Bond instrument to analyze
/// @param {MarketContext} market - Market context with discount curve
/// @param {Date} asOf - Valuation date
/// @param {number[] | null} bucketsYears - Optional tenor buckets in years (default: [0.25, 0.5, 1, 2, 3, 5, 7, 10, 15, 20, 30])
/// @param {number | null} bumpBp - Parallel shift in basis points (default: 1.0)
/// @returns {Object} Object with {bucket: string[], dv01: number[]}
///
/// @example
/// ```javascript
/// const ladder = krdDv01Ladder(bond, market, asOf, null, null);
/// console.log('Buckets:', ladder.bucket);
/// console.log('DV01:', ladder.dv01);
///
/// // Custom buckets
/// const custom = krdDv01Ladder(bond, market, asOf, [0.5, 1.0, 2.0, 5.0], 0.5);
/// ```
#[wasm_bindgen(js_name = krdDv01Ladder)]
pub fn krd_dv01_ladder(
    bond: &JsBond,
    market: &JsMarketContext,
    as_of: &JsDate,
    buckets_years: Option<Vec<f64>>,
    bump_bp: Option<f64>,
) -> Result<JsValue, JsValue> {
    let as_of_date = as_of.inner();
    let bump = bump_bp.unwrap_or(1.0);
    let buckets = buckets_years.unwrap_or_else(standard_ir_dv01_buckets);

    // Price bond at base case
    let bond_inner = bond.inner_bond();
    let base_pv = bond_inner
        .value(market.inner(), as_of_date)
        .map_err(|e| JsValue::from_str(&format!("Pricing failed: {}", e)))?;

    // Get first discount curve from market
    let mut disc_iter = market.inner().curves_of_type("Discount");
    let (_, disc_storage) = disc_iter
        .next()
        .ok_or_else(|| JsValue::from_str("No discount curves in market context"))?;
    let disc = disc_storage
        .discount()
        .ok_or_else(|| JsValue::from_str("Failed to extract discount curve"))?;

    // Compute DV01 for each bucket
    let mut bucket_labels = Vec::new();
    let mut dv01_values = Vec::new();

    for t in buckets {
        let label = if t < 1.0 {
            format!("{}m", (t * 12.0).round() as i32)
        } else {
            format!("{}y", t as i32)
        };

        // Bump curve at this key rate
        let bumped = disc
            .try_with_key_rate_bump_years(t, bump)
            .map_err(|e| JsValue::from_str(&format!("Bump failed: {}", e)))?;
        let temp_market = market.inner().clone().insert_discount(bumped);

        // Revalue with bumped curve
        let pv_bumped = bond_inner
            .value(&temp_market, as_of_date)
            .map_err(|e| JsValue::from_str(&format!("Revaluation failed: {}", e)))?;

        // DV01 = (PV_bumped - PV_base) / bump_bp
        let dv01 = (pv_bumped.amount() - base_pv.amount()) / bump;

        bucket_labels.push(label);
        dv01_values.push(dv01);
    }

    // Create JavaScript object with arrays
    let result = js_sys::Object::new();
    let bucket_array = js_sys::Array::from_iter(bucket_labels.iter().map(|s| JsValue::from_str(s)));
    let dv01_array = js_sys::Array::from_iter(dv01_values.iter().map(|v| JsValue::from_f64(*v)));

    js_sys::Reflect::set(&result, &JsValue::from_str("bucket"), &bucket_array)?;
    js_sys::Reflect::set(&result, &JsValue::from_str("dv01"), &dv01_array)?;

    Ok(result.into())
}

/// Compute CS01 ladder for a bond.
///
/// Similar to KRD but for credit spread sensitivity.
///
/// @param {Bond} bond - Bond instrument to analyze
/// @param {MarketContext} market - Market context
/// @param {Date} asOf - Valuation date
/// @param {number[] | null} bucketsYears - Optional tenor buckets in years
/// @param {number | null} bumpBp - Shift size in basis points (default: 1.0)
/// @returns {Object} Object with {bucket: string[], cs01: number[]}
#[wasm_bindgen(js_name = cs01Ladder)]
pub fn cs01_ladder(
    bond: &JsBond,
    market: &JsMarketContext,
    as_of: &JsDate,
    buckets_years: Option<Vec<f64>>,
    bump_bp: Option<f64>,
) -> Result<JsValue, JsValue> {
    let as_of_date = as_of.inner();
    let bump = bump_bp.unwrap_or(1.0);
    let buckets = buckets_years.unwrap_or_else(standard_ir_dv01_buckets);

    // Price bond at base case
    let bond_inner = bond.inner_bond();
    let base_pv = bond_inner
        .value(market.inner(), as_of_date)
        .map_err(|e| JsValue::from_str(&format!("Pricing failed: {}", e)))?;

    // Get discount curve
    let mut disc_iter = market.inner().curves_of_type("Discount");
    let (_, disc_storage) = disc_iter
        .next()
        .ok_or_else(|| JsValue::from_str("No discount curves in market context"))?;
    let disc = disc_storage
        .discount()
        .ok_or_else(|| JsValue::from_str("Failed to extract discount curve"))?;

    // Compute CS01 for each bucket
    let mut bucket_labels = Vec::new();
    let mut cs01_values = Vec::new();

    for t in buckets {
        let label = if t < 1.0 {
            format!("{}m", (t * 12.0).round() as i32)
        } else {
            format!("{}y", t as i32)
        };

        // Bump curve at this key rate
        let bumped = disc
            .try_with_key_rate_bump_years(t, bump)
            .map_err(|e| JsValue::from_str(&format!("Bump failed: {}", e)))?;
        let temp_market = market.inner().clone().insert_discount(bumped);

        // Revalue
        let pv_bumped = bond_inner
            .value(&temp_market, as_of_date)
            .map_err(|e| JsValue::from_str(&format!("Revaluation failed: {}", e)))?;

        // CS01 = (PV_bumped - PV_base) / bump_bp
        let cs01 = (pv_bumped.amount() - base_pv.amount()) / bump;

        bucket_labels.push(label);
        cs01_values.push(cs01);
    }

    // Create JavaScript object
    let result = js_sys::Object::new();
    let bucket_array = js_sys::Array::from_iter(bucket_labels.iter().map(|s| JsValue::from_str(s)));
    let cs01_array = js_sys::Array::from_iter(cs01_values.iter().map(|v| JsValue::from_f64(*v)));

    js_sys::Reflect::set(&result, &JsValue::from_str("bucket"), &bucket_array)?;
    js_sys::Reflect::set(&result, &JsValue::from_str("cs01"), &cs01_array)?;

    Ok(result.into())
}
