//! Special mathematical functions for WASM bindings.

use finstack_core::math::special_functions::{
    erf, norm_cdf, norm_cdf_with_params, norm_pdf, norm_pdf_with_params, standard_normal_inv_cdf,
    try_student_t_cdf, try_student_t_inv_cdf,
};
use wasm_bindgen::prelude::*;

/// Standard normal cumulative distribution function (CDF).
///
/// Returns P(Z ≤ x) where Z is standard normal N(0,1).
///
/// @param {number} x - Value to evaluate
/// @returns {number} Probability P(Z ≤ x)
///
/// @example
/// ```javascript
/// normCdf(0.0);   // 0.5
/// normCdf(1.96);  // ~0.975
/// normCdf(-1.96); // ~0.025
/// ```
#[wasm_bindgen(js_name = normCdf)]
pub fn norm_cdf_js(x: f64) -> f64 {
    norm_cdf(x)
}

/// Standard normal probability density function (PDF).
///
/// Returns the density of the standard normal distribution at x.
///
/// @param {number} x - Value to evaluate
/// @returns {number} Probability density
///
/// @example
/// ```javascript
/// normPdf(0.0);  // ~0.3989 (peak of the bell curve)
/// normPdf(1.0);  // ~0.2420
/// ```
#[wasm_bindgen(js_name = normPdf)]
pub fn norm_pdf_js(x: f64) -> f64 {
    norm_pdf(x)
}

/// General normal cumulative distribution function.
///
/// @param {number} x - Value to evaluate
/// @param {number} mean - Distribution mean
/// @param {number} stdDev - Distribution standard deviation (must be positive)
/// @returns {number} Probability P(X ≤ x)
/// @throws {Error} If `stdDev` is not finite and positive
#[wasm_bindgen(js_name = normCdfWithParams)]
pub fn norm_cdf_with_params_js(x: f64, mean: f64, std_dev: f64) -> Result<f64, JsValue> {
    norm_cdf_with_params(x, mean, std_dev).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// General normal probability density function.
///
/// @param {number} x - Value to evaluate
/// @param {number} mean - Distribution mean
/// @param {number} stdDev - Distribution standard deviation (must be positive)
/// @returns {number} Probability density
/// @throws {Error} If `stdDev` is not finite and positive
#[wasm_bindgen(js_name = normPdfWithParams)]
pub fn norm_pdf_with_params_js(x: f64, mean: f64, std_dev: f64) -> Result<f64, JsValue> {
    norm_pdf_with_params(x, mean, std_dev).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Inverse of the standard normal CDF (quantile function).
///
/// Given probability p, returns x such that P(Z ≤ x) = p.
///
/// @param {number} p - Probability in (0, 1)
/// @returns {number} Quantile value
/// @throws {Error} If p is not in (0, 1)
///
/// @example
/// ```javascript
/// normInvCdf(0.5);   // 0.0
/// normInvCdf(0.975); // ~1.96
/// normInvCdf(0.025); // ~-1.96
/// ```
#[wasm_bindgen(js_name = normInvCdf)]
pub fn norm_inv_cdf_js(p: f64) -> Result<f64, JsValue> {
    if p <= 0.0 || p >= 1.0 {
        return Err(JsValue::from_str("Probability must be in (0, 1)"));
    }
    Ok(standard_normal_inv_cdf(p))
}

/// Error function (erf).
///
/// @param {number} x - Value to evaluate
/// @returns {number} Error function value
///
/// @example
/// ```javascript
/// erf(0.0);  // 0.0
/// erf(1.0);  // ~0.8427
/// ```
#[wasm_bindgen(js_name = erf)]
pub fn erf_js(x: f64) -> f64 {
    erf(x)
}

/// Student's t-distribution CDF.
///
/// @param {number} x - Value to evaluate
/// @param {number} df - Degrees of freedom (must be positive)
/// @returns {number} Probability P(T ≤ x)
///
/// @example
/// ```javascript
/// studentTCdf(0.0, 10);   // 0.5
/// studentTCdf(2.228, 10); // ~0.975 (t-critical for 95% CI)
/// ```
#[wasm_bindgen(js_name = studentTCdf)]
pub fn student_t_cdf_js(x: f64, df: f64) -> Result<f64, JsValue> {
    try_student_t_cdf(x, df).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Inverse of Student's t-distribution CDF.
///
/// @param {number} p - Probability in (0, 1)
/// @param {number} df - Degrees of freedom (must be positive)
/// @returns {number} Quantile value
///
/// @example
/// ```javascript
/// studentTInvCdf(0.975, 10); // ~2.228 (t-critical for 95% CI)
/// ```
#[wasm_bindgen(js_name = studentTInvCdf)]
pub fn student_t_inv_cdf_js(p: f64, df: f64) -> Result<f64, JsValue> {
    try_student_t_inv_cdf(p, df).map_err(|e| JsValue::from_str(&e.to_string()))
}
