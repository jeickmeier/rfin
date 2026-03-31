//! WASM bindings for drawdown analytics.

use wasm_bindgen::prelude::*;

/// Compute a drawdown series from simple returns.
///
/// At each step, drawdown = wealth / peak - 1 (values <= 0).
///
/// @param {Float64Array} returns - Simple period returns
/// @returns {Float64Array} Drawdown series (same length, values <= 0)
#[wasm_bindgen(js_name = toDrawdownSeries)]
pub fn to_drawdown_series(returns: &[f64]) -> Vec<f64> {
    finstack_analytics::drawdown::to_drawdown_series(returns)
}

/// Maximum drawdown from a pre-computed drawdown series.
///
/// @param {Float64Array} drawdown - Pre-computed drawdown series (values <= 0)
/// @returns {number} Most negative drawdown value
#[wasm_bindgen(js_name = maxDrawdown)]
pub fn max_drawdown(drawdown: &[f64]) -> f64 {
    finstack_analytics::drawdown::max_drawdown(drawdown)
}

/// Maximum drawdown computed directly from returns.
///
/// @param {Float64Array} returns - Simple period returns
/// @returns {number} Most negative drawdown value
#[wasm_bindgen(js_name = maxDrawdownFromReturns)]
pub fn max_drawdown_from_returns(returns: &[f64]) -> f64 {
    finstack_analytics::drawdown::max_drawdown_from_returns(returns)
}

/// Average drawdown depth across all periods.
///
/// @param {Float64Array} drawdown - Pre-computed drawdown series
/// @returns {number} Mean drawdown (non-positive)
#[wasm_bindgen(js_name = averageDrawdown)]
pub fn average_drawdown(drawdown: &[f64]) -> f64 {
    finstack_analytics::drawdown::average_drawdown(drawdown)
}

/// Ulcer index: root-mean-square of the drawdown series.
///
/// @param {Float64Array} drawdown - Pre-computed drawdown series
/// @returns {number} Ulcer index (non-negative)
#[wasm_bindgen(js_name = ulcerIndex)]
pub fn ulcer_index(drawdown: &[f64]) -> f64 {
    finstack_analytics::drawdown::ulcer_index(drawdown)
}

/// Pain index: mean absolute drawdown.
///
/// @param {Float64Array} drawdown - Pre-computed drawdown series
/// @returns {number} Pain index (non-negative)
#[wasm_bindgen(js_name = painIndex)]
pub fn pain_index(drawdown: &[f64]) -> f64 {
    finstack_analytics::drawdown::pain_index(drawdown)
}

/// Conditional Drawdown at Risk (CDaR) at the given confidence level.
///
/// The expected drawdown depth in the tail beyond the (1-alpha) quantile.
///
/// @param {Float64Array} drawdown - Pre-computed drawdown series
/// @param {number} confidence - Confidence level in (0, 1), e.g. 0.95
/// @returns {number} CDaR (non-negative, absolute drawdown depth)
#[wasm_bindgen(js_name = cdar)]
pub fn cdar(drawdown: &[f64], confidence: f64) -> f64 {
    finstack_analytics::drawdown::cdar(drawdown, confidence)
}

/// Calmar ratio = CAGR / |max drawdown|.
///
/// @param {number} cagrVal - Compound annual growth rate
/// @param {number} maxDd - Maximum drawdown (negative, e.g. -0.25)
/// @returns {number} Calmar ratio
#[wasm_bindgen(js_name = calmarRatio)]
pub fn calmar_ratio(cagr_val: f64, max_dd: f64) -> f64 {
    finstack_analytics::drawdown::calmar(cagr_val, max_dd)
}

/// Calmar ratio from returns.
///
/// @param {Float64Array} returns - Simple period returns
/// @param {number} annFactor - Periods per year
/// @returns {number} Calmar ratio
#[wasm_bindgen(js_name = calmarRatioFromReturns)]
pub fn calmar_ratio_from_returns(returns: &[f64], ann_factor: f64) -> f64 {
    finstack_analytics::drawdown::calmar_from_returns(returns, ann_factor)
}

/// Recovery factor = total return / |max drawdown|.
///
/// @param {number} totalReturn - Total compounded return
/// @param {number} maxDd - Maximum drawdown (negative)
/// @returns {number} Recovery factor
#[wasm_bindgen(js_name = recoveryFactor)]
pub fn recovery_factor(total_return: f64, max_dd: f64) -> f64 {
    finstack_analytics::drawdown::recovery_factor(total_return, max_dd)
}

/// Recovery factor from returns.
///
/// @param {Float64Array} returns - Simple period returns
/// @returns {number} Recovery factor
#[wasm_bindgen(js_name = recoveryFactorFromReturns)]
pub fn recovery_factor_from_returns(returns: &[f64]) -> f64 {
    finstack_analytics::drawdown::recovery_factor_from_returns(returns)
}

/// Martin ratio = CAGR / Ulcer Index.
///
/// @param {number} cagrVal - Compound annual growth rate
/// @param {number} ulcer - Ulcer index value
/// @returns {number} Martin ratio
#[wasm_bindgen(js_name = martinRatio)]
pub fn martin_ratio(cagr_val: f64, ulcer: f64) -> f64 {
    finstack_analytics::drawdown::martin_ratio(cagr_val, ulcer)
}

/// Martin ratio from returns.
///
/// @param {Float64Array} returns - Simple period returns
/// @param {number} annFactor - Periods per year
/// @returns {number} Martin ratio
#[wasm_bindgen(js_name = martinRatioFromReturns)]
pub fn martin_ratio_from_returns(returns: &[f64], ann_factor: f64) -> f64 {
    finstack_analytics::drawdown::martin_ratio_from_returns(returns, ann_factor)
}

/// Sterling ratio = (CAGR - Rf) / |avg drawdown|.
///
/// @param {number} cagrVal - Compound annual growth rate
/// @param {number} avgDd - Average drawdown (negative)
/// @param {number} riskFreeRate - Annualized risk-free rate
/// @returns {number} Sterling ratio
#[wasm_bindgen(js_name = sterlingRatio)]
pub fn sterling_ratio(cagr_val: f64, avg_dd: f64, risk_free_rate: f64) -> f64 {
    finstack_analytics::drawdown::sterling_ratio(cagr_val, avg_dd, risk_free_rate)
}

/// Sterling ratio from returns.
///
/// @param {Float64Array} returns - Simple period returns
/// @param {number} annFactor - Periods per year
/// @param {number} riskFreeRate - Annualized risk-free rate
/// @returns {number} Sterling ratio
#[wasm_bindgen(js_name = sterlingRatioFromReturns)]
pub fn sterling_ratio_from_returns(
    returns: &[f64],
    ann_factor: f64,
    risk_free_rate: f64,
) -> f64 {
    finstack_analytics::drawdown::sterling_ratio_from_returns(returns, ann_factor, risk_free_rate)
}

/// Burke ratio = (CAGR - Rf) / RMS(episode drawdowns).
///
/// @param {number} cagrVal - Compound annual growth rate
/// @param {Float64Array} ddEpisodes - Max-drawdown depths of each episode (negative)
/// @param {number} riskFreeRate - Annualized risk-free rate
/// @returns {number} Burke ratio
#[wasm_bindgen(js_name = burkeRatio)]
pub fn burke_ratio(cagr_val: f64, dd_episodes: &[f64], risk_free_rate: f64) -> f64 {
    finstack_analytics::drawdown::burke_ratio(cagr_val, dd_episodes, risk_free_rate)
}

/// Pain ratio = (CAGR - Rf) / Pain Index.
///
/// @param {number} cagrVal - Compound annual growth rate
/// @param {number} pain - Pain index value
/// @param {number} riskFreeRate - Annualized risk-free rate
/// @returns {number} Pain ratio
#[wasm_bindgen(js_name = painRatio)]
pub fn pain_ratio(cagr_val: f64, pain: f64, risk_free_rate: f64) -> f64 {
    finstack_analytics::drawdown::pain_ratio(cagr_val, pain, risk_free_rate)
}

/// Pain ratio from returns.
///
/// @param {Float64Array} returns - Simple period returns
/// @param {number} annFactor - Periods per year
/// @param {number} riskFreeRate - Annualized risk-free rate
/// @returns {number} Pain ratio
#[wasm_bindgen(js_name = painRatioFromReturns)]
pub fn pain_ratio_from_returns(
    returns: &[f64],
    ann_factor: f64,
    risk_free_rate: f64,
) -> f64 {
    finstack_analytics::drawdown::pain_ratio_from_returns(returns, ann_factor, risk_free_rate)
}
