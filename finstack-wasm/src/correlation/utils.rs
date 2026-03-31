//! WASM bindings for correlation-specific utilities.
//!
//! Provides correlation matrix validation (throwing variant) and Cholesky
//! decomposition from the `finstack-correlation` crate.
//!
//! Note: `CorrelatedBernoulli`, `correlationBounds`, and `jointProbabilities`
//! are already exposed via the `core::math` module and are not duplicated here.

use finstack_correlation::{cholesky_decompose, validate_correlation_matrix};
use wasm_bindgen::prelude::*;

use crate::core::error::{js_error_with_kind, ErrorKind};

/// Validate a correlation matrix (throwing variant).
///
/// Checks symmetry, unit diagonal, bounds, and positive semi-definiteness.
/// Unlike the `validateCorrelationMatrix` from the math module (which returns
/// a boolean), this version throws a `ValidationError` with a descriptive
/// message on failure.
///
/// @param matrix - Flattened row-major correlation matrix.
/// @param n - Number of factors (matrix should be n x n).
/// @throws ValidationError if the matrix is invalid.
#[wasm_bindgen(js_name = validateCorrelationMatrixStrict)]
pub fn validate_correlation_matrix_strict(matrix: Vec<f64>, n: usize) -> Result<(), JsValue> {
    validate_correlation_matrix(&matrix, n).map_err(|e| {
        js_error_with_kind(
            ErrorKind::Validation,
            format!("Invalid correlation matrix: {e}"),
        )
    })
}

/// Cholesky decomposition via the correlation crate.
///
/// Uses diagonal pivoting to handle near-singular matrices. Produces the
/// lower-triangular factor used by `MultiFactorModel` for correlated
/// factor generation.
///
/// @param matrix - Flattened row-major correlation matrix.
/// @param n - Matrix dimension.
/// @returns Flattened lower-triangular Cholesky factor (row-major).
/// @throws ValidationError if the matrix is not positive semi-definite.
#[wasm_bindgen(js_name = choleskyDecomposeCorrelation)]
pub fn cholesky_decompose_correlation(matrix: Vec<f64>, n: usize) -> Result<Vec<f64>, JsValue> {
    let factor = cholesky_decompose(&matrix, n).map_err(|e| {
        js_error_with_kind(
            ErrorKind::Validation,
            format!("Cholesky decomposition failed: {e}"),
        )
    })?;
    Ok(factor.factor_matrix().to_vec())
}
