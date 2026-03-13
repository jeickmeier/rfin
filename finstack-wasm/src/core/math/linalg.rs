//! Linear algebra utilities for WASM bindings.

use finstack_core::math::linalg::{
    apply_correlation, build_correlation_matrix, cholesky_correlation, cholesky_decomposition,
    validate_correlation_matrix,
};
use wasm_bindgen::prelude::*;

/// Cholesky decomposition of a correlation/covariance matrix.
///
/// Computes L such that Σ = L L^T, where Σ is the input matrix.
/// Used for generating correlated random variables in Monte Carlo simulations.
///
/// @param {Float64Array} matrix - Symmetric positive definite matrix (n×n, row-major)
/// @param {number} n - Matrix dimension
/// @returns {Float64Array} Lower triangular Cholesky factor L (n×n, row-major)
/// @throws {Error} If matrix is not positive definite or dimensions mismatch
///
/// @example
/// ```javascript
/// // Correlation matrix: [[1.0, 0.5], [0.5, 1.0]]
/// const corr = new Float64Array([1.0, 0.5, 0.5, 1.0]);
/// const chol = choleskyDecomposition(corr, 2);
/// // chol = [1.0, 0.0, 0.5, 0.866...]
/// ```
#[wasm_bindgen(js_name = choleskyDecomposition)]
pub fn cholesky_decomposition_js(matrix: &[f64], n: usize) -> Result<Vec<f64>, JsValue> {
    cholesky_decomposition(matrix, n).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Pivoted Cholesky factorisation of a correlation or covariance matrix.
///
/// Uses complete diagonal pivoting with relative tolerance, making it robust for
/// near-singular and positive-semidefinite correlation matrices. The returned factor
/// is in the **original variable ordering**.
///
/// @param {Float64Array} matrix - Symmetric PSD matrix (n×n, row-major)
/// @param {number} n - Matrix dimension
/// @returns {{ factor: Float64Array, effectiveRank: number }} Factor in original order plus rank
/// @throws {Error} If the matrix is indefinite
///
/// @example
/// ```javascript
/// const corr = new Float64Array([1.0, 0.9999999, 0.9999999, 1.0]);
/// const { factor, effectiveRank } = choleskyCorrelation(corr, 2);
/// // effectiveRank may be 1 or 2 depending on numerical precision
/// ```
#[wasm_bindgen(js_name = choleskyCorrelation)]
pub fn cholesky_correlation_js(matrix: &[f64], n: usize) -> Result<JsValue, JsValue> {
    let factor = cholesky_correlation(matrix, n).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let result = js_sys::Object::new();
    let flat: Vec<f64> = factor.factor_matrix().to_vec();
    let js_array = js_sys::Float64Array::from(flat.as_slice());
    js_sys::Reflect::set(&result, &JsValue::from_str("factor"), &js_array)
        .map_err(|e| JsValue::from_str(&format!("reflect set failed: {e:?}")))?;
    js_sys::Reflect::set(
        &result,
        &JsValue::from_str("effectiveRank"),
        &JsValue::from_f64(factor.effective_rank() as f64),
    )
    .map_err(|e| JsValue::from_str(&format!("reflect set failed: {e:?}")))?;
    Ok(result.into())
}

/// Apply correlation via Cholesky factor to independent shocks.
///
/// Transforms independent N(0,1) shocks into correlated shocks using L from Cholesky decomposition.
///
/// @param {Float64Array} chol - Lower triangular Cholesky factor (n×n, row-major)
/// @param {Float64Array} independent - Independent shocks (length n)
/// @returns {Float64Array} Correlated shocks (length n)
///
/// @example
/// ```javascript
/// const corr = new Float64Array([1.0, 0.5, 0.5, 1.0]);
/// const chol = choleskyDecomposition(corr, 2);
/// const z = new Float64Array([1.0, 0.0]); // Independent N(0,1) shocks
/// const zCorr = applyCorrelation(chol, z);
/// // zCorr now contains correlated shocks
/// ```
/// # Panics
///
/// Cannot panic in practice: the dimension check above ensures `chol` and `correlated` are
/// correctly sized before calling `apply_correlation`.
#[allow(clippy::expect_used)]
#[wasm_bindgen(js_name = applyCorrelation)]
pub fn apply_correlation_js(chol: &[f64], independent: &[f64]) -> Result<Vec<f64>, JsValue> {
    let n = independent.len();
    if chol.len() != n * n {
        return Err(JsValue::from_str(&format!(
            "Cholesky factor must be {}x{} = {} elements, got {}",
            n,
            n,
            n * n,
            chol.len()
        )));
    }

    let mut correlated = vec![0.0; n];
    // Dimension check already performed above, so this cannot fail.
    apply_correlation(chol, independent, &mut correlated)
        .expect("apply_correlation: dimensions pre-validated");
    Ok(correlated)
}

/// Build a correlation matrix from correlation pairs.
///
/// Creates a symmetric correlation matrix with 1.0 on diagonal and specified correlations.
///
/// @param {number} n - Matrix dimension
/// @param {Array} correlations - Array of [i, j, correlation] tuples
/// @returns {Float64Array} Symmetric correlation matrix (n×n, row-major)
///
/// @example
/// ```javascript
/// // Create 3x3 correlation matrix with ρ(0,1)=0.5 and ρ(1,2)=0.3
/// const correlations = [[0, 1, 0.5], [1, 2, 0.3]];
/// const matrix = buildCorrelationMatrix(3, correlations);
/// ```
#[wasm_bindgen(js_name = buildCorrelationMatrix)]
pub fn build_correlation_matrix_js(n: usize, correlations: JsValue) -> Result<Vec<f64>, JsValue> {
    let corr_array: Vec<(usize, usize, f64)> = serde_wasm_bindgen::from_value(correlations)
        .map_err(|e| JsValue::from_str(&format!("Invalid correlations format: {}", e)))?;

    build_correlation_matrix(n, &corr_array).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Validate that a matrix is a valid correlation matrix.
///
/// Checks: diagonal elements are 1.0, off-diagonal in [-1, 1], symmetric, positive semi-definite.
///
/// @param {Float64Array} matrix - Matrix to validate (n×n, row-major)
/// @param {number} n - Matrix dimension
/// @returns {boolean} True if valid correlation matrix
///
/// @example
/// ```javascript
/// const valid = new Float64Array([1.0, 0.5, 0.5, 1.0]);
/// const isValid = validateCorrelationMatrix(valid, 2); // true
///
/// const invalid = new Float64Array([1.0, 1.5, 1.5, 1.0]); // correlation > 1
/// const isInvalid = validateCorrelationMatrix(invalid, 2); // false
/// ```
#[wasm_bindgen(js_name = validateCorrelationMatrix)]
pub fn validate_correlation_matrix_js(matrix: &[f64], n: usize) -> bool {
    if matrix.len() != n * n {
        return false;
    }
    validate_correlation_matrix(matrix, n).is_ok()
}
