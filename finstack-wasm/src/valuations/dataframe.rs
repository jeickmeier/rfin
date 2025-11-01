//! WASM bindings for DataFrame export from valuation results.
//!
//! Provides conversion of valuation results to JavaScript-compatible formats:
//! - Array of row objects (for use with d3, lodash, or vanilla JS)
//! - JSON format for easy serialization

use finstack_valuations::results::dataframe::results_to_rows;
use finstack_valuations::results::ValuationResult;
use js_sys::{Array, Object, Reflect};
use wasm_bindgen::prelude::*;

/// Convert a list of ValuationResult to an array of row objects for DataFrame construction.
///
/// Returns an array of objects with columns: instrument_id, as_of_date, pv, currency,
/// dv01 (optional), convexity (optional), duration (optional), ytm (optional)
///
/// @param {Array<ValuationResult>} results - Array of valuation results to convert
/// @returns {Array<Object>} Array of row objects suitable for DataFrame libraries
///
/// @example
/// ```typescript
/// const results = [result1, result2, result3];
/// const rows = resultsToRows(results);
/// // rows is an array like:
/// // [
/// //   { instrument_id: "BOND-001", as_of_date: "2025-01-15", pv: 1042315.67, currency: "USD", dv01: 1250.0, ... },
/// //   { instrument_id: "BOND-002", as_of_date: "2025-01-15", pv: 500000.0, currency: "EUR", ... },
/// //   ...
/// // ]
/// ```
#[wasm_bindgen(js_name = resultsToRows)]
pub fn results_to_rows_wasm(results: Array) -> Result<Array, JsValue> {
    // Parse ValuationResult objects from JavaScript
    let mut rust_results: Vec<ValuationResult> = Vec::new();
    for result_js in results.iter() {
        // Try to deserialize ValuationResult directly from JsValue
        // ValuationResult implements Serialize/Deserialize, so serde_wasm_bindgen should work
        let valuation_result = serde_wasm_bindgen::from_value::<ValuationResult>(result_js.clone())
            .map_err(|e| JsValue::from_str(&format!("Failed to parse ValuationResult: {}. Make sure all items are ValuationResult objects.", e)))?;
        rust_results.push(valuation_result);
    }

    // Convert to rows using the core function
    let rows = results_to_rows(&rust_results);

    // Convert to JavaScript array of objects
    let js_rows = Array::new();
    for row in rows {
        let obj = Object::new();
        let _ = Reflect::set(
            &obj,
            &JsValue::from_str("instrument_id"),
            &JsValue::from_str(&row.instrument_id),
        );
        let _ = Reflect::set(
            &obj,
            &JsValue::from_str("as_of_date"),
            &JsValue::from_str(&row.as_of_date),
        );
        let _ = Reflect::set(&obj, &JsValue::from_str("pv"), &JsValue::from_f64(row.pv));
        let _ = Reflect::set(
            &obj,
            &JsValue::from_str("currency"),
            &JsValue::from_str(&row.currency),
        );

        if let Some(dv01) = row.dv01 {
            let _ = Reflect::set(&obj, &JsValue::from_str("dv01"), &JsValue::from_f64(dv01));
        }
        if let Some(convexity) = row.convexity {
            let _ = Reflect::set(
                &obj,
                &JsValue::from_str("convexity"),
                &JsValue::from_f64(convexity),
            );
        }
        if let Some(duration) = row.duration {
            let _ = Reflect::set(
                &obj,
                &JsValue::from_str("duration"),
                &JsValue::from_f64(duration),
            );
        }
        if let Some(ytm) = row.ytm {
            let _ = Reflect::set(&obj, &JsValue::from_str("ytm"), &JsValue::from_f64(ytm));
        }

        js_rows.push(&obj);
    }

    Ok(js_rows)
}

/// Convert a list of ValuationResult to JSON format for DataFrame construction.
///
/// This is a convenience wrapper around resultsToRows that returns JSON string.
/// Useful for serialization or when working with JSON-based DataFrame libraries.
///
/// @param {Array<ValuationResult>} results - Array of valuation results to convert
/// @returns {string} JSON string representation of the rows
///
/// @example
/// ```typescript
/// const results = [result1, result2, result3];
/// const json = resultsToJson(results);
/// const rows = JSON.parse(json);
/// ```
#[wasm_bindgen(js_name = resultsToJson)]
pub fn results_to_json_wasm(results: Array) -> Result<String, JsValue> {
    let rows_array = results_to_rows_wasm(results)?;
    let json = js_sys::JSON::stringify(&rows_array)
        .map_err(|_| JsValue::from_str("Failed to serialize to JSON"))?;
    Ok(json.as_string().unwrap_or_default())
}
