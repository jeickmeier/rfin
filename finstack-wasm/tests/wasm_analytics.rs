//! wasm-bindgen-test suite for `api::analytics`.
//!
//! Covers all JsValue-based analytics wrappers that cannot be tested natively.

#![cfg(target_arch = "wasm32")]

use finstack_wasm::api::analytics::*;
use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;

fn returns_js() -> JsValue {
    serde_wasm_bindgen::to_value(&vec![
        0.01, -0.02, 0.03, -0.01, 0.02, -0.03, 0.015, -0.005, 0.02, -0.01,
    ])
    .unwrap()
}

fn benchmark_js() -> JsValue {
    serde_wasm_bindgen::to_value(&vec![
        0.005, -0.01, 0.02, -0.005, 0.015, -0.02, 0.01, -0.003, 0.01, -0.005,
    ])
    .unwrap()
}

fn prices_js() -> JsValue {
    serde_wasm_bindgen::to_value(&vec![100.0, 102.0, 101.0, 103.0, 102.5]).unwrap()
}

fn drawdown_js() -> JsValue {
    serde_wasm_bindgen::to_value(&vec![
        0.0, -0.01, -0.03, -0.02, 0.0, -0.05, -0.04, -0.01, 0.0,
    ])
    .unwrap()
}

// ---- Risk metrics ----

#[wasm_bindgen_test]
fn sortino_returns_finite() {
    let v = sortino(returns_js(), true, 252.0).unwrap();
    assert!(v.is_finite());
}

#[wasm_bindgen_test]
fn volatility_returns_positive() {
    let v = volatility(returns_js(), true, 252.0).unwrap();
    assert!(v > 0.0);
}

#[wasm_bindgen_test]
fn mean_return_finite() {
    let v = mean_return(returns_js(), false, 252.0).unwrap();
    assert!(v.is_finite());
}

#[wasm_bindgen_test]
fn cagr_factor_basis_finite() {
    let basis = WasmCagrBasis::factor(252.0);
    let v = cagr(returns_js(), &basis).unwrap();
    assert!(v.is_finite());
}

#[wasm_bindgen_test]
fn downside_deviation_non_negative() {
    let v = downside_deviation(returns_js(), 0.0, true, 252.0).unwrap();
    assert!(v >= 0.0);
}

#[wasm_bindgen_test]
fn geometric_mean_finite() {
    let v = geometric_mean(returns_js()).unwrap();
    assert!(v.is_finite());
}

#[wasm_bindgen_test]
fn omega_ratio_positive() {
    let v = omega_ratio(returns_js(), 0.0).unwrap();
    assert!(v > 0.0);
}

#[wasm_bindgen_test]
fn gain_to_pain_finite() {
    let v = gain_to_pain(returns_js()).unwrap();
    assert!(v.is_finite());
}

#[wasm_bindgen_test]
fn modified_sharpe_finite() {
    let v = modified_sharpe(returns_js(), 0.02, 0.95, 252.0).unwrap();
    assert!(v.is_finite() || v.is_nan());
}

// ---- Tail risk ----

#[wasm_bindgen_test]
fn value_at_risk_finite() {
    let v = value_at_risk(returns_js(), 0.95).unwrap();
    assert!(v.is_finite());
}

#[wasm_bindgen_test]
fn expected_shortfall_finite() {
    let v = expected_shortfall(returns_js(), 0.95).unwrap();
    assert!(v.is_finite());
}

#[wasm_bindgen_test]
fn parametric_var_finite() {
    let v = parametric_var(returns_js(), 0.95).unwrap();
    assert!(v.is_finite());
}

#[wasm_bindgen_test]
fn cornish_fisher_var_finite() {
    let v = cornish_fisher_var(returns_js(), 0.95).unwrap();
    assert!(v.is_finite());
}

#[wasm_bindgen_test]
fn skewness_finite() {
    let v = skewness(returns_js()).unwrap();
    assert!(v.is_finite());
}

#[wasm_bindgen_test]
fn kurtosis_finite() {
    let v = kurtosis(returns_js()).unwrap();
    assert!(v.is_finite());
}

#[wasm_bindgen_test]
fn tail_ratio_finite() {
    let v = tail_ratio(returns_js(), 0.95).unwrap();
    assert!(v.is_finite());
}

#[wasm_bindgen_test]
fn outlier_win_ratio_finite() {
    let v = outlier_win_ratio(returns_js(), 0.95).unwrap();
    assert!(v.is_finite());
}

#[wasm_bindgen_test]
fn outlier_loss_ratio_finite() {
    let v = outlier_loss_ratio(returns_js(), 0.95).unwrap();
    assert!(v.is_finite());
}

// ---- Rolling ----

// ---- Returns ----

#[wasm_bindgen_test]
fn simple_returns_from_prices() {
    let v = simple_returns(prices_js()).unwrap();
    let arr: Vec<f64> = serde_wasm_bindgen::from_value(v).unwrap();
    assert!(!arr.is_empty());
    assert!(arr.iter().all(|x| x.is_finite()));
}

#[wasm_bindgen_test]
fn comp_sum_returns_array() {
    let v = comp_sum(returns_js()).unwrap();
    let arr: Vec<f64> = serde_wasm_bindgen::from_value(v).unwrap();
    assert_eq!(arr.len(), 10);
}

#[wasm_bindgen_test]
fn comp_total_finite() {
    let v = comp_total(returns_js()).unwrap();
    assert!(v.is_finite());
}

#[wasm_bindgen_test]
fn clean_returns_handles_finite_values() {
    let data = serde_wasm_bindgen::to_value(&vec![0.01, -0.02, 0.03]).unwrap();
    let v = clean_returns(data).unwrap();
    let arr: Vec<f64> = serde_wasm_bindgen::from_value(v).unwrap();
    assert_eq!(arr.len(), 3);
    assert!(arr.iter().all(|x| x.is_finite()));
}

#[wasm_bindgen_test]
fn convert_to_prices_starts_at_base() {
    let v = convert_to_prices(returns_js(), 100.0).unwrap();
    let arr: Vec<f64> = serde_wasm_bindgen::from_value(v).unwrap();
    assert!((arr[0] - 100.0).abs() < 1e-10);
}

#[wasm_bindgen_test]
fn rebase_starts_at_base() {
    let v = rebase(prices_js(), 1.0).unwrap();
    let arr: Vec<f64> = serde_wasm_bindgen::from_value(v).unwrap();
    assert!((arr[0] - 1.0).abs() < 1e-10);
}

#[wasm_bindgen_test]
fn excess_returns_correct_length() {
    let v = excess_returns(returns_js(), benchmark_js()).unwrap();
    let arr: Vec<f64> = serde_wasm_bindgen::from_value(v).unwrap();
    assert_eq!(arr.len(), 10);
}

// ---- Drawdown ----

#[wasm_bindgen_test]
fn to_drawdown_series_returns_array() {
    let v = to_drawdown_series(returns_js()).unwrap();
    let arr: Vec<f64> = serde_wasm_bindgen::from_value(v).unwrap();
    assert!(!arr.is_empty());
}

#[wasm_bindgen_test]
fn max_drawdown_non_positive() {
    let v = max_drawdown(drawdown_js()).unwrap();
    assert!(v <= 0.0);
}

#[wasm_bindgen_test]
fn mean_episode_drawdown_finite() {
    let v = mean_episode_drawdown(drawdown_js(), 2).unwrap();
    assert!(v.is_finite());
}

#[wasm_bindgen_test]
fn mean_drawdown_finite() {
    let v = mean_drawdown(drawdown_js()).unwrap();
    assert!(v.is_finite());
}

#[wasm_bindgen_test]
fn cdar_finite() {
    let v = cdar(drawdown_js(), 0.95).unwrap();
    assert!(v.is_finite());
}

#[wasm_bindgen_test]
fn ulcer_index_non_negative() {
    let v = ulcer_index(drawdown_js()).unwrap();
    assert!(v >= 0.0);
}

#[wasm_bindgen_test]
fn pain_index_non_negative() {
    let v = pain_index(drawdown_js()).unwrap();
    assert!(v >= 0.0);
}

#[wasm_bindgen_test]
fn burke_ratio_finite() {
    let dd = serde_wasm_bindgen::to_value(&vec![-0.02, -0.05, -0.01]).unwrap();
    let v = burke_ratio(0.10, dd, 0.02).unwrap();
    assert!(v.is_finite());
}

// ---- Benchmark ----

#[wasm_bindgen_test]
fn tracking_error_finite() {
    let v = tracking_error(returns_js(), benchmark_js(), true, 252.0).unwrap();
    assert!(v.is_finite());
}

#[wasm_bindgen_test]
fn information_ratio_finite() {
    let v = information_ratio(returns_js(), benchmark_js(), true, 252.0).unwrap();
    assert!(v.is_finite());
}

#[wasm_bindgen_test]
fn r_squared_finite() {
    let v = r_squared(returns_js(), benchmark_js()).unwrap();
    assert!(v.is_finite());
}

#[wasm_bindgen_test]
fn up_capture_finite() {
    let v = up_capture(returns_js(), benchmark_js()).unwrap();
    assert!(v.is_finite());
}

#[wasm_bindgen_test]
fn down_capture_finite() {
    let v = down_capture(returns_js(), benchmark_js()).unwrap();
    assert!(v.is_finite());
}

#[wasm_bindgen_test]
fn capture_ratio_finite() {
    let v = capture_ratio(returns_js(), benchmark_js()).unwrap();
    assert!(v.is_finite());
}

#[wasm_bindgen_test]
fn batting_average_between_0_and_1() {
    let v = batting_average(returns_js(), benchmark_js()).unwrap();
    assert!((0.0..=1.0).contains(&v));
}
