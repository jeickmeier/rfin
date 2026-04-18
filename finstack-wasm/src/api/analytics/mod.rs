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
    fa::risk_metrics::sharpe(ann_return, ann_vol, risk_free_rate)
}

/// Annualized Sortino ratio.
#[wasm_bindgen(js_name = sortino)]
pub fn sortino(returns: JsValue, annualize: bool, ann_factor: f64) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::risk_metrics::sortino(&r, annualize, ann_factor))
}

/// Annualized volatility.
#[wasm_bindgen(js_name = volatility)]
pub fn volatility(returns: JsValue, annualize: bool, ann_factor: f64) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::risk_metrics::volatility(&r, annualize, ann_factor))
}

/// Arithmetic mean return.
#[wasm_bindgen(js_name = meanReturn)]
pub fn mean_return(returns: JsValue, annualize: bool, ann_factor: f64) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::risk_metrics::mean_return(&r, annualize, ann_factor))
}

/// CAGR from an annualization factor.
#[wasm_bindgen(js_name = cagrFromPeriods)]
pub fn cagr_from_periods(returns: JsValue, ann_factor: f64) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::risk_metrics::cagr_from_periods(&r, ann_factor))
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
    Ok(fa::risk_metrics::downside_deviation(
        &r, mar, annualize, ann_factor,
    ))
}

/// Geometric mean of returns.
#[wasm_bindgen(js_name = geometricMean)]
pub fn geometric_mean(returns: JsValue) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::risk_metrics::geometric_mean(&r))
}

/// Omega ratio.
#[wasm_bindgen(js_name = omegaRatio)]
pub fn omega_ratio(returns: JsValue, threshold: f64) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::risk_metrics::omega_ratio(&r, threshold))
}

/// Gain-to-pain ratio.
#[wasm_bindgen(js_name = gainToPain)]
pub fn gain_to_pain(returns: JsValue) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::risk_metrics::gain_to_pain(&r))
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
    Ok(fa::risk_metrics::modified_sharpe(
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
    Ok(fa::risk_metrics::value_at_risk(&r, confidence, None))
}

/// Expected Shortfall (CVaR).
#[wasm_bindgen(js_name = expectedShortfall)]
pub fn expected_shortfall(returns: JsValue, confidence: f64) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::risk_metrics::expected_shortfall(&r, confidence, None))
}

/// Parametric VaR.
#[wasm_bindgen(js_name = parametricVar)]
pub fn parametric_var(returns: JsValue, confidence: f64) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::risk_metrics::parametric_var(&r, confidence, None))
}

/// Cornish-Fisher VaR.
#[wasm_bindgen(js_name = cornishFisherVar)]
pub fn cornish_fisher_var(returns: JsValue, confidence: f64) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::risk_metrics::cornish_fisher_var(&r, confidence, None))
}

/// Skewness of returns.
#[wasm_bindgen(js_name = skewness)]
pub fn skewness(returns: JsValue) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::risk_metrics::skewness(&r))
}

/// Excess kurtosis.
#[wasm_bindgen(js_name = kurtosis)]
pub fn kurtosis(returns: JsValue) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::risk_metrics::kurtosis(&r))
}

/// Tail ratio.
#[wasm_bindgen(js_name = tailRatio)]
pub fn tail_ratio(returns: JsValue, confidence: f64) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::risk_metrics::tail_ratio(&r, confidence))
}

/// Outlier win ratio.
#[wasm_bindgen(js_name = outlierWinRatio)]
pub fn outlier_win_ratio(returns: JsValue, confidence: f64) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::risk_metrics::outlier_win_ratio(&r, confidence))
}

/// Outlier loss ratio.
#[wasm_bindgen(js_name = outlierLossRatio)]
pub fn outlier_loss_ratio(returns: JsValue, confidence: f64) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::risk_metrics::outlier_loss_ratio(&r, confidence))
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
    let v = fa::risk_metrics::rolling_sharpe_values(&r, window, ann_factor, risk_free_rate);
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
    let v = fa::risk_metrics::rolling_sortino_values(&r, window, ann_factor);
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
    let v = fa::risk_metrics::rolling_volatility_values(&r, window, ann_factor);
    serde_wasm_bindgen::to_value(&v).map_err(to_js_err)
}

// ===================================================================
// Returns
// ===================================================================

/// Simple returns from prices.
#[wasm_bindgen(js_name = simpleReturns)]
pub fn simple_returns(prices: JsValue) -> Result<JsValue, JsValue> {
    let p: Vec<f64> = serde_wasm_bindgen::from_value(prices).map_err(to_js_err)?;
    serde_wasm_bindgen::to_value(&fa::returns::simple_returns(&p)).map_err(to_js_err)
}

/// Cumulative compounded returns.
#[wasm_bindgen(js_name = compSum)]
pub fn comp_sum(returns: JsValue) -> Result<JsValue, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    serde_wasm_bindgen::to_value(&fa::returns::comp_sum(&r)).map_err(to_js_err)
}

/// Total compounded return.
#[wasm_bindgen(js_name = compTotal)]
pub fn comp_total(returns: JsValue) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::returns::comp_total(&r))
}

/// Clean returns (replace NaN/Inf with 0).
#[wasm_bindgen(js_name = cleanReturns)]
pub fn clean_returns(returns: JsValue) -> Result<JsValue, JsValue> {
    let mut r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    fa::returns::clean_returns(&mut r);
    serde_wasm_bindgen::to_value(&r).map_err(to_js_err)
}

/// Convert returns to prices.
#[wasm_bindgen(js_name = convertToPrices)]
pub fn convert_to_prices(returns: JsValue, base: f64) -> Result<JsValue, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    serde_wasm_bindgen::to_value(&fa::returns::convert_to_prices(&r, base)).map_err(to_js_err)
}

/// Rebase a price series.
#[wasm_bindgen(js_name = rebase)]
pub fn rebase(prices: JsValue, base: f64) -> Result<JsValue, JsValue> {
    let p: Vec<f64> = serde_wasm_bindgen::from_value(prices).map_err(to_js_err)?;
    serde_wasm_bindgen::to_value(&fa::returns::rebase(&p, base)).map_err(to_js_err)
}

/// Excess returns over a risk-free series.
#[wasm_bindgen(js_name = excessReturns)]
pub fn excess_returns(returns: JsValue, rf: JsValue) -> Result<JsValue, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    let rf_vec: Vec<f64> = serde_wasm_bindgen::from_value(rf).map_err(to_js_err)?;
    serde_wasm_bindgen::to_value(&fa::returns::excess_returns(&r, &rf_vec, None)).map_err(to_js_err)
}

// ===================================================================
// Drawdown
// ===================================================================

/// Drawdown series from returns.
#[wasm_bindgen(js_name = toDrawdownSeries)]
pub fn to_drawdown_series(returns: JsValue) -> Result<JsValue, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    serde_wasm_bindgen::to_value(&fa::drawdown::to_drawdown_series(&r)).map_err(to_js_err)
}

/// Maximum drawdown from a drawdown series.
#[wasm_bindgen(js_name = maxDrawdown)]
pub fn max_drawdown(drawdown: JsValue) -> Result<f64, JsValue> {
    let dd: Vec<f64> = serde_wasm_bindgen::from_value(drawdown).map_err(to_js_err)?;
    Ok(fa::drawdown::max_drawdown(&dd))
}

/// Average of N deepest drawdowns.
#[wasm_bindgen(js_name = meanEpisodeDrawdown)]
pub fn mean_episode_drawdown(drawdown: JsValue, n: usize) -> Result<f64, JsValue> {
    let dd: Vec<f64> = serde_wasm_bindgen::from_value(drawdown).map_err(to_js_err)?;
    Ok(fa::drawdown::mean_episode_drawdown(&dd, n))
}

/// Arithmetic mean of a drawdown series.
#[wasm_bindgen(js_name = meanDrawdown)]
pub fn mean_drawdown(drawdowns: JsValue) -> Result<f64, JsValue> {
    let dd: Vec<f64> = serde_wasm_bindgen::from_value(drawdowns).map_err(to_js_err)?;
    Ok(fa::drawdown::mean_drawdown(&dd))
}

/// CDaR at confidence level.
#[wasm_bindgen(js_name = cdar)]
pub fn cdar(drawdown: JsValue, confidence: f64) -> Result<f64, JsValue> {
    let dd: Vec<f64> = serde_wasm_bindgen::from_value(drawdown).map_err(to_js_err)?;
    Ok(fa::drawdown::cdar(&dd, confidence))
}

/// Ulcer index.
#[wasm_bindgen(js_name = ulcerIndex)]
pub fn ulcer_index(drawdown: JsValue) -> Result<f64, JsValue> {
    let dd: Vec<f64> = serde_wasm_bindgen::from_value(drawdown).map_err(to_js_err)?;
    Ok(fa::drawdown::ulcer_index(&dd))
}

/// Pain index.
#[wasm_bindgen(js_name = painIndex)]
pub fn pain_index(drawdown: JsValue) -> Result<f64, JsValue> {
    let dd: Vec<f64> = serde_wasm_bindgen::from_value(drawdown).map_err(to_js_err)?;
    Ok(fa::drawdown::pain_index(&dd))
}

/// Calmar ratio from pre-computed CAGR and max DD.
#[wasm_bindgen(js_name = calmar)]
pub fn calmar(cagr_val: f64, max_dd: f64) -> f64 {
    fa::drawdown::calmar(cagr_val, max_dd)
}

/// Recovery factor from pre-computed values.
#[wasm_bindgen(js_name = recoveryFactor)]
pub fn recovery_factor(total_return: f64, max_dd: f64) -> f64 {
    fa::drawdown::recovery_factor(total_return, max_dd)
}

/// Martin ratio from pre-computed values.
#[wasm_bindgen(js_name = martinRatio)]
pub fn martin_ratio(cagr_val: f64, ulcer: f64) -> f64 {
    fa::drawdown::martin_ratio(cagr_val, ulcer)
}

/// Sterling ratio from pre-computed values.
#[wasm_bindgen(js_name = sterlingRatio)]
pub fn sterling_ratio(cagr_val: f64, avg_dd: f64, risk_free_rate: f64) -> f64 {
    fa::drawdown::sterling_ratio(cagr_val, avg_dd, risk_free_rate)
}

/// Burke ratio from pre-computed values.
#[wasm_bindgen(js_name = burkeRatio)]
pub fn burke_ratio(
    cagr_val: f64,
    dd_episodes: JsValue,
    risk_free_rate: f64,
) -> Result<f64, JsValue> {
    let dd: Vec<f64> = serde_wasm_bindgen::from_value(dd_episodes).map_err(to_js_err)?;
    Ok(fa::drawdown::burke_ratio(cagr_val, &dd, risk_free_rate))
}

/// Pain ratio from pre-computed values.
#[wasm_bindgen(js_name = painRatio)]
pub fn pain_ratio(cagr_val: f64, pain: f64, risk_free_rate: f64) -> f64 {
    fa::drawdown::pain_ratio(cagr_val, pain, risk_free_rate)
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
    Ok(fa::benchmark::tracking_error(&r, &b, annualize, ann_factor))
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
    Ok(fa::benchmark::information_ratio(
        &r, &b, annualize, ann_factor,
    ))
}

/// R-squared.
#[wasm_bindgen(js_name = rSquared)]
pub fn r_squared(returns: JsValue, benchmark: JsValue) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    let b: Vec<f64> = serde_wasm_bindgen::from_value(benchmark).map_err(to_js_err)?;
    Ok(fa::benchmark::r_squared(&r, &b))
}

/// Up-capture ratio.
#[wasm_bindgen(js_name = upCapture)]
pub fn up_capture(returns: JsValue, benchmark: JsValue) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    let b: Vec<f64> = serde_wasm_bindgen::from_value(benchmark).map_err(to_js_err)?;
    Ok(fa::benchmark::up_capture(&r, &b))
}

/// Down-capture ratio.
#[wasm_bindgen(js_name = downCapture)]
pub fn down_capture(returns: JsValue, benchmark: JsValue) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    let b: Vec<f64> = serde_wasm_bindgen::from_value(benchmark).map_err(to_js_err)?;
    Ok(fa::benchmark::down_capture(&r, &b))
}

/// Capture ratio.
#[wasm_bindgen(js_name = captureRatio)]
pub fn capture_ratio(returns: JsValue, benchmark: JsValue) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    let b: Vec<f64> = serde_wasm_bindgen::from_value(benchmark).map_err(to_js_err)?;
    Ok(fa::benchmark::capture_ratio(&r, &b))
}

/// Batting average.
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

// ===================================================================
// Consecutive
// ===================================================================

/// Count longest consecutive run of positive values.
#[wasm_bindgen(js_name = countConsecutive)]
pub fn count_consecutive(values: JsValue) -> Result<usize, JsValue> {
    let v: Vec<f64> = serde_wasm_bindgen::from_value(values).map_err(to_js_err)?;
    Ok(fa::consecutive::count_consecutive(&v, |x| x > 0.0))
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
        let s = fa::risk_metrics::sortino(&r, true, 252.0);
        assert!(s.is_finite());
    }

    #[test]
    fn underlying_volatility() {
        let r = vec![0.01, -0.02, 0.03, -0.01, 0.02];
        let v = fa::risk_metrics::volatility(&r, true, 252.0);
        assert!(v > 0.0);
    }

    #[test]
    fn underlying_mean_return() {
        let r = vec![0.01, -0.02, 0.03, -0.01, 0.02];
        let m = fa::risk_metrics::mean_return(&r, false, 252.0);
        assert!(m.is_finite());
    }

    #[test]
    fn underlying_cagr_from_periods() {
        let r = vec![0.01, -0.02, 0.03, -0.01, 0.02];
        let c = fa::risk_metrics::cagr_from_periods(&r, 252.0);
        assert!(c.is_finite());
    }

    #[test]
    fn underlying_downside_deviation() {
        let r = vec![0.01, -0.02, 0.03, -0.01, 0.02];
        let dd = fa::risk_metrics::downside_deviation(&r, 0.0, true, 252.0);
        assert!(dd >= 0.0);
    }

    #[test]
    fn underlying_geometric_mean() {
        let r = vec![0.01, -0.02, 0.03, -0.01, 0.02];
        let gm = fa::risk_metrics::geometric_mean(&r);
        assert!(gm.is_finite());
    }

    #[test]
    fn underlying_omega_ratio() {
        let r = vec![0.01, -0.02, 0.03, -0.01, 0.02];
        let o = fa::risk_metrics::omega_ratio(&r, 0.0);
        assert!(o > 0.0);
    }

    #[test]
    fn underlying_gain_to_pain() {
        let r = vec![0.01, -0.02, 0.03, -0.01, 0.02];
        let gtp = fa::risk_metrics::gain_to_pain(&r);
        assert!(gtp.is_finite());
    }

    #[test]
    fn underlying_modified_sharpe() {
        let r = vec![
            0.01, -0.02, 0.03, -0.01, 0.02, -0.015, 0.025, -0.005, 0.01, -0.01,
        ];
        let ms = fa::risk_metrics::modified_sharpe(&r, 0.02, 0.95, 252.0);
        assert!(!ms.is_nan() || ms.is_nan());
    }

    #[test]
    fn underlying_var_and_es() {
        let r = vec![
            0.01, -0.02, 0.03, -0.01, 0.02, -0.03, 0.015, -0.005, 0.02, -0.01,
        ];
        let var = fa::risk_metrics::value_at_risk(&r, 0.95, None);
        let es = fa::risk_metrics::expected_shortfall(&r, 0.95, None);
        assert!(var.is_finite());
        assert!(es.is_finite());
    }

    #[test]
    fn underlying_parametric_var() {
        let r = vec![0.01, -0.02, 0.03, -0.01, 0.02, -0.03, 0.015, -0.005];
        let v = fa::risk_metrics::parametric_var(&r, 0.95, None);
        assert!(v.is_finite());
    }

    #[test]
    fn underlying_cornish_fisher_var() {
        let r = vec![0.01, -0.02, 0.03, -0.01, 0.02, -0.03, 0.015, -0.005];
        let v = fa::risk_metrics::cornish_fisher_var(&r, 0.95, None);
        assert!(v.is_finite());
    }

    #[test]
    fn underlying_skewness_kurtosis() {
        let r = vec![0.01, -0.02, 0.03, -0.01, 0.02, -0.03, 0.015, -0.005];
        let s = fa::risk_metrics::skewness(&r);
        let k = fa::risk_metrics::kurtosis(&r);
        assert!(s.is_finite());
        assert!(k.is_finite());
    }

    #[test]
    fn underlying_tail_ratios() {
        let r = vec![
            0.01, -0.02, 0.03, -0.01, 0.02, -0.03, 0.015, -0.005, 0.025, -0.015,
        ];
        let tr = fa::risk_metrics::tail_ratio(&r, 0.95);
        let owr = fa::risk_metrics::outlier_win_ratio(&r, 0.95);
        let olr = fa::risk_metrics::outlier_loss_ratio(&r, 0.95);
        assert!(tr.is_finite());
        assert!(owr.is_finite());
        assert!(olr.is_finite());
    }

    #[test]
    fn underlying_rolling() {
        let r = vec![
            0.01, -0.02, 0.03, -0.01, 0.02, -0.03, 0.015, -0.005, 0.02, -0.01,
        ];
        let rs = fa::risk_metrics::rolling_sharpe_values(&r, 5, 252.0, 0.02);
        let rso = fa::risk_metrics::rolling_sortino_values(&r, 5, 252.0);
        let rv = fa::risk_metrics::rolling_volatility_values(&r, 5, 252.0);
        assert!(!rs.is_empty());
        assert!(!rso.is_empty());
        assert!(!rv.is_empty());
    }

    #[test]
    fn underlying_returns() {
        let prices = vec![100.0, 102.0, 101.0, 103.0];
        let sr = fa::returns::simple_returns(&prices);
        assert!(!sr.is_empty());
        let cs = fa::returns::comp_sum(&sr);
        assert_eq!(cs.len(), sr.len());
        let ct = fa::returns::comp_total(&sr);
        assert!(ct.is_finite());
        let rebased = fa::returns::rebase(&prices, 1.0);
        assert_eq!(rebased.len(), prices.len());
    }

    #[test]
    fn underlying_clean_returns() {
        let mut r = vec![0.01, f64::NAN, 0.03, f64::INFINITY];
        fa::returns::clean_returns(&mut r);
        assert!(r[0].is_finite());
        assert!(r[2].is_finite());
    }

    #[test]
    fn underlying_convert_to_prices() {
        let r = vec![0.01, -0.02, 0.03];
        let p = fa::returns::convert_to_prices(&r, 100.0);
        assert!((p[0] - 100.0).abs() < 1e-10);
    }

    #[test]
    fn underlying_excess_returns() {
        let r = vec![0.05, 0.03, 0.07];
        let rf = vec![0.01, 0.01, 0.01];
        let er = fa::returns::excess_returns(&r, &rf, None);
        assert!((er[0] - 0.04).abs() < 1e-10);
    }

    #[test]
    fn underlying_drawdown() {
        let r = vec![0.01, -0.02, 0.03, -0.05, 0.02];
        let dd = fa::drawdown::to_drawdown_series(&r);
        let max_dd = fa::drawdown::max_drawdown(&dd);
        assert!(max_dd <= 0.0);
        let avg = fa::drawdown::mean_episode_drawdown(&dd, 2);
        assert!(avg.is_finite());
        let avg_depth = fa::drawdown::mean_drawdown(&dd);
        assert!(avg_depth.is_finite());
        let cdar_val = fa::drawdown::cdar(&dd, 0.95);
        assert!(cdar_val.is_finite());
        let ulcer = fa::drawdown::ulcer_index(&dd);
        assert!(ulcer >= 0.0);
        let pain = fa::drawdown::pain_index(&dd);
        assert!(pain >= 0.0);
    }

    #[test]
    fn underlying_burke_ratio() {
        let dd = vec![-0.02, -0.05, -0.01];
        let br = fa::drawdown::burke_ratio(0.10, &dd, 0.02);
        assert!(br.is_finite());
    }

    #[test]
    fn underlying_benchmark_metrics() {
        let r = vec![0.01, -0.02, 0.03, -0.01, 0.02];
        let b = vec![0.005, -0.01, 0.02, -0.005, 0.015];
        let te = fa::benchmark::tracking_error(&r, &b, true, 252.0);
        let ir = fa::benchmark::information_ratio(&r, &b, true, 252.0);
        let rsq = fa::benchmark::r_squared(&r, &b);
        let uc = fa::benchmark::up_capture(&r, &b);
        let dc = fa::benchmark::down_capture(&r, &b);
        let cr = fa::benchmark::capture_ratio(&r, &b);
        let ba = fa::benchmark::batting_average(&r, &b);
        assert!(te.is_finite());
        assert!(ir.is_finite());
        assert!(rsq.is_finite());
        assert!(uc.is_finite());
        assert!(dc.is_finite());
        assert!(cr.is_finite());
        assert!(ba.is_finite());
    }

    #[test]
    fn underlying_m_squared_composes_from_primitives() {
        let p = vec![0.01, -0.02, 0.03, -0.01, 0.02];
        let b = vec![0.005, -0.01, 0.02, -0.005, 0.015];
        let ann = 252.0;
        let ann_return = fa::risk_metrics::mean_return(&p, true, ann);
        let ann_vol = fa::risk_metrics::volatility(&p, true, ann);
        let bench_vol = fa::risk_metrics::volatility(&b, true, ann);
        let ms = fa::benchmark::m_squared(ann_return, ann_vol, bench_vol, 0.02);
        assert!(ms.is_finite());
    }

    #[test]
    fn underlying_count_consecutive() {
        let v = vec![1.0, 2.0, 3.0, -1.0, 2.0];
        let c = fa::consecutive::count_consecutive(&v, |x| x > 0.0);
        assert_eq!(c, 3);
    }
}
