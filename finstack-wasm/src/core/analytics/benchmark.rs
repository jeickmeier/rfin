//! WASM bindings for benchmark-relative analytics.

use crate::core::error::{core_to_js, js_error};
use wasm_bindgen::prelude::*;

/// Tracking error: annualized volatility of active (excess) returns.
///
/// @param {Float64Array} returns - Portfolio return series
/// @param {Float64Array} benchmark - Benchmark return series
/// @param {boolean} annualize - Whether to annualize
/// @param {number} annFactor - Periods per year
/// @returns {number} Tracking error
#[wasm_bindgen(js_name = trackingError)]
pub fn tracking_error(returns: &[f64], benchmark: &[f64], annualize: bool, ann_factor: f64) -> f64 {
    finstack_analytics::benchmark::tracking_error(returns, benchmark, annualize, ann_factor)
}

/// Information ratio: annualized active return / tracking error.
///
/// @param {Float64Array} returns - Portfolio return series
/// @param {Float64Array} benchmark - Benchmark return series
/// @param {boolean} annualize - Whether to annualize
/// @param {number} annFactor - Periods per year
/// @returns {number} Information ratio
#[wasm_bindgen(js_name = informationRatio)]
pub fn information_ratio(
    returns: &[f64],
    benchmark: &[f64],
    annualize: bool,
    ann_factor: f64,
) -> f64 {
    finstack_analytics::benchmark::information_ratio(returns, benchmark, annualize, ann_factor)
}

/// R-squared: proportion of portfolio variance explained by the benchmark.
///
/// @param {Float64Array} returns - Portfolio return series
/// @param {Float64Array} benchmark - Benchmark return series
/// @returns {number} R-squared in [0, 1]
#[wasm_bindgen(js_name = rSquared)]
pub fn r_squared(returns: &[f64], benchmark: &[f64]) -> f64 {
    finstack_analytics::benchmark::r_squared(returns, benchmark)
}

/// OLS beta of portfolio vs benchmark, returned as a JS object.
///
/// @param {Float64Array} portfolio - Portfolio return series
/// @param {Float64Array} benchmark - Benchmark return series
/// @returns {{ beta: number, stdErr: number, ciLower: number, ciUpper: number }}
#[wasm_bindgen(js_name = calcBeta)]
pub fn calc_beta(portfolio: &[f64], benchmark: &[f64]) -> Result<JsValue, JsValue> {
    let result = finstack_analytics::benchmark::calc_beta(portfolio, benchmark);
    let obj = js_sys::Object::new();
    js_sys::Reflect::set(&obj, &"beta".into(), &result.beta.into())
        .map_err(|_| js_error("Failed to set beta"))?;
    js_sys::Reflect::set(&obj, &"stdErr".into(), &result.std_err.into())
        .map_err(|_| js_error("Failed to set stdErr"))?;
    js_sys::Reflect::set(&obj, &"ciLower".into(), &result.ci_lower.into())
        .map_err(|_| js_error("Failed to set ciLower"))?;
    js_sys::Reflect::set(&obj, &"ciUpper".into(), &result.ci_upper.into())
        .map_err(|_| js_error("Failed to set ciUpper"))?;
    Ok(obj.into())
}

/// Single-factor greeks (alpha, beta, r_squared).
///
/// @param {Float64Array} returns - Portfolio return series
/// @param {Float64Array} benchmark - Benchmark return series
/// @param {number} annFactor - Periods per year
/// @returns {{ alpha: number, beta: number, rSquared: number }}
#[wasm_bindgen(js_name = greeks)]
pub fn greeks_js(returns: &[f64], benchmark: &[f64], ann_factor: f64) -> Result<JsValue, JsValue> {
    let g = finstack_analytics::benchmark::greeks(returns, benchmark, ann_factor);
    let obj = js_sys::Object::new();
    js_sys::Reflect::set(&obj, &"alpha".into(), &g.alpha.into())
        .map_err(|_| js_error("Failed to set alpha"))?;
    js_sys::Reflect::set(&obj, &"beta".into(), &g.beta.into())
        .map_err(|_| js_error("Failed to set beta"))?;
    js_sys::Reflect::set(&obj, &"rSquared".into(), &g.r_squared.into())
        .map_err(|_| js_error("Failed to set rSquared"))?;
    Ok(obj.into())
}

/// Up-market capture ratio.
///
/// @param {Float64Array} returns - Portfolio return series
/// @param {Float64Array} benchmark - Benchmark return series
/// @returns {number} Up capture ratio
#[wasm_bindgen(js_name = upCapture)]
pub fn up_capture(returns: &[f64], benchmark: &[f64]) -> f64 {
    finstack_analytics::benchmark::up_capture(returns, benchmark)
}

/// Down-market capture ratio.
///
/// @param {Float64Array} returns - Portfolio return series
/// @param {Float64Array} benchmark - Benchmark return series
/// @returns {number} Down capture ratio
#[wasm_bindgen(js_name = downCapture)]
pub fn down_capture(returns: &[f64], benchmark: &[f64]) -> f64 {
    finstack_analytics::benchmark::down_capture(returns, benchmark)
}

/// Capture ratio = up capture / down capture.
///
/// @param {Float64Array} returns - Portfolio return series
/// @param {Float64Array} benchmark - Benchmark return series
/// @returns {number} Capture ratio
#[wasm_bindgen(js_name = captureRatio)]
pub fn capture_ratio(returns: &[f64], benchmark: &[f64]) -> f64 {
    finstack_analytics::benchmark::capture_ratio(returns, benchmark)
}

/// Batting average: fraction of periods outperforming the benchmark.
///
/// @param {Float64Array} returns - Portfolio return series
/// @param {Float64Array} benchmark - Benchmark return series
/// @returns {number} Fraction in [0, 1]
#[wasm_bindgen(js_name = battingAverage)]
pub fn batting_average(returns: &[f64], benchmark: &[f64]) -> f64 {
    finstack_analytics::benchmark::batting_average(returns, benchmark)
}

/// Treynor ratio = (R_p - R_f) / beta.
///
/// @param {number} annReturn - Annualized portfolio return
/// @param {number} riskFreeRate - Annualized risk-free rate
/// @param {number} beta - Portfolio beta
/// @returns {number} Treynor ratio
#[wasm_bindgen(js_name = treynorRatio)]
pub fn treynor_ratio(ann_return: f64, risk_free_rate: f64, beta: f64) -> f64 {
    finstack_analytics::benchmark::treynor(ann_return, risk_free_rate, beta)
}

/// M-squared (Modigliani-Modigliani) risk-adjusted return.
///
/// @param {number} annReturn - Annualized portfolio return
/// @param {number} annVol - Annualized portfolio volatility
/// @param {number} benchVol - Annualized benchmark volatility
/// @param {number} riskFreeRate - Annualized risk-free rate
/// @returns {number} M-squared return
#[wasm_bindgen(js_name = mSquared)]
pub fn m_squared(ann_return: f64, ann_vol: f64, bench_vol: f64, risk_free_rate: f64) -> f64 {
    finstack_analytics::benchmark::m_squared(ann_return, ann_vol, bench_vol, risk_free_rate)
}

/// M-squared from return series.
///
/// @param {Float64Array} portfolio - Portfolio return series
/// @param {Float64Array} benchmark - Benchmark return series
/// @param {number} annFactor - Periods per year
/// @param {number} riskFreeRate - Annualized risk-free rate
/// @returns {number} M-squared return
#[wasm_bindgen(js_name = mSquaredFromReturns)]
pub fn m_squared_from_returns(
    portfolio: &[f64],
    benchmark: &[f64],
    ann_factor: f64,
    risk_free_rate: f64,
) -> f64 {
    finstack_analytics::benchmark::m_squared_from_returns(
        portfolio,
        benchmark,
        ann_factor,
        risk_free_rate,
    )
}

/// Multi-factor OLS regression of portfolio returns on factor returns.
///
/// @param {Float64Array} returns - Portfolio return series
/// @param {Float64Array[]} factors - Array of factor return series (flattened with factorCount)
/// @param {number} factorCount - Number of factors
/// @param {number} annFactor - Periods per year
/// @returns {{ alpha: number, betas: number[], rSquared: number, adjustedRSquared: number, residualVol: number }}
#[wasm_bindgen(js_name = multiFactorGreeks)]
pub fn multi_factor_greeks(
    returns: &[f64],
    factors_flat: &[f64],
    factor_count: usize,
    ann_factor: f64,
) -> Result<JsValue, JsValue> {
    if factor_count == 0 || factors_flat.is_empty() {
        return Err(js_error("At least one factor is required"));
    }
    let n = returns.len();
    if factors_flat.len() != factor_count * n {
        return Err(js_error(
            "factors_flat length must equal factorCount * returns.length",
        ));
    }

    let factor_vecs: Vec<Vec<f64>> = (0..factor_count)
        .map(|i| factors_flat[i * n..(i + 1) * n].to_vec())
        .collect();
    let factor_refs: Vec<&[f64]> = factor_vecs.iter().map(|v| v.as_slice()).collect();

    let result =
        finstack_analytics::benchmark::multi_factor_greeks(returns, &factor_refs, ann_factor)
            .map_err(core_to_js)?;

    let obj = js_sys::Object::new();
    js_sys::Reflect::set(&obj, &"alpha".into(), &result.alpha.into())
        .map_err(|_| js_error("Failed to set alpha"))?;

    let betas_arr = js_sys::Float64Array::new_with_length(result.betas.len() as u32);
    for (i, &b) in result.betas.iter().enumerate() {
        betas_arr.set_index(i as u32, b);
    }
    js_sys::Reflect::set(&obj, &"betas".into(), &betas_arr.into())
        .map_err(|_| js_error("Failed to set betas"))?;

    js_sys::Reflect::set(&obj, &"rSquared".into(), &result.r_squared.into())
        .map_err(|_| js_error("Failed to set rSquared"))?;
    js_sys::Reflect::set(
        &obj,
        &"adjustedRSquared".into(),
        &result.adjusted_r_squared.into(),
    )
    .map_err(|_| js_error("Failed to set adjustedRSquared"))?;
    js_sys::Reflect::set(&obj, &"residualVol".into(), &result.residual_vol.into())
        .map_err(|_| js_error("Failed to set residualVol"))?;
    Ok(obj.into())
}
