//! wasm-bindgen-test suite for `api::core::math`.
//!
//! Covers JsValue-based linear algebra, statistics, and summation wrappers.

#![cfg(target_arch = "wasm32")]

use finstack_wasm::api::core::math::*;
use wasm_bindgen_test::*;

// ---- Linear algebra ----

#[wasm_bindgen_test]
fn cholesky_decomposition_identity() {
    let matrix = serde_wasm_bindgen::to_value(&vec![vec![1.0, 0.0], vec![0.0, 1.0]]).unwrap();
    let result = cholesky_decomposition(matrix).unwrap();
    let rows: Vec<Vec<f64>> = serde_wasm_bindgen::from_value(result).unwrap();
    assert_eq!(rows.len(), 2);
    assert!((rows[0][0] - 1.0).abs() < 1e-10);
    assert!((rows[1][1] - 1.0).abs() < 1e-10);
}

#[wasm_bindgen_test]
fn cholesky_decomposition_spd() {
    let matrix = serde_wasm_bindgen::to_value(&vec![vec![4.0, 2.0], vec![2.0, 3.0]]).unwrap();
    let result = cholesky_decomposition(matrix).unwrap();
    let rows: Vec<Vec<f64>> = serde_wasm_bindgen::from_value(result).unwrap();
    assert!((rows[0][0] - 2.0).abs() < 1e-10);
}

#[wasm_bindgen_test]
fn cholesky_solve_identity_system() {
    let chol = serde_wasm_bindgen::to_value(&vec![vec![1.0, 0.0], vec![0.0, 1.0]]).unwrap();
    let b = serde_wasm_bindgen::to_value(&vec![3.0, 5.0]).unwrap();
    let result = cholesky_solve(chol, b).unwrap();
    let x: Vec<f64> = serde_wasm_bindgen::from_value(result).unwrap();
    assert!((x[0] - 3.0).abs() < 1e-10);
    assert!((x[1] - 5.0).abs() < 1e-10);
}

#[wasm_bindgen_test]
fn validate_correlation_matrix_valid() {
    let matrix = serde_wasm_bindgen::to_value(&vec![vec![1.0, 0.5], vec![0.5, 1.0]]).unwrap();
    validate_correlation_matrix(matrix).unwrap();
}

#[wasm_bindgen_test]
fn validate_correlation_matrix_invalid_diagonal() {
    let matrix = serde_wasm_bindgen::to_value(&vec![vec![0.9, 0.5], vec![0.5, 1.0]]).unwrap();
    assert!(validate_correlation_matrix(matrix).is_err());
}

// ---- Statistics ----

#[wasm_bindgen_test]
fn mean_of_known_values() {
    let data = serde_wasm_bindgen::to_value(&vec![1.0, 2.0, 3.0, 4.0, 5.0]).unwrap();
    let v = mean(data).unwrap();
    assert!((v - 3.0).abs() < 1e-10);
}

#[wasm_bindgen_test]
fn variance_of_known_values() {
    let data = serde_wasm_bindgen::to_value(&vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]).unwrap();
    let v = variance(data).unwrap();
    assert!(v > 0.0);
}

#[wasm_bindgen_test]
fn population_variance_positive() {
    let data = serde_wasm_bindgen::to_value(&vec![1.0, 2.0, 3.0]).unwrap();
    let v = population_variance(data).unwrap();
    assert!(v > 0.0);
}

#[wasm_bindgen_test]
fn correlation_perfect_positive() {
    let x = serde_wasm_bindgen::to_value(&vec![1.0, 2.0, 3.0, 4.0, 5.0]).unwrap();
    let y = serde_wasm_bindgen::to_value(&vec![2.0, 4.0, 6.0, 8.0, 10.0]).unwrap();
    let v = correlation(x, y).unwrap();
    assert!((v - 1.0).abs() < 1e-10);
}

#[wasm_bindgen_test]
fn covariance_positive_for_correlated() {
    let x = serde_wasm_bindgen::to_value(&vec![1.0, 2.0, 3.0, 4.0]).unwrap();
    let y = serde_wasm_bindgen::to_value(&vec![1.0, 2.0, 3.0, 4.0]).unwrap();
    let v = covariance(x, y).unwrap();
    assert!(v > 0.0);
}

#[wasm_bindgen_test]
fn quantile_median() {
    let data = serde_wasm_bindgen::to_value(&vec![1.0, 2.0, 3.0, 4.0, 5.0]).unwrap();
    let v = quantile(data, 0.5).unwrap();
    assert!((v - 3.0).abs() < 1e-10);
}

// ---- Summation ----

#[wasm_bindgen_test]
fn kahan_sum_accurate() {
    let data = serde_wasm_bindgen::to_value(&vec![1.0, 2.0, 3.0, 4.0]).unwrap();
    let v = kahan_sum(data).unwrap();
    assert!((v - 10.0).abs() < 1e-10);
}

#[wasm_bindgen_test]
fn neumaier_sum_accurate() {
    let data = serde_wasm_bindgen::to_value(&vec![1e16, 1.0, -1e16, 1.0]).unwrap();
    let v = neumaier_sum(data).unwrap();
    assert!((v - 2.0).abs() < 1e-10);
}
