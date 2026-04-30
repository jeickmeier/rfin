//! wasm-bindgen-test suite for `api::analytics`.
//!
//! Covers all JsValue-based analytics wrappers that cannot be tested natively.

#![cfg(target_arch = "wasm32")]

use finstack_wasm::api::analytics::*;
use finstack_wasm::api::statements_analytics::compute_multiple;
use js_sys::{Array, Float64Array};
use serde::Deserialize;
use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;

const API_INVARIANTS_FIXTURE: &str =
    include_str!("../../finstack/analytics/tests/fixtures/api_invariants_data.json");

#[derive(Deserialize)]
struct AnalyticsParityFixture {
    returns: Vec<f64>,
    benchmark: Vec<f64>,
    factors: Vec<Vec<f64>>,
    dates: Vec<String>,
    expected: AnalyticsParityExpected,
}

#[derive(Deserialize)]
struct AnalyticsParityExpected {
    cagr_factor: f64,
    sharpe: f64,
    sortino: f64,
    value_at_risk: f64,
    expected_shortfall: f64,
    rolling_greeks: ExpectedRollingGreeks,
    multi_factor_greeks: ExpectedMultiFactorGreeks,
}

#[derive(Deserialize)]
struct ExpectedRollingGreeks {
    alphas: Vec<f64>,
    betas: Vec<f64>,
}

#[derive(Deserialize)]
struct ExpectedMultiFactorGreeks {
    alpha: f64,
    betas: Vec<f64>,
    r_squared: f64,
    adjusted_r_squared: f64,
    residual_vol: f64,
}

#[derive(Deserialize)]
struct WasmRollingGreeksResult {
    alphas: Vec<f64>,
    betas: Vec<f64>,
}

#[derive(Deserialize)]
struct WasmMultiFactorResult {
    alpha: f64,
    betas: Vec<f64>,
    r_squared: f64,
    adjusted_r_squared: f64,
    residual_vol: f64,
}

fn api_invariants_fixture() -> AnalyticsParityFixture {
    serde_json::from_str(API_INVARIANTS_FIXTURE).unwrap()
}

fn to_js<T: serde::Serialize>(value: &T) -> JsValue {
    serde_wasm_bindgen::to_value(value).unwrap()
}

fn to_f64_array(value: &[f64]) -> JsValue {
    Float64Array::from(value).into()
}

fn to_f64_matrix(rows: &[Vec<f64>]) -> JsValue {
    let array = Array::new_with_length(rows.len() as u32);
    for (i, row) in rows.iter().enumerate() {
        let row_value: JsValue = Float64Array::from(row.as_slice()).into();
        array.set(i as u32, row_value);
    }
    array.into()
}

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1.0e-12,
        "actual={actual}, expected={expected}"
    );
}

fn assert_vec_close(actual: &[f64], expected: &[f64]) {
    assert_eq!(actual.len(), expected.len());
    for (&actual, &expected) in actual.iter().zip(expected.iter()) {
        assert_close(actual, expected);
    }
}

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

fn dates_js() -> JsValue {
    serde_wasm_bindgen::to_value(&vec![
        "2025-01-01",
        "2025-01-02",
        "2025-01-03",
        "2025-01-04",
        "2025-01-05",
        "2025-01-06",
        "2025-01-07",
        "2025-01-08",
        "2025-01-09",
        "2025-01-10",
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
    let v = sortino(returns_js(), true, 252.0, 0.0).unwrap();
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
fn cagr_date_basis_rejects_non_positive_span() {
    let basis = WasmCagrBasis::dates("2024-01-01", "2024-01-01", None).unwrap();
    assert!(cagr(returns_js(), &basis).is_err());
}

#[wasm_bindgen_test]
fn analytics_matches_shared_parity_fixture() {
    let fixture = api_invariants_fixture();
    let expected = &fixture.expected;

    let basis = WasmCagrBasis::factor(252.0);
    assert_close(
        cagr(to_js(&fixture.returns), &basis).unwrap(),
        expected.cagr_factor,
    );
    assert_close(sharpe(0.12, 0.18, 0.02), expected.sharpe);
    assert_close(
        sortino(to_js(&fixture.returns), true, 252.0, 0.0).unwrap(),
        expected.sortino,
    );
    assert_close(
        value_at_risk(to_js(&fixture.returns), 0.95).unwrap(),
        expected.value_at_risk,
    );
    assert_close(
        expected_shortfall(to_js(&fixture.returns), 0.95).unwrap(),
        expected.expected_shortfall,
    );

    let rolling: WasmRollingGreeksResult = serde_wasm_bindgen::from_value(
        rolling_greeks(
            to_js(&fixture.returns),
            to_js(&fixture.benchmark),
            to_js(&fixture.dates),
            5,
            252.0,
        )
        .unwrap(),
    )
    .unwrap();
    assert_vec_close(&rolling.alphas, &expected.rolling_greeks.alphas);
    assert_vec_close(&rolling.betas, &expected.rolling_greeks.betas);

    let multi: WasmMultiFactorResult = serde_wasm_bindgen::from_value(
        multi_factor_greeks(to_js(&fixture.returns), to_js(&fixture.factors), 252.0).unwrap(),
    )
    .unwrap();
    assert_close(multi.alpha, expected.multi_factor_greeks.alpha);
    assert_vec_close(&multi.betas, &expected.multi_factor_greeks.betas);
    assert_close(multi.r_squared, expected.multi_factor_greeks.r_squared);
    assert_close(
        multi.adjusted_r_squared,
        expected.multi_factor_greeks.adjusted_r_squared,
    );
    assert_close(
        multi.residual_vol,
        expected.multi_factor_greeks.residual_vol,
    );
}

#[wasm_bindgen_test]
fn typed_array_inputs_match_shared_parity_fixture() {
    let fixture = api_invariants_fixture();
    let expected = &fixture.expected;

    assert_close(
        value_at_risk(to_f64_array(&fixture.returns), 0.95).unwrap(),
        expected.value_at_risk,
    );
    assert_close(
        expected_shortfall(to_f64_array(&fixture.returns), 0.95).unwrap(),
        expected.expected_shortfall,
    );

    let rolling: WasmRollingGreeksResult = serde_wasm_bindgen::from_value(
        rolling_greeks(
            to_f64_array(&fixture.returns),
            to_f64_array(&fixture.benchmark),
            to_js(&fixture.dates),
            5,
            252.0,
        )
        .unwrap(),
    )
    .unwrap();
    assert_vec_close(&rolling.alphas, &expected.rolling_greeks.alphas);
    assert_vec_close(&rolling.betas, &expected.rolling_greeks.betas);

    let multi: WasmMultiFactorResult = serde_wasm_bindgen::from_value(
        multi_factor_greeks(
            to_f64_array(&fixture.returns),
            to_f64_matrix(&fixture.factors),
            252.0,
        )
        .unwrap(),
    )
    .unwrap();
    assert_close(multi.alpha, expected.multi_factor_greeks.alpha);
    assert_vec_close(&multi.betas, &expected.multi_factor_greeks.betas);

    let prices = to_f64_array(&[100.0, 102.0, 101.0, 103.0]);
    let simple: Vec<f64> = serde_wasm_bindgen::from_value(simple_returns(prices).unwrap()).unwrap();
    assert_eq!(simple.len(), 4);
    assert!(simple.iter().all(|x| x.is_finite()));
}

#[wasm_bindgen_test]
fn multi_factor_greeks_rejects_non_finite_factor_inputs() {
    let returns = to_js(&vec![0.01, -0.02, 0.03, -0.01]);
    let factors = to_js(&vec![vec![0.01, f64::NAN, 0.02, -0.01]]);

    assert!(multi_factor_greeks(returns, factors, 252.0).is_err());
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

#[wasm_bindgen_test]
fn estimate_ruin_returns_struct_like_object() {
    let definition = WasmRuinDefinition::wealth_floor(0.8);
    let model = WasmRuinModel::new(Some(30), Some(1_000), Some(3), Some(7), Some(0.95));
    let estimate = estimate_ruin(returns_js(), &definition, &model).unwrap();
    let json: serde_json::Value = serde_wasm_bindgen::from_value(estimate).unwrap();
    assert!(json["probability"].as_f64().is_some());
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
    let v = parametric_var(returns_js(), 0.95, Some(252.0)).unwrap();
    assert!(v.is_finite());
}

#[wasm_bindgen_test]
fn cornish_fisher_var_finite() {
    let v = cornish_fisher_var(returns_js(), 0.95, Some(252.0)).unwrap();
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
    let v = excess_returns(returns_js(), benchmark_js(), None).unwrap();
    let arr: Vec<f64> = serde_wasm_bindgen::from_value(v).unwrap();
    assert_eq!(arr.len(), 10);
}

#[wasm_bindgen_test]
fn excess_returns_supports_optional_nperiods() {
    let returns = serde_wasm_bindgen::to_value(&vec![0.02, 0.01]).unwrap();
    let rf = serde_wasm_bindgen::to_value(&vec![0.12, 0.12]).unwrap();
    let v = excess_returns(returns, rf, Some(12.0)).unwrap();
    let arr: Vec<f64> = serde_wasm_bindgen::from_value(v).unwrap();
    assert_eq!(arr.len(), 2);
    assert!(arr[0].is_finite());
    assert!(arr[1].is_finite());
}

// ---- Aggregation ----

#[wasm_bindgen_test]
fn group_by_period_accepts_dates_first() {
    let grouped = group_by_period(dates_js(), returns_js(), "weekly").unwrap();
    let tuples: Vec<(String, f64)> = serde_wasm_bindgen::from_value(grouped).unwrap();
    assert!(!tuples.is_empty());
}

#[wasm_bindgen_test]
fn period_stats_accepts_flat_returns() {
    let stats = period_stats(returns_js()).unwrap();
    let json: serde_json::Value = serde_wasm_bindgen::from_value(stats).unwrap();
    assert!(json["win_rate"].as_f64().is_some());
}

#[wasm_bindgen_test]
fn align_benchmark_zero_fills_missing_dates() {
    let bench_returns = serde_wasm_bindgen::to_value(&vec![0.01, 0.03]).unwrap();
    let bench_dates = serde_wasm_bindgen::to_value(&vec!["2025-01-01", "2025-01-03"]).unwrap();
    let target_dates =
        serde_wasm_bindgen::to_value(&vec!["2025-01-01", "2025-01-02", "2025-01-03"]).unwrap();
    let policy = WasmBenchmarkAlignmentPolicy::zero_on_missing();

    let aligned = align_benchmark(bench_returns, bench_dates, target_dates, &policy).unwrap();
    let values: Vec<f64> = serde_wasm_bindgen::from_value(aligned).unwrap();
    assert_eq!(values, vec![0.01, 0.0, 0.03]);
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

#[wasm_bindgen_test]
fn rolling_greeks_returns_dates_alphas_and_betas() {
    let v = rolling_greeks(returns_js(), benchmark_js(), dates_js(), 5, 252.0).unwrap();
    let value: serde_json::Value = serde_wasm_bindgen::from_value(v).unwrap();
    let dates = value["dates"].as_array().unwrap();
    let alphas = value["alphas"].as_array().unwrap();
    let betas = value["betas"].as_array().unwrap();
    assert_eq!(dates.len(), 6);
    assert_eq!(alphas.len(), 6);
    assert_eq!(betas.len(), 6);
}

#[wasm_bindgen_test]
fn compute_multiple_uses_company_metrics_shape() {
    let metrics = serde_wasm_bindgen::to_value(&serde_json::json!({
        "enterprise_value": 8500.0,
        "ebitda": 1000.0,
    }))
    .unwrap();
    let v = compute_multiple(metrics, "ev_ebitda").unwrap();
    let parsed: Option<f64> = serde_wasm_bindgen::from_value(v).unwrap();
    assert_eq!(parsed, Some(8.5));
}

#[wasm_bindgen_test]
fn rolling_var_forecasts_returns_two_aligned_series() {
    let v = rolling_var_forecasts(returns_js(), 5, 0.99, "Historical").unwrap();
    let parsed: (Vec<f64>, Vec<f64>) = serde_wasm_bindgen::from_value(v).unwrap();
    assert_eq!(parsed.0.len(), 5);
    assert_eq!(parsed.1.len(), 5);
}

#[wasm_bindgen_test]
fn classify_breaches_returns_dense_boolean_series() {
    let forecasts = serde_wasm_bindgen::to_value(&vec![-0.02, -0.02]).unwrap();
    let realized = serde_wasm_bindgen::to_value(&vec![-0.01, -0.03]).unwrap();
    let v = classify_breaches(forecasts, realized).unwrap();
    let parsed: Vec<bool> = serde_wasm_bindgen::from_value(v).unwrap();
    assert_eq!(parsed, vec![false, true]);
}

#[wasm_bindgen_test]
fn compare_var_backtests_returns_two_models() {
    let models = serde_wasm_bindgen::to_value(&serde_json::json!([
        ["Historical", [-0.02, -0.02, -0.02]],
        ["Parametric", [-0.015, -0.015, -0.015]]
    ]))
    .unwrap();
    let realized = serde_wasm_bindgen::to_value(&vec![-0.01, -0.03, -0.01]).unwrap();
    let v = compare_var_backtests(models, realized, 0.99, 250).unwrap();
    let value: serde_json::Value = serde_wasm_bindgen::from_value(v).unwrap();
    let results = value["results"].as_array().unwrap();
    assert_eq!(results.len(), 2);
}

#[wasm_bindgen_test]
fn pnl_explanation_returns_struct() {
    let hypothetical = serde_wasm_bindgen::to_value(&vec![100.0, 110.0, 105.0]).unwrap();
    let risk_theoretical = serde_wasm_bindgen::to_value(&vec![99.0, 109.0, 104.0]).unwrap();
    let var = serde_wasm_bindgen::to_value(&vec![10.0, 10.0, 10.0]).unwrap();
    let v = pnl_explanation(hypothetical, risk_theoretical, var).unwrap();
    let value: serde_json::Value = serde_wasm_bindgen::from_value(v).unwrap();
    assert_eq!(value["n"], 3);
    assert_eq!(value["mean_abs_unexplained"], 1.0);
    assert!(value["aggregate_explanation_ratio"].is_number());
}

#[wasm_bindgen_test]
fn lookback_selectors_return_ranges() {
    let range: [usize; 2] =
        serde_wasm_bindgen::from_value(mtd_select(dates_js(), "2025-01-10", 0).unwrap()).unwrap();
    assert_eq!(range, [0, 10]);

    let ytd: [usize; 2] =
        serde_wasm_bindgen::from_value(ytd_select(dates_js(), "2025-01-10", 0).unwrap()).unwrap();
    assert_eq!(ytd, [0, 10]);
}
