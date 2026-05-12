//! wasm-bindgen-test suite for `api::analytics`.
//!
//! Covers the `Performance` panel facade and the two value-object inputs
//! (`CagrBasis`, `BenchmarkAlignmentPolicy`) — the entire WASM analytics
//! surface after the Performance-only consolidation.

#![cfg(target_arch = "wasm32")]

use finstack_wasm::api::analytics::{WasmBenchmarkAlignmentPolicy, WasmCagrBasis, WasmPerformance};
use js_sys::{Array, Float64Array};
use serde::Deserialize;
use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;

const API_INVARIANTS_FIXTURE: &str =
    include_str!("../../finstack/analytics/tests/fixtures/api_invariants_data.json");

#[derive(Deserialize)]
struct AnalyticsFixture {
    returns: Vec<f64>,
    benchmark: Vec<f64>,
    factors: Vec<Vec<f64>>,
    dates: Vec<String>,
}

fn fixture() -> AnalyticsFixture {
    serde_json::from_str(API_INVARIANTS_FIXTURE).unwrap()
}

fn to_js<T: serde::Serialize>(value: &T) -> JsValue {
    serde_wasm_bindgen::to_value(value).unwrap()
}

fn to_f64_matrix(rows: &[Vec<f64>]) -> JsValue {
    let array = Array::new_with_length(rows.len() as u32);
    for (i, row) in rows.iter().enumerate() {
        let row_value: JsValue = Float64Array::from(row.as_slice()).into();
        array.set(i as u32, row_value);
    }
    array.into()
}

/// Build a two-ticker Performance ("TARGET", "BENCH") from the fixture's
/// return + benchmark series, with TARGET as the benchmark for greeks etc.
fn build_perf() -> WasmPerformance {
    let fx = fixture();
    // returns matrix is column-major in the binding: returns[i] is the column
    // for ticker i. Two columns => target series, benchmark series.
    let returns = vec![fx.returns.clone(), fx.benchmark.clone()];
    let names = vec!["TARGET".to_string(), "BENCH".to_string()];
    WasmPerformance::from_returns(
        to_js(&fx.dates),
        to_f64_matrix(&returns),
        to_js(&names),
        Some("BENCH".to_string()),
        Some("daily".to_string()),
    )
    .unwrap()
}

// ---- Construction ----

#[wasm_bindgen_test]
fn from_returns_exposes_active_dates() {
    let perf = build_perf();
    let dates = perf.dates();
    assert_eq!(dates.len(), fixture().dates.len());
}

#[wasm_bindgen_test]
fn ticker_names_round_trip() {
    let perf = build_perf();
    let names: Vec<String> = serde_wasm_bindgen::from_value(perf.ticker_names().unwrap()).unwrap();
    assert_eq!(names, vec!["TARGET", "BENCH"]);
    assert_eq!(perf.benchmark_idx(), 1);
    assert_eq!(perf.freq(), "daily");
}

// ---- Scalar metrics ----

#[wasm_bindgen_test]
fn cagr_returns_per_ticker_vec() {
    let perf = build_perf();
    let values: Vec<f64> = serde_wasm_bindgen::from_value(perf.cagr().unwrap()).unwrap();
    assert_eq!(values.len(), 2);
    assert!(values.iter().all(|v| v.is_finite()));
}

#[wasm_bindgen_test]
fn sharpe_sortino_volatility_finite() {
    let perf = build_perf();
    for raw in [
        perf.sharpe(Some(0.0)).unwrap(),
        perf.sortino(Some(0.0)).unwrap(),
        perf.volatility(Some(true)).unwrap(),
        perf.mean_return(Some(true)).unwrap(),
    ] {
        let v: Vec<f64> = serde_wasm_bindgen::from_value(raw).unwrap();
        assert_eq!(v.len(), 2);
        assert!(v.iter().all(|x| x.is_finite()));
    }
}

#[wasm_bindgen_test]
fn tail_metrics_finite() {
    let perf = build_perf();
    for raw in [
        perf.value_at_risk(Some(0.95)).unwrap(),
        perf.expected_shortfall(Some(0.95)).unwrap(),
        perf.parametric_var(Some(0.95)).unwrap(),
        perf.cornish_fisher_var(Some(0.95)).unwrap(),
        perf.tail_ratio(Some(0.95)).unwrap(),
    ] {
        let v: Vec<f64> = serde_wasm_bindgen::from_value(raw).unwrap();
        assert_eq!(v.len(), 2);
        assert!(v.iter().all(|x| x.is_finite()));
    }
}

#[wasm_bindgen_test]
fn drawdown_scalars_match_panel_width() {
    let perf = build_perf();
    let max_dd: Vec<f64> = serde_wasm_bindgen::from_value(perf.max_drawdown().unwrap()).unwrap();
    assert_eq!(max_dd.len(), 2);
    assert!(max_dd.iter().all(|v| v <= &0.0));

    let calmar: Vec<f64> = serde_wasm_bindgen::from_value(perf.calmar().unwrap()).unwrap();
    assert_eq!(calmar.len(), 2);
}

// ---- Vector outputs ----

#[wasm_bindgen_test]
fn cumulative_and_drawdown_series_are_per_ticker_panels() {
    let perf = build_perf();
    let cum: Vec<Vec<f64>> =
        serde_wasm_bindgen::from_value(perf.cumulative_returns().unwrap()).unwrap();
    assert_eq!(cum.len(), 2);
    assert_eq!(cum[0].len(), fixture().dates.len());

    let dd: Vec<Vec<f64>> =
        serde_wasm_bindgen::from_value(perf.drawdown_series().unwrap()).unwrap();
    assert_eq!(dd.len(), 2);
    assert!(dd[0].iter().all(|v| v <= &0.0));
}

#[wasm_bindgen_test]
fn correlation_matrix_is_square() {
    let perf = build_perf();
    let mat: Vec<Vec<f64>> =
        serde_wasm_bindgen::from_value(perf.correlation_matrix().unwrap()).unwrap();
    assert_eq!(mat.len(), 2);
    assert_eq!(mat[0].len(), 2);
    assert!((mat[0][0] - 1.0).abs() < 1e-12);
}

// ---- Benchmark / greeks ----

#[wasm_bindgen_test]
fn beta_and_greeks_return_per_ticker_structs() {
    let perf = build_perf();
    let betas: serde_json::Value = serde_wasm_bindgen::from_value(perf.beta().unwrap()).unwrap();
    assert_eq!(betas.as_array().unwrap().len(), 2);

    let greeks: serde_json::Value = serde_wasm_bindgen::from_value(perf.greeks().unwrap()).unwrap();
    assert_eq!(greeks.as_array().unwrap().len(), 2);
}

#[wasm_bindgen_test]
fn rolling_greeks_emit_dates_alphas_betas() {
    let perf = build_perf();
    let raw = perf.rolling_greeks(0, Some(5)).unwrap();
    let value: serde_json::Value = serde_wasm_bindgen::from_value(raw).unwrap();
    let dates = value["dates"].as_array().unwrap();
    let alphas = value["alphas"].as_array().unwrap();
    let betas = value["betas"].as_array().unwrap();
    assert_eq!(dates.len(), alphas.len());
    assert_eq!(alphas.len(), betas.len());
    assert!(!dates.is_empty());
}

#[wasm_bindgen_test]
fn rolling_returns_match_dated_series_shape() {
    let perf = build_perf();
    let raw = perf.rolling_returns(0, 3).unwrap();
    let value: serde_json::Value = serde_wasm_bindgen::from_value(raw).unwrap();
    let values = value["values"].as_array().unwrap();
    let dates = value["dates"].as_array().unwrap();
    assert_eq!(values.len(), dates.len());
    assert!(!values.is_empty());
}

#[wasm_bindgen_test]
fn multi_factor_greeks_resolves_to_struct() {
    let perf = build_perf();
    let fx = fixture();
    let raw = perf
        .multi_factor_greeks(0, to_f64_matrix(&fx.factors))
        .unwrap();
    let value: serde_json::Value = serde_wasm_bindgen::from_value(raw).unwrap();
    assert!(value["alpha"].is_number());
    let betas = value["betas"].as_array().unwrap();
    assert_eq!(betas.len(), fx.factors.len());
}

// ---- Lookback & aggregation ----

#[wasm_bindgen_test]
fn lookback_returns_emit_mtd_qtd_ytd() {
    let perf = build_perf();
    let raw = perf.lookback_returns("2025-01-12", None).unwrap();
    let value: serde_json::Value = serde_wasm_bindgen::from_value(raw).unwrap();
    assert_eq!(value["mtd"].as_array().unwrap().len(), 2);
    assert_eq!(value["qtd"].as_array().unwrap().len(), 2);
    assert_eq!(value["ytd"].as_array().unwrap().len(), 2);
}

#[wasm_bindgen_test]
fn period_stats_emit_win_rate() {
    let perf = build_perf();
    let raw = perf
        .period_stats(0, Some("weekly".to_string()), None)
        .unwrap();
    let value: serde_json::Value = serde_wasm_bindgen::from_value(raw).unwrap();
    assert!(value["win_rate"].as_f64().is_some());
}

// ---- Mutators ----

#[wasm_bindgen_test]
fn reset_date_range_narrows_active_dates() {
    let mut perf = build_perf();
    perf.reset_date_range("2025-01-05", "2025-01-10").unwrap();
    let dates = perf.dates();
    assert!(dates.first().map(String::as_str) == Some("2025-01-05"));
    assert!(dates.last().map(String::as_str) == Some("2025-01-10"));
}

#[wasm_bindgen_test]
fn reset_bench_ticker_updates_index() {
    let mut perf = build_perf();
    perf.reset_bench_ticker("TARGET").unwrap();
    assert_eq!(perf.benchmark_idx(), 0);
}

// ---- Value-object inputs ----

#[wasm_bindgen_test]
fn cagr_basis_constructors_work() {
    // Constructors should not panic; they're consumed by future Performance
    // overloads, but the WASM facade keeps them exposed for symmetry.
    let _factor = WasmCagrBasis::factor(252.0);
    let _dates = WasmCagrBasis::dates("2024-01-01", "2024-12-31", None).unwrap();
    let _err =
        WasmCagrBasis::dates("2024-01-01", "2023-12-31", Some("act365_25".to_string())).unwrap();
    let _z = WasmBenchmarkAlignmentPolicy::zero_on_missing();
    let _e = WasmBenchmarkAlignmentPolicy::error_on_missing();
}
