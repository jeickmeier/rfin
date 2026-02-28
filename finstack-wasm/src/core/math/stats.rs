//! Statistical functions for WASM bindings.

use finstack_core::math::stats::{correlation, covariance, mean, variance};
use wasm_bindgen::prelude::*;

/// Calculate the arithmetic mean of a slice.
///
/// @param {Float64Array} values - Array of values
/// @returns {number} Arithmetic mean
///
/// @example
/// ```javascript
/// const data = new Float64Array([1.0, 2.0, 3.0, 4.0]);
/// const avg = mean(data); // 2.5
/// ```
#[wasm_bindgen(js_name = mean)]
pub fn mean_js(values: &[f64]) -> f64 {
    mean(values)
}

/// Calculate the sample variance of a slice (unbiased, n-1 denominator).
///
/// @param {Float64Array} values - Array of values
/// @returns {number} Sample variance
///
/// @example
/// ```javascript
/// const data = new Float64Array([1.0, 2.0, 3.0, 4.0]);
/// const v = variance(data); // 1.6667
/// ```
#[wasm_bindgen(js_name = variance)]
pub fn variance_js(values: &[f64]) -> f64 {
    variance(values)
}

/// Calculate the sample covariance between two arrays.
///
/// @param {Float64Array} x - First array
/// @param {Float64Array} y - Second array
/// @returns {number} Sample covariance
/// @throws {Error} If arrays have different lengths
///
/// @example
/// ```javascript
/// const x = new Float64Array([1.0, 2.0, 3.0]);
/// const y = new Float64Array([2.0, 4.0, 6.0]);
/// const cov = covariance(x, y); // Perfect positive covariance
/// ```
#[wasm_bindgen(js_name = covariance)]
pub fn covariance_js(x: &[f64], y: &[f64]) -> Result<f64, JsValue> {
    if x.len() != y.len() {
        return Err(JsValue::from_str("Arrays must have the same length"));
    }
    Ok(covariance(x, y))
}

/// Calculate the Pearson correlation coefficient between two arrays.
///
/// @param {Float64Array} x - First array
/// @param {Float64Array} y - Second array
/// @returns {number} Correlation coefficient in [-1, 1]
/// @throws {Error} If arrays have different lengths
///
/// @example
/// ```javascript
/// const x = new Float64Array([1.0, 2.0, 3.0]);
/// const y = new Float64Array([2.0, 4.0, 6.0]);
/// const corr = correlation(x, y); // 1.0 (perfect positive)
/// ```
#[wasm_bindgen(js_name = correlation)]
pub fn correlation_js(x: &[f64], y: &[f64]) -> Result<f64, JsValue> {
    if x.len() != y.len() {
        return Err(JsValue::from_str("Arrays must have the same length"));
    }
    Ok(correlation(x, y))
}
