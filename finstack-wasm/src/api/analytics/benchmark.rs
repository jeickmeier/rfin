use crate::utils::to_js_err;
use finstack_analytics as fa;
use wasm_bindgen::prelude::*;

use super::support::parse_iso_dates;

/// Policy for handling missing dates during benchmark alignment.
#[wasm_bindgen(js_name = BenchmarkAlignmentPolicy)]
pub struct WasmBenchmarkAlignmentPolicy {
    inner: fa::benchmark::BenchmarkAlignmentPolicy,
}

#[wasm_bindgen(js_class = BenchmarkAlignmentPolicy)]
impl WasmBenchmarkAlignmentPolicy {
    /// Fill missing benchmark dates with zero returns.
    #[wasm_bindgen(js_name = zeroOnMissing)]
    pub fn zero_on_missing() -> Self {
        Self {
            inner: fa::benchmark::BenchmarkAlignmentPolicy::ZeroReturnOnMissingDates,
        }
    }

    /// Raise an error if benchmark dates don't cover all target dates.
    #[wasm_bindgen(js_name = errorOnMissing)]
    pub fn error_on_missing() -> Self {
        Self {
            inner: fa::benchmark::BenchmarkAlignmentPolicy::ErrorOnMissingDates,
        }
    }
}

/// Align benchmark returns to target dates using an explicit missing-date policy.
#[wasm_bindgen(js_name = alignBenchmark)]
pub fn align_benchmark(
    bench_returns: JsValue,
    bench_dates: JsValue,
    target_dates: JsValue,
    policy: &WasmBenchmarkAlignmentPolicy,
) -> Result<JsValue, JsValue> {
    let returns: Vec<f64> = serde_wasm_bindgen::from_value(bench_returns).map_err(to_js_err)?;
    let bench_date_strs: Vec<String> =
        serde_wasm_bindgen::from_value(bench_dates).map_err(to_js_err)?;
    let target_date_strs: Vec<String> =
        serde_wasm_bindgen::from_value(target_dates).map_err(to_js_err)?;
    let bench = parse_iso_dates(&bench_date_strs)?;
    let target = parse_iso_dates(&target_date_strs)?;
    let aligned = fa::benchmark::align_benchmark(&returns, &bench, &target, policy.inner)
        .map_err(to_js_err)?;
    serde_wasm_bindgen::to_value(&aligned).map_err(to_js_err)
}

/// Annualized tracking error between portfolio and benchmark.
#[wasm_bindgen(js_name = trackingError)]
pub fn tracking_error(
    returns: JsValue,
    benchmark: JsValue,
    annualize: bool,
    ann_factor: f64,
) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    let b: Vec<f64> = serde_wasm_bindgen::from_value(benchmark).map_err(to_js_err)?;
    Ok(fa::benchmark::tracking_error(&r, &b, annualize, ann_factor))
}

/// Information ratio (excess return per unit of tracking error).
#[wasm_bindgen(js_name = informationRatio)]
pub fn information_ratio(
    returns: JsValue,
    benchmark: JsValue,
    annualize: bool,
    ann_factor: f64,
) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    let b: Vec<f64> = serde_wasm_bindgen::from_value(benchmark).map_err(to_js_err)?;
    Ok(fa::benchmark::information_ratio(
        &r, &b, annualize, ann_factor,
    ))
}

/// R-squared of portfolio returns against benchmark.
#[wasm_bindgen(js_name = rSquared)]
pub fn r_squared(returns: JsValue, benchmark: JsValue) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    let b: Vec<f64> = serde_wasm_bindgen::from_value(benchmark).map_err(to_js_err)?;
    Ok(fa::benchmark::r_squared(&r, &b))
}

/// Up-capture ratio (participation in benchmark gains).
#[wasm_bindgen(js_name = upCapture)]
pub fn up_capture(returns: JsValue, benchmark: JsValue) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    let b: Vec<f64> = serde_wasm_bindgen::from_value(benchmark).map_err(to_js_err)?;
    Ok(fa::benchmark::up_capture(&r, &b))
}

/// Down-capture ratio (participation in benchmark losses).
#[wasm_bindgen(js_name = downCapture)]
pub fn down_capture(returns: JsValue, benchmark: JsValue) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    let b: Vec<f64> = serde_wasm_bindgen::from_value(benchmark).map_err(to_js_err)?;
    Ok(fa::benchmark::down_capture(&r, &b))
}

/// Capture ratio (up-capture / down-capture).
#[wasm_bindgen(js_name = captureRatio)]
pub fn capture_ratio(returns: JsValue, benchmark: JsValue) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    let b: Vec<f64> = serde_wasm_bindgen::from_value(benchmark).map_err(to_js_err)?;
    Ok(fa::benchmark::capture_ratio(&r, &b))
}

/// Batting average (fraction of periods outperforming the benchmark).
#[wasm_bindgen(js_name = battingAverage)]
pub fn batting_average(returns: JsValue, benchmark: JsValue) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    let b: Vec<f64> = serde_wasm_bindgen::from_value(benchmark).map_err(to_js_err)?;
    Ok(fa::benchmark::batting_average(&r, &b))
}

/// Treynor ratio from pre-computed values.
#[wasm_bindgen(js_name = treynor)]
pub fn treynor(ann_return: f64, risk_free_rate: f64, beta: f64) -> f64 {
    fa::benchmark::treynor(ann_return, risk_free_rate, beta)
}

/// M-squared from pre-computed values.
#[wasm_bindgen(js_name = mSquared)]
pub fn m_squared(ann_return: f64, ann_vol: f64, bench_vol: f64, risk_free_rate: f64) -> f64 {
    fa::benchmark::m_squared(ann_return, ann_vol, bench_vol, risk_free_rate)
}

/// OLS beta regression with standard error and 95% confidence interval.
#[wasm_bindgen(js_name = beta)]
pub fn beta(portfolio: JsValue, benchmark: JsValue) -> Result<JsValue, JsValue> {
    let p: Vec<f64> = serde_wasm_bindgen::from_value(portfolio).map_err(to_js_err)?;
    let b: Vec<f64> = serde_wasm_bindgen::from_value(benchmark).map_err(to_js_err)?;
    let result = fa::benchmark::beta(&p, &b);
    serde_wasm_bindgen::to_value(&result).map_err(to_js_err)
}

/// Single-index greeks: alpha, beta, R-squared, adjusted R-squared.
#[wasm_bindgen(js_name = greeks)]
pub fn greeks(returns: JsValue, benchmark: JsValue, ann_factor: f64) -> Result<JsValue, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    let b: Vec<f64> = serde_wasm_bindgen::from_value(benchmark).map_err(to_js_err)?;
    let result = fa::benchmark::greeks(&r, &b, ann_factor);
    serde_wasm_bindgen::to_value(&result).map_err(to_js_err)
}

/// Rolling alpha and beta over a sliding window.
#[wasm_bindgen(js_name = rollingGreeks)]
pub fn rolling_greeks(
    returns: JsValue,
    benchmark: JsValue,
    dates: JsValue,
    window: usize,
    ann_factor: f64,
) -> Result<JsValue, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    let b: Vec<f64> = serde_wasm_bindgen::from_value(benchmark).map_err(to_js_err)?;
    let date_strs: Vec<String> = serde_wasm_bindgen::from_value(dates).map_err(to_js_err)?;
    let rd = parse_iso_dates(&date_strs)?;
    let rg = fa::benchmark::rolling_greeks(&r, &b, &rd, window, ann_factor);
    serde_wasm_bindgen::to_value(&rg).map_err(to_js_err)
}

/// Multi-factor regression: alpha, factor betas, R-squared, residual vol.
#[wasm_bindgen(js_name = multiFactorGreeks)]
pub fn multi_factor_greeks(
    returns: JsValue,
    factors: JsValue,
    ann_factor: f64,
) -> Result<JsValue, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    let f: Vec<Vec<f64>> = serde_wasm_bindgen::from_value(factors).map_err(to_js_err)?;
    let refs: Vec<&[f64]> = f.iter().map(|v| v.as_slice()).collect();
    let result = fa::benchmark::multi_factor_greeks(&r, &refs, ann_factor).map_err(to_js_err)?;
    serde_wasm_bindgen::to_value(&result).map_err(to_js_err)
}
