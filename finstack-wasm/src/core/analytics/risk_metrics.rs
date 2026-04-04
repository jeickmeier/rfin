//! WASM bindings for return-based risk metrics.

use wasm_bindgen::prelude::*;

/// CAGR (Compound Annual Growth Rate) from a return series and holding period.
///
/// @param {Float64Array} returns - Simple period returns
/// @param {number} annFactor - Periods per year (e.g. 252 for daily)
/// @returns {number} Annualized CAGR
#[wasm_bindgen(js_name = cagrFromPeriods)]
pub fn cagr_from_periods(returns: &[f64], ann_factor: f64) -> f64 {
    finstack_analytics::risk_metrics::cagr_from_periods(returns, ann_factor)
}

/// Arithmetic mean return.
///
/// @param {Float64Array} returns - Simple period returns
/// @param {boolean} annualize - Whether to scale by the annualization factor
/// @param {number} annFactor - Periods per year
/// @returns {number} Mean return
#[wasm_bindgen(js_name = meanReturn)]
pub fn mean_return(returns: &[f64], annualize: bool, ann_factor: f64) -> f64 {
    finstack_analytics::risk_metrics::mean_return(returns, annualize, ann_factor)
}

/// Sample volatility (standard deviation of returns).
///
/// @param {Float64Array} returns - Simple period returns
/// @param {boolean} annualize - Whether to scale by sqrt(annFactor)
/// @param {number} annFactor - Periods per year
/// @returns {number} Volatility
#[wasm_bindgen(js_name = returnsVolatility)]
pub fn returns_volatility(returns: &[f64], annualize: bool, ann_factor: f64) -> f64 {
    finstack_analytics::risk_metrics::volatility(returns, annualize, ann_factor)
}

/// Sharpe ratio from pre-computed annualized return and volatility.
///
/// @param {number} annReturn - Annualized mean return
/// @param {number} annVol - Annualized volatility
/// @param {number} riskFreeRate - Annualized risk-free rate
/// @returns {number} Sharpe ratio
#[wasm_bindgen(js_name = sharpeRatio)]
pub fn sharpe_ratio(ann_return: f64, ann_vol: f64, risk_free_rate: f64) -> f64 {
    finstack_analytics::risk_metrics::sharpe(ann_return, ann_vol, risk_free_rate)
}

/// Sortino ratio from a return series.
///
/// @param {Float64Array} returns - Simple period returns
/// @param {boolean} annualize - Whether to annualize
/// @param {number} annFactor - Periods per year
/// @returns {number} Sortino ratio
#[wasm_bindgen(js_name = sortinoRatio)]
pub fn sortino_ratio(returns: &[f64], annualize: bool, ann_factor: f64) -> f64 {
    finstack_analytics::risk_metrics::sortino(returns, annualize, ann_factor)
}

/// Downside deviation (semi-deviation below the MAR threshold).
///
/// @param {Float64Array} returns - Simple period returns
/// @param {number} mar - Minimum acceptable return
/// @param {boolean} annualize - Whether to annualize
/// @param {number} annFactor - Periods per year
/// @returns {number} Downside deviation
#[wasm_bindgen(js_name = downsideDeviation)]
pub fn downside_deviation(returns: &[f64], mar: f64, annualize: bool, ann_factor: f64) -> f64 {
    finstack_analytics::risk_metrics::downside_deviation(returns, mar, annualize, ann_factor)
}

/// Geometric mean return.
///
/// @param {Float64Array} returns - Simple period returns
/// @returns {number} Geometric mean return per period
#[wasm_bindgen(js_name = geometricMeanReturn)]
pub fn geometric_mean_return(returns: &[f64]) -> f64 {
    finstack_analytics::risk_metrics::geometric_mean(returns)
}

/// Omega ratio: probability-weighted gain/loss ratio.
///
/// @param {Float64Array} returns - Simple period returns
/// @param {number} threshold - Per-period threshold (typically 0.0)
/// @returns {number} Omega ratio
#[wasm_bindgen(js_name = omegaRatio)]
pub fn omega_ratio(returns: &[f64], threshold: f64) -> f64 {
    finstack_analytics::risk_metrics::omega_ratio(returns, threshold)
}

/// Gain-to-pain ratio: sum of gains / sum of absolute losses.
///
/// @param {Float64Array} returns - Simple period returns
/// @returns {number} Gain-to-pain ratio
#[wasm_bindgen(js_name = gainToPain)]
pub fn gain_to_pain(returns: &[f64]) -> f64 {
    finstack_analytics::risk_metrics::gain_to_pain(returns)
}

/// Modified Sharpe ratio using Cornish-Fisher VaR.
///
/// @param {Float64Array} returns - Simple period returns
/// @param {number} riskFreeRate - Annualized risk-free rate
/// @param {number} confidence - VaR confidence level (e.g. 0.95)
/// @param {number} annFactor - Periods per year
/// @returns {number} Modified Sharpe ratio
#[wasm_bindgen(js_name = modifiedSharpe)]
pub fn modified_sharpe(
    returns: &[f64],
    risk_free_rate: f64,
    confidence: f64,
    ann_factor: f64,
) -> f64 {
    finstack_analytics::risk_metrics::modified_sharpe(
        returns,
        risk_free_rate,
        confidence,
        ann_factor,
    )
}

/// Sample skewness of returns.
///
/// @param {Float64Array} returns - Return series
/// @returns {number} Bias-corrected sample skewness
#[wasm_bindgen(js_name = returnsSkewness)]
pub fn returns_skewness(returns: &[f64]) -> f64 {
    finstack_analytics::risk_metrics::skewness(returns)
}

/// Excess kurtosis of returns.
///
/// @param {Float64Array} returns - Return series
/// @returns {number} Bias-corrected excess kurtosis
#[wasm_bindgen(js_name = returnsKurtosis)]
pub fn returns_kurtosis(returns: &[f64]) -> f64 {
    finstack_analytics::risk_metrics::kurtosis(returns)
}

/// Historical Value-at-Risk at the given confidence level.
///
/// @param {Float64Array} returns - Return series
/// @param {number} confidence - Confidence level in (0, 1), e.g. 0.95
/// @returns {number} VaR (non-positive)
#[wasm_bindgen(js_name = historicalVar)]
pub fn historical_var(returns: &[f64], confidence: f64) -> f64 {
    finstack_analytics::risk_metrics::value_at_risk(returns, confidence, None)
}

/// Expected Shortfall (CVaR) at the given confidence level.
///
/// @param {Float64Array} returns - Return series
/// @param {number} confidence - Confidence level in (0, 1), e.g. 0.95
/// @returns {number} Expected shortfall (non-positive)
#[wasm_bindgen(js_name = expectedShortfall)]
pub fn expected_shortfall(returns: &[f64], confidence: f64) -> f64 {
    finstack_analytics::risk_metrics::expected_shortfall(returns, confidence, None)
}

/// Parametric (Gaussian) VaR.
///
/// @param {Float64Array} returns - Return series
/// @param {number} confidence - Confidence level in (0, 1)
/// @returns {number} Parametric VaR
#[wasm_bindgen(js_name = parametricVar)]
pub fn parametric_var(returns: &[f64], confidence: f64) -> f64 {
    finstack_analytics::risk_metrics::parametric_var(returns, confidence, None)
}

/// Cornish-Fisher adjusted VaR.
///
/// @param {Float64Array} returns - Return series
/// @param {number} confidence - Confidence level in (0, 1)
/// @returns {number} Cornish-Fisher VaR
#[wasm_bindgen(js_name = cornishFisherVar)]
pub fn cornish_fisher_var(returns: &[f64], confidence: f64) -> f64 {
    finstack_analytics::risk_metrics::cornish_fisher_var(returns, confidence, None)
}

/// Tail ratio: upper quantile / |lower quantile|.
///
/// @param {Float64Array} returns - Return series
/// @param {number} confidence - Quantile level (e.g. 0.95)
/// @returns {number} Tail ratio
#[wasm_bindgen(js_name = tailRatio)]
pub fn tail_ratio(returns: &[f64], confidence: f64) -> f64 {
    let mut scratch = returns.to_vec();
    finstack_analytics::risk_metrics::tail_ratio_with_scratch(&mut scratch, confidence)
}
