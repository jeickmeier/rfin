use crate::utils::to_js_err;
use finstack_analytics as fa;
use wasm_bindgen::prelude::*;

use super::support::parse_iso_dates;

/// Compute a drawdown series from simple returns.
#[wasm_bindgen(js_name = toDrawdownSeries)]
pub fn to_drawdown_series(returns: JsValue) -> Result<JsValue, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    serde_wasm_bindgen::to_value(&fa::drawdown::to_drawdown_series(&r)).map_err(to_js_err)
}

/// Deepest drawdown from a pre-computed drawdown series.
#[wasm_bindgen(js_name = maxDrawdown)]
pub fn max_drawdown(drawdown: JsValue) -> Result<f64, JsValue> {
    let dd: Vec<f64> = serde_wasm_bindgen::from_value(drawdown).map_err(to_js_err)?;
    Ok(fa::drawdown::max_drawdown(&dd))
}

/// Mean of the N deepest drawdown episodes.
#[wasm_bindgen(js_name = meanEpisodeDrawdown)]
pub fn mean_episode_drawdown(drawdown: JsValue, n: usize) -> Result<f64, JsValue> {
    let dd: Vec<f64> = serde_wasm_bindgen::from_value(drawdown).map_err(to_js_err)?;
    Ok(fa::drawdown::mean_episode_drawdown(&dd, n))
}

/// Simple arithmetic mean of drawdown values.
#[wasm_bindgen(js_name = meanDrawdown)]
pub fn mean_drawdown(drawdowns: JsValue) -> Result<f64, JsValue> {
    let dd: Vec<f64> = serde_wasm_bindgen::from_value(drawdowns).map_err(to_js_err)?;
    Ok(fa::drawdown::mean_drawdown(&dd))
}

/// Conditional Drawdown at Risk (CDaR).
#[wasm_bindgen(js_name = cdar)]
pub fn cdar(drawdown: JsValue, confidence: f64) -> Result<f64, JsValue> {
    let dd: Vec<f64> = serde_wasm_bindgen::from_value(drawdown).map_err(to_js_err)?;
    Ok(fa::drawdown::cdar(&dd, confidence))
}

/// Ulcer index (square root of mean squared drawdown).
#[wasm_bindgen(js_name = ulcerIndex)]
pub fn ulcer_index(drawdown: JsValue) -> Result<f64, JsValue> {
    let dd: Vec<f64> = serde_wasm_bindgen::from_value(drawdown).map_err(to_js_err)?;
    Ok(fa::drawdown::ulcer_index(&dd))
}

/// Pain index (average drawdown depth).
#[wasm_bindgen(js_name = painIndex)]
pub fn pain_index(drawdown: JsValue) -> Result<f64, JsValue> {
    let dd: Vec<f64> = serde_wasm_bindgen::from_value(drawdown).map_err(to_js_err)?;
    Ok(fa::drawdown::pain_index(&dd))
}

/// Calmar ratio: CAGR / |max drawdown|.
#[wasm_bindgen(js_name = calmar)]
pub fn calmar(cagr_val: f64, max_dd: f64) -> f64 {
    fa::drawdown::calmar(cagr_val, max_dd)
}

/// Recovery factor: total return / |max drawdown|.
#[wasm_bindgen(js_name = recoveryFactor)]
pub fn recovery_factor(total_return: f64, max_dd: f64) -> f64 {
    fa::drawdown::recovery_factor(total_return, max_dd)
}

/// Martin ratio: CAGR / ulcer index.
#[wasm_bindgen(js_name = martinRatio)]
pub fn martin_ratio(cagr_val: f64, ulcer: f64) -> f64 {
    fa::drawdown::martin_ratio(cagr_val, ulcer)
}

/// Sterling ratio: (CAGR - risk-free rate) / |average drawdown|.
#[wasm_bindgen(js_name = sterlingRatio)]
pub fn sterling_ratio(cagr_val: f64, avg_dd: f64, risk_free_rate: f64) -> f64 {
    fa::drawdown::sterling_ratio(cagr_val, avg_dd, risk_free_rate)
}

/// Burke ratio: (CAGR - risk-free rate) / sqrt(sum of squared drawdown episodes).
#[wasm_bindgen(js_name = burkeRatio)]
pub fn burke_ratio(
    cagr_val: f64,
    dd_episodes: JsValue,
    risk_free_rate: f64,
) -> Result<f64, JsValue> {
    let dd: Vec<f64> = serde_wasm_bindgen::from_value(dd_episodes).map_err(to_js_err)?;
    Ok(fa::drawdown::burke_ratio(cagr_val, &dd, risk_free_rate))
}

/// Pain ratio: (CAGR - risk-free rate) / pain index.
#[wasm_bindgen(js_name = painRatio)]
pub fn pain_ratio(cagr_val: f64, pain: f64, risk_free_rate: f64) -> f64 {
    fa::drawdown::pain_ratio(cagr_val, pain, risk_free_rate)
}

/// Top-N drawdown episodes with start, valley, end, and depth.
#[wasm_bindgen(js_name = drawdownDetails)]
pub fn drawdown_details(drawdown: JsValue, dates: JsValue, n: usize) -> Result<JsValue, JsValue> {
    let dd: Vec<f64> = serde_wasm_bindgen::from_value(drawdown).map_err(to_js_err)?;
    let date_strs: Vec<String> = serde_wasm_bindgen::from_value(dates).map_err(to_js_err)?;
    let rd = parse_iso_dates(&date_strs)?;
    let episodes = fa::drawdown::drawdown_details(&dd, &rd, n);
    serde_wasm_bindgen::to_value(&episodes).map_err(to_js_err)
}

/// Maximum drawdown duration in calendar days.
#[wasm_bindgen(js_name = maxDrawdownDuration)]
pub fn max_drawdown_duration(drawdown: JsValue, dates: JsValue) -> Result<i64, JsValue> {
    let dd: Vec<f64> = serde_wasm_bindgen::from_value(drawdown).map_err(to_js_err)?;
    let date_strs: Vec<String> = serde_wasm_bindgen::from_value(dates).map_err(to_js_err)?;
    let rd = parse_iso_dates(&date_strs)?;
    Ok(fa::drawdown::max_drawdown_duration(&dd, &rd))
}
