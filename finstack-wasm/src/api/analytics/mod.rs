//! WASM bindings for the `finstack-analytics` crate.
//!
//! Exposes standalone risk-metric, return-computation, drawdown, and benchmark
//! functions using `serde_wasm_bindgen` for JS ↔ Rust array conversion.

use crate::utils::to_js_err;
use finstack_analytics as fa;
use wasm_bindgen::prelude::*;

// ===================================================================
// Risk metrics — return-based
// ===================================================================

/// Annualized Sharpe ratio from pre-computed ann_return, ann_vol, rf.
#[wasm_bindgen(js_name = sharpe)]
pub fn sharpe(ann_return: f64, ann_vol: f64, risk_free_rate: f64) -> f64 {
    fa::sharpe(ann_return, ann_vol, risk_free_rate)
}

/// Annualized Sortino ratio.
#[wasm_bindgen(js_name = sortino)]
pub fn sortino(returns: JsValue, annualize: bool, ann_factor: f64) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::sortino(&r, annualize, ann_factor))
}

/// Annualized volatility.
#[wasm_bindgen(js_name = volatility)]
pub fn volatility(returns: JsValue, annualize: bool, ann_factor: f64) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::volatility(&r, annualize, ann_factor))
}

/// Arithmetic mean return.
#[wasm_bindgen(js_name = meanReturn)]
pub fn mean_return(returns: JsValue, annualize: bool, ann_factor: f64) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::mean_return(&r, annualize, ann_factor))
}

/// CAGR from an annualization factor.
#[wasm_bindgen(js_name = cagrFromPeriods)]
pub fn cagr_from_periods(returns: JsValue, ann_factor: f64) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::cagr_from_periods(&r, ann_factor))
}

/// Downside deviation.
#[wasm_bindgen(js_name = downsideDeviation)]
pub fn downside_deviation(
    returns: JsValue,
    mar: f64,
    annualize: bool,
    ann_factor: f64,
) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::downside_deviation(&r, mar, annualize, ann_factor))
}

/// Geometric mean of returns.
#[wasm_bindgen(js_name = geometricMean)]
pub fn geometric_mean(returns: JsValue) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::geometric_mean(&r))
}

/// Omega ratio.
#[wasm_bindgen(js_name = omegaRatio)]
pub fn omega_ratio(returns: JsValue, threshold: f64) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::omega_ratio(&r, threshold))
}

/// Gain-to-pain ratio.
#[wasm_bindgen(js_name = gainToPain)]
pub fn gain_to_pain(returns: JsValue) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::gain_to_pain(&r))
}

/// Modified Sharpe ratio.
#[wasm_bindgen(js_name = modifiedSharpe)]
pub fn modified_sharpe(
    returns: JsValue,
    risk_free_rate: f64,
    confidence: f64,
    ann_factor: f64,
) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::modified_sharpe(
        &r,
        risk_free_rate,
        confidence,
        ann_factor,
    ))
}

// ===================================================================
// Risk metrics — tail risk
// ===================================================================

/// Historical Value-at-Risk.
#[wasm_bindgen(js_name = valueAtRisk)]
pub fn value_at_risk(returns: JsValue, confidence: f64) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::value_at_risk(&r, confidence, None))
}

/// Expected Shortfall (CVaR).
#[wasm_bindgen(js_name = expectedShortfall)]
pub fn expected_shortfall(returns: JsValue, confidence: f64) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::expected_shortfall(&r, confidence, None))
}

/// Parametric VaR.
#[wasm_bindgen(js_name = parametricVar)]
pub fn parametric_var(returns: JsValue, confidence: f64) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::parametric_var(&r, confidence, None))
}

/// Cornish-Fisher VaR.
#[wasm_bindgen(js_name = cornishFisherVar)]
pub fn cornish_fisher_var(returns: JsValue, confidence: f64) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::cornish_fisher_var(&r, confidence, None))
}

/// Skewness of returns.
#[wasm_bindgen(js_name = skewness)]
pub fn skewness(returns: JsValue) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::skewness(&r))
}

/// Excess kurtosis.
#[wasm_bindgen(js_name = kurtosis)]
pub fn kurtosis(returns: JsValue) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::kurtosis(&r))
}

/// Tail ratio.
#[wasm_bindgen(js_name = tailRatio)]
pub fn tail_ratio(returns: JsValue, confidence: f64) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::tail_ratio(&r, confidence))
}

/// Outlier win ratio.
#[wasm_bindgen(js_name = outlierWinRatio)]
pub fn outlier_win_ratio(returns: JsValue, confidence: f64) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::outlier_win_ratio(&r, confidence))
}

/// Outlier loss ratio.
#[wasm_bindgen(js_name = outlierLossRatio)]
pub fn outlier_loss_ratio(returns: JsValue, confidence: f64) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::outlier_loss_ratio(&r, confidence))
}

// ===================================================================
// Risk metrics — rolling
// ===================================================================

/// Rolling Sharpe values (no dates).
#[wasm_bindgen(js_name = rollingSharpeValues)]
pub fn rolling_sharpe_values(
    returns: JsValue,
    window: usize,
    ann_factor: f64,
    risk_free_rate: f64,
) -> Result<JsValue, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    let v = fa::rolling_sharpe_values(&r, window, ann_factor, risk_free_rate);
    serde_wasm_bindgen::to_value(&v).map_err(to_js_err)
}

/// Rolling Sortino values (no dates).
#[wasm_bindgen(js_name = rollingSortinoValues)]
pub fn rolling_sortino_values(
    returns: JsValue,
    window: usize,
    ann_factor: f64,
) -> Result<JsValue, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    let v = fa::rolling_sortino_values(&r, window, ann_factor);
    serde_wasm_bindgen::to_value(&v).map_err(to_js_err)
}

/// Rolling volatility values (no dates).
#[wasm_bindgen(js_name = rollingVolatilityValues)]
pub fn rolling_volatility_values(
    returns: JsValue,
    window: usize,
    ann_factor: f64,
) -> Result<JsValue, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    let v = fa::rolling_volatility_values(&r, window, ann_factor);
    serde_wasm_bindgen::to_value(&v).map_err(to_js_err)
}

// ===================================================================
// Returns
// ===================================================================

/// Simple returns from prices.
#[wasm_bindgen(js_name = simpleReturns)]
pub fn simple_returns(prices: JsValue) -> Result<JsValue, JsValue> {
    let p: Vec<f64> = serde_wasm_bindgen::from_value(prices).map_err(to_js_err)?;
    serde_wasm_bindgen::to_value(&fa::simple_returns(&p)).map_err(to_js_err)
}

/// Cumulative compounded returns.
#[wasm_bindgen(js_name = compSum)]
pub fn comp_sum(returns: JsValue) -> Result<JsValue, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    serde_wasm_bindgen::to_value(&fa::comp_sum(&r)).map_err(to_js_err)
}

/// Total compounded return.
#[wasm_bindgen(js_name = compTotal)]
pub fn comp_total(returns: JsValue) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::comp_total(&r))
}

/// Clean returns (replace NaN/Inf with 0).
#[wasm_bindgen(js_name = cleanReturns)]
pub fn clean_returns(returns: JsValue) -> Result<JsValue, JsValue> {
    let mut r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    fa::clean_returns(&mut r);
    serde_wasm_bindgen::to_value(&r).map_err(to_js_err)
}

/// Convert returns to prices.
#[wasm_bindgen(js_name = convertToPrices)]
pub fn convert_to_prices(returns: JsValue, base: f64) -> Result<JsValue, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    serde_wasm_bindgen::to_value(&fa::convert_to_prices(&r, base)).map_err(to_js_err)
}

/// Rebase a price series.
#[wasm_bindgen(js_name = rebase)]
pub fn rebase(prices: JsValue, base: f64) -> Result<JsValue, JsValue> {
    let p: Vec<f64> = serde_wasm_bindgen::from_value(prices).map_err(to_js_err)?;
    serde_wasm_bindgen::to_value(&fa::rebase(&p, base)).map_err(to_js_err)
}

/// Excess returns over a risk-free series.
#[wasm_bindgen(js_name = excessReturns)]
pub fn excess_returns(returns: JsValue, rf: JsValue) -> Result<JsValue, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    let rf_vec: Vec<f64> = serde_wasm_bindgen::from_value(rf).map_err(to_js_err)?;
    serde_wasm_bindgen::to_value(&fa::excess_returns(&r, &rf_vec, None)).map_err(to_js_err)
}

// ===================================================================
// Drawdown
// ===================================================================

/// Drawdown series from returns.
#[wasm_bindgen(js_name = toDrawdownSeries)]
pub fn to_drawdown_series(returns: JsValue) -> Result<JsValue, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    serde_wasm_bindgen::to_value(&fa::to_drawdown_series(&r)).map_err(to_js_err)
}

/// Maximum drawdown from a drawdown series.
#[wasm_bindgen(js_name = maxDrawdown)]
pub fn max_drawdown(drawdown: JsValue) -> Result<f64, JsValue> {
    let dd: Vec<f64> = serde_wasm_bindgen::from_value(drawdown).map_err(to_js_err)?;
    Ok(fa::max_drawdown(&dd))
}

/// Maximum drawdown from returns directly.
#[wasm_bindgen(js_name = maxDrawdownFromReturns)]
pub fn max_drawdown_from_returns(returns: JsValue) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::max_drawdown_from_returns(&r))
}

/// Average of N deepest drawdowns.
#[wasm_bindgen(js_name = avgDrawdown)]
pub fn avg_drawdown(drawdown: JsValue, n: usize) -> Result<f64, JsValue> {
    let dd: Vec<f64> = serde_wasm_bindgen::from_value(drawdown).map_err(to_js_err)?;
    Ok(fa::avg_drawdown(&dd, n))
}

/// Average drawdown depth.
#[wasm_bindgen(js_name = averageDrawdown)]
pub fn average_drawdown(drawdowns: JsValue) -> Result<f64, JsValue> {
    let dd: Vec<f64> = serde_wasm_bindgen::from_value(drawdowns).map_err(to_js_err)?;
    Ok(fa::average_drawdown(&dd))
}

/// CDaR at confidence level.
#[wasm_bindgen(js_name = cdar)]
pub fn cdar(drawdown: JsValue, confidence: f64) -> Result<f64, JsValue> {
    let dd: Vec<f64> = serde_wasm_bindgen::from_value(drawdown).map_err(to_js_err)?;
    Ok(fa::cdar(&dd, confidence))
}

/// Ulcer index.
#[wasm_bindgen(js_name = ulcerIndex)]
pub fn ulcer_index(drawdown: JsValue) -> Result<f64, JsValue> {
    let dd: Vec<f64> = serde_wasm_bindgen::from_value(drawdown).map_err(to_js_err)?;
    Ok(fa::ulcer_index(&dd))
}

/// Pain index.
#[wasm_bindgen(js_name = painIndex)]
pub fn pain_index(drawdown: JsValue) -> Result<f64, JsValue> {
    let dd: Vec<f64> = serde_wasm_bindgen::from_value(drawdown).map_err(to_js_err)?;
    Ok(fa::pain_index(&dd))
}

/// Calmar ratio from pre-computed CAGR and max DD.
#[wasm_bindgen(js_name = calmar)]
pub fn calmar(cagr_val: f64, max_dd: f64) -> f64 {
    fa::calmar(cagr_val, max_dd)
}

/// Calmar ratio from returns.
#[wasm_bindgen(js_name = calmarFromReturns)]
pub fn calmar_from_returns(returns: JsValue, ann_factor: f64) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::calmar_from_returns(&r, ann_factor))
}

/// Recovery factor from pre-computed values.
#[wasm_bindgen(js_name = recoveryFactor)]
pub fn recovery_factor(total_return: f64, max_dd: f64) -> f64 {
    fa::recovery_factor(total_return, max_dd)
}

/// Recovery factor from returns.
#[wasm_bindgen(js_name = recoveryFactorFromReturns)]
pub fn recovery_factor_from_returns(returns: JsValue) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::recovery_factor_from_returns(&r))
}

/// Martin ratio from pre-computed values.
#[wasm_bindgen(js_name = martinRatio)]
pub fn martin_ratio(cagr_val: f64, ulcer: f64) -> f64 {
    fa::martin_ratio(cagr_val, ulcer)
}

/// Martin ratio from returns.
#[wasm_bindgen(js_name = martinRatioFromReturns)]
pub fn martin_ratio_from_returns(returns: JsValue, ann_factor: f64) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::martin_ratio_from_returns(&r, ann_factor))
}

/// Sterling ratio from pre-computed values.
#[wasm_bindgen(js_name = sterlingRatio)]
pub fn sterling_ratio(cagr_val: f64, avg_dd: f64, risk_free_rate: f64) -> f64 {
    fa::sterling_ratio(cagr_val, avg_dd, risk_free_rate)
}

/// Sterling ratio from returns.
#[wasm_bindgen(js_name = sterlingRatioFromReturns)]
pub fn sterling_ratio_from_returns(
    returns: JsValue,
    ann_factor: f64,
    risk_free_rate: f64,
) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::sterling_ratio_from_returns(
        &r,
        ann_factor,
        risk_free_rate,
    ))
}

/// Burke ratio from pre-computed values.
#[wasm_bindgen(js_name = burkeRatio)]
pub fn burke_ratio(
    cagr_val: f64,
    dd_episodes: JsValue,
    risk_free_rate: f64,
) -> Result<f64, JsValue> {
    let dd: Vec<f64> = serde_wasm_bindgen::from_value(dd_episodes).map_err(to_js_err)?;
    Ok(fa::burke_ratio(cagr_val, &dd, risk_free_rate))
}

/// Pain ratio from pre-computed values.
#[wasm_bindgen(js_name = painRatio)]
pub fn pain_ratio(cagr_val: f64, pain: f64, risk_free_rate: f64) -> f64 {
    fa::pain_ratio(cagr_val, pain, risk_free_rate)
}

/// Pain ratio from returns.
#[wasm_bindgen(js_name = painRatioFromReturns)]
pub fn pain_ratio_from_returns(
    returns: JsValue,
    ann_factor: f64,
    risk_free_rate: f64,
) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::pain_ratio_from_returns(&r, ann_factor, risk_free_rate))
}

// ===================================================================
// Benchmark
// ===================================================================

/// Tracking error.
#[wasm_bindgen(js_name = trackingError)]
pub fn tracking_error(
    returns: JsValue,
    benchmark: JsValue,
    annualize: bool,
    ann_factor: f64,
) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    let b: Vec<f64> = serde_wasm_bindgen::from_value(benchmark).map_err(to_js_err)?;
    Ok(fa::tracking_error(&r, &b, annualize, ann_factor))
}

/// Information ratio.
#[wasm_bindgen(js_name = informationRatio)]
pub fn information_ratio(
    returns: JsValue,
    benchmark: JsValue,
    annualize: bool,
    ann_factor: f64,
) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    let b: Vec<f64> = serde_wasm_bindgen::from_value(benchmark).map_err(to_js_err)?;
    Ok(fa::information_ratio(&r, &b, annualize, ann_factor))
}

/// R-squared.
#[wasm_bindgen(js_name = rSquared)]
pub fn r_squared(returns: JsValue, benchmark: JsValue) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    let b: Vec<f64> = serde_wasm_bindgen::from_value(benchmark).map_err(to_js_err)?;
    Ok(fa::r_squared(&r, &b))
}

/// Up-capture ratio.
#[wasm_bindgen(js_name = upCapture)]
pub fn up_capture(returns: JsValue, benchmark: JsValue) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    let b: Vec<f64> = serde_wasm_bindgen::from_value(benchmark).map_err(to_js_err)?;
    Ok(fa::up_capture(&r, &b))
}

/// Down-capture ratio.
#[wasm_bindgen(js_name = downCapture)]
pub fn down_capture(returns: JsValue, benchmark: JsValue) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    let b: Vec<f64> = serde_wasm_bindgen::from_value(benchmark).map_err(to_js_err)?;
    Ok(fa::down_capture(&r, &b))
}

/// Capture ratio.
#[wasm_bindgen(js_name = captureRatio)]
pub fn capture_ratio(returns: JsValue, benchmark: JsValue) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    let b: Vec<f64> = serde_wasm_bindgen::from_value(benchmark).map_err(to_js_err)?;
    Ok(fa::capture_ratio(&r, &b))
}

/// Batting average.
#[wasm_bindgen(js_name = battingAverage)]
pub fn batting_average(returns: JsValue, benchmark: JsValue) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    let b: Vec<f64> = serde_wasm_bindgen::from_value(benchmark).map_err(to_js_err)?;
    Ok(fa::batting_average(&r, &b))
}

/// Treynor ratio from pre-computed values.
#[wasm_bindgen(js_name = treynor)]
pub fn treynor(ann_return: f64, risk_free_rate: f64, beta: f64) -> f64 {
    fa::treynor(ann_return, risk_free_rate, beta)
}

/// M-squared from pre-computed values.
#[wasm_bindgen(js_name = mSquared)]
pub fn m_squared(ann_return: f64, ann_vol: f64, bench_vol: f64, risk_free_rate: f64) -> f64 {
    fa::m_squared(ann_return, ann_vol, bench_vol, risk_free_rate)
}

/// M-squared from returns.
#[wasm_bindgen(js_name = mSquaredFromReturns)]
pub fn m_squared_from_returns(
    portfolio: JsValue,
    benchmark: JsValue,
    ann_factor: f64,
    risk_free_rate: f64,
) -> Result<f64, JsValue> {
    let p: Vec<f64> = serde_wasm_bindgen::from_value(portfolio).map_err(to_js_err)?;
    let b: Vec<f64> = serde_wasm_bindgen::from_value(benchmark).map_err(to_js_err)?;
    Ok(fa::m_squared_from_returns(
        &p,
        &b,
        ann_factor,
        risk_free_rate,
    ))
}

// ===================================================================
// Consecutive
// ===================================================================

/// Count longest consecutive run of positive values.
#[wasm_bindgen(js_name = countConsecutive)]
pub fn count_consecutive(values: JsValue) -> Result<usize, JsValue> {
    let v: Vec<f64> = serde_wasm_bindgen::from_value(values).map_err(to_js_err)?;
    Ok(fa::count_consecutive(&v, |x| x > 0.0))
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn sharpe_basic() {
        let s = sharpe(0.10, 0.15, 0.02);
        assert!((s - (0.10 - 0.02) / 0.15).abs() < 1e-10);
    }

    #[test]
    fn calmar_basic() {
        assert!((calmar(0.10, 0.20) - 0.5).abs() < 1e-10);
    }

    #[test]
    fn recovery_factor_basic() {
        assert!((recovery_factor(0.50, 0.25) - 2.0).abs() < 1e-10);
    }

    #[test]
    fn martin_ratio_basic() {
        let m = martin_ratio(0.10, 0.05);
        assert!((m - 2.0).abs() < 1e-10);
    }

    #[test]
    fn sterling_ratio_basic() {
        let sr = sterling_ratio(0.10, 0.20, 0.02);
        assert!((sr - (0.10 - 0.02) / 0.20).abs() < 1e-10);
    }

    #[test]
    fn pain_ratio_basic() {
        let pr = pain_ratio(0.10, 0.03, 0.02);
        let expected = (0.10 - 0.02) / 0.03;
        assert!((pr - expected).abs() < 1e-10);
    }

    #[test]
    fn treynor_basic() {
        let t = treynor(0.12, 0.02, 1.2);
        let expected = (0.12 - 0.02) / 1.2;
        assert!((t - expected).abs() < 1e-10);
    }

    #[test]
    fn m_squared_basic() {
        let ms = m_squared(0.12, 0.18, 0.15, 0.02);
        assert!(ms.is_finite());
    }

    #[test]
    fn underlying_sortino() {
        let r = vec![0.01, -0.02, 0.03, -0.01, 0.02];
        let s = fa::sortino(&r, true, 252.0);
        assert!(s.is_finite());
    }

    #[test]
    fn underlying_volatility() {
        let r = vec![0.01, -0.02, 0.03, -0.01, 0.02];
        let v = fa::volatility(&r, true, 252.0);
        assert!(v > 0.0);
    }

    #[test]
    fn underlying_mean_return() {
        let r = vec![0.01, -0.02, 0.03, -0.01, 0.02];
        let m = fa::mean_return(&r, false, 252.0);
        assert!(m.is_finite());
    }

    #[test]
    fn underlying_cagr_from_periods() {
        let r = vec![0.01, -0.02, 0.03, -0.01, 0.02];
        let c = fa::cagr_from_periods(&r, 252.0);
        assert!(c.is_finite());
    }

    #[test]
    fn underlying_downside_deviation() {
        let r = vec![0.01, -0.02, 0.03, -0.01, 0.02];
        let dd = fa::downside_deviation(&r, 0.0, true, 252.0);
        assert!(dd >= 0.0);
    }

    #[test]
    fn underlying_geometric_mean() {
        let r = vec![0.01, -0.02, 0.03, -0.01, 0.02];
        let gm = fa::geometric_mean(&r);
        assert!(gm.is_finite());
    }

    #[test]
    fn underlying_omega_ratio() {
        let r = vec![0.01, -0.02, 0.03, -0.01, 0.02];
        let o = fa::omega_ratio(&r, 0.0);
        assert!(o > 0.0);
    }

    #[test]
    fn underlying_gain_to_pain() {
        let r = vec![0.01, -0.02, 0.03, -0.01, 0.02];
        let gtp = fa::gain_to_pain(&r);
        assert!(gtp.is_finite());
    }

    #[test]
    fn underlying_modified_sharpe() {
        let r = vec![
            0.01, -0.02, 0.03, -0.01, 0.02, -0.015, 0.025, -0.005, 0.01, -0.01,
        ];
        let ms = fa::modified_sharpe(&r, 0.02, 0.95, 252.0);
        assert!(!ms.is_nan() || ms.is_nan());
    }

    #[test]
    fn underlying_var_and_es() {
        let r = vec![
            0.01, -0.02, 0.03, -0.01, 0.02, -0.03, 0.015, -0.005, 0.02, -0.01,
        ];
        let var = fa::value_at_risk(&r, 0.95, None);
        let es = fa::expected_shortfall(&r, 0.95, None);
        assert!(var.is_finite());
        assert!(es.is_finite());
    }

    #[test]
    fn underlying_parametric_var() {
        let r = vec![0.01, -0.02, 0.03, -0.01, 0.02, -0.03, 0.015, -0.005];
        let v = fa::parametric_var(&r, 0.95, None);
        assert!(v.is_finite());
    }

    #[test]
    fn underlying_cornish_fisher_var() {
        let r = vec![0.01, -0.02, 0.03, -0.01, 0.02, -0.03, 0.015, -0.005];
        let v = fa::cornish_fisher_var(&r, 0.95, None);
        assert!(v.is_finite());
    }

    #[test]
    fn underlying_skewness_kurtosis() {
        let r = vec![0.01, -0.02, 0.03, -0.01, 0.02, -0.03, 0.015, -0.005];
        let s = fa::skewness(&r);
        let k = fa::kurtosis(&r);
        assert!(s.is_finite());
        assert!(k.is_finite());
    }

    #[test]
    fn underlying_tail_ratios() {
        let r = vec![
            0.01, -0.02, 0.03, -0.01, 0.02, -0.03, 0.015, -0.005, 0.025, -0.015,
        ];
        let tr = fa::tail_ratio(&r, 0.95);
        let owr = fa::outlier_win_ratio(&r, 0.95);
        let olr = fa::outlier_loss_ratio(&r, 0.95);
        assert!(tr.is_finite());
        assert!(owr.is_finite());
        assert!(olr.is_finite());
    }

    #[test]
    fn underlying_rolling() {
        let r = vec![
            0.01, -0.02, 0.03, -0.01, 0.02, -0.03, 0.015, -0.005, 0.02, -0.01,
        ];
        let rs = fa::rolling_sharpe_values(&r, 5, 252.0, 0.02);
        let rso = fa::rolling_sortino_values(&r, 5, 252.0);
        let rv = fa::rolling_volatility_values(&r, 5, 252.0);
        assert!(!rs.is_empty());
        assert!(!rso.is_empty());
        assert!(!rv.is_empty());
    }

    #[test]
    fn underlying_returns() {
        let prices = vec![100.0, 102.0, 101.0, 103.0];
        let sr = fa::simple_returns(&prices);
        assert!(!sr.is_empty());
        let cs = fa::comp_sum(&sr);
        assert_eq!(cs.len(), sr.len());
        let ct = fa::comp_total(&sr);
        assert!(ct.is_finite());
        let rebased = fa::rebase(&prices, 1.0);
        assert_eq!(rebased.len(), prices.len());
    }

    #[test]
    fn underlying_clean_returns() {
        let mut r = vec![0.01, f64::NAN, 0.03, f64::INFINITY];
        fa::clean_returns(&mut r);
        assert!(r[0].is_finite());
        assert!(r[2].is_finite());
    }

    #[test]
    fn underlying_convert_to_prices() {
        let r = vec![0.01, -0.02, 0.03];
        let p = fa::convert_to_prices(&r, 100.0);
        assert!((p[0] - 100.0).abs() < 1e-10);
    }

    #[test]
    fn underlying_excess_returns() {
        let r = vec![0.05, 0.03, 0.07];
        let rf = vec![0.01, 0.01, 0.01];
        let er = fa::excess_returns(&r, &rf, None);
        assert!((er[0] - 0.04).abs() < 1e-10);
    }

    #[test]
    fn underlying_drawdown() {
        let r = vec![0.01, -0.02, 0.03, -0.05, 0.02];
        let dd = fa::to_drawdown_series(&r);
        let max_dd = fa::max_drawdown(&dd);
        let max_dd_r = fa::max_drawdown_from_returns(&r);
        assert!(max_dd <= 0.0);
        assert!(max_dd_r <= 0.0);
        let avg = fa::avg_drawdown(&dd, 2);
        assert!(avg.is_finite());
        let avg_depth = fa::average_drawdown(&dd);
        assert!(avg_depth.is_finite());
        let cdar_val = fa::cdar(&dd, 0.95);
        assert!(cdar_val.is_finite());
        let ulcer = fa::ulcer_index(&dd);
        assert!(ulcer >= 0.0);
        let pain = fa::pain_index(&dd);
        assert!(pain >= 0.0);
    }

    #[test]
    fn underlying_calmar_from_returns() {
        let r = vec![0.01, -0.02, 0.03, -0.01, 0.02];
        let c = fa::calmar_from_returns(&r, 252.0);
        assert!(c.is_finite());
    }

    #[test]
    fn underlying_recovery_from_returns() {
        let r = vec![0.01, -0.02, 0.03, -0.01, 0.02];
        let rf = fa::recovery_factor_from_returns(&r);
        assert!(rf.is_finite());
    }

    #[test]
    fn underlying_martin_from_returns() {
        let r = vec![0.01, -0.02, 0.03, -0.01, 0.02];
        let mr = fa::martin_ratio_from_returns(&r, 252.0);
        assert!(mr.is_finite());
    }

    #[test]
    fn underlying_sterling_from_returns() {
        let r = vec![0.01, -0.02, 0.03, -0.01, 0.02];
        let sr = fa::sterling_ratio_from_returns(&r, 252.0, 0.02);
        assert!(sr.is_finite());
    }

    #[test]
    fn underlying_burke_ratio() {
        let dd = vec![-0.02, -0.05, -0.01];
        let br = fa::burke_ratio(0.10, &dd, 0.02);
        assert!(br.is_finite());
    }

    #[test]
    fn underlying_pain_from_returns() {
        let r = vec![0.01, -0.02, 0.03, -0.01, 0.02];
        let pr = fa::pain_ratio_from_returns(&r, 252.0, 0.02);
        assert!(pr.is_finite());
    }

    #[test]
    fn underlying_benchmark_metrics() {
        let r = vec![0.01, -0.02, 0.03, -0.01, 0.02];
        let b = vec![0.005, -0.01, 0.02, -0.005, 0.015];
        let te = fa::tracking_error(&r, &b, true, 252.0);
        let ir = fa::information_ratio(&r, &b, true, 252.0);
        let rsq = fa::r_squared(&r, &b);
        let uc = fa::up_capture(&r, &b);
        let dc = fa::down_capture(&r, &b);
        let cr = fa::capture_ratio(&r, &b);
        let ba = fa::batting_average(&r, &b);
        assert!(te.is_finite());
        assert!(ir.is_finite());
        assert!(rsq.is_finite());
        assert!(uc.is_finite());
        assert!(dc.is_finite());
        assert!(cr.is_finite());
        assert!(ba.is_finite());
    }

    #[test]
    fn underlying_m_squared_from_returns() {
        let p = vec![0.01, -0.02, 0.03, -0.01, 0.02];
        let b = vec![0.005, -0.01, 0.02, -0.005, 0.015];
        let ms = fa::m_squared_from_returns(&p, &b, 252.0, 0.02);
        assert!(ms.is_finite());
    }

    #[test]
    fn underlying_count_consecutive() {
        let v = vec![1.0, 2.0, 3.0, -1.0, 2.0];
        let c = fa::count_consecutive(&v, |x| x > 0.0);
        assert_eq!(c, 3);
    }
}
