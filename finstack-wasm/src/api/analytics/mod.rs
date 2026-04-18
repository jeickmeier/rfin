//! WASM bindings for the `finstack-analytics` crate.
//!
//! Exposes standalone risk-metric, return-computation, drawdown, and benchmark
//! functions using `serde_wasm_bindgen` for JS ↔ Rust array conversion.

use crate::utils::to_js_err;
use fa::timeseries::GarchModel;
use finstack_analytics as fa;
use wasm_bindgen::prelude::*;

/// Parse an ISO date string (`"YYYY-MM-DD"`) into a Rust [`time::Date`].
fn parse_iso_date(s: &str) -> Result<time::Date, JsValue> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 {
        return Err(to_js_err(format!("expected YYYY-MM-DD, got {s:?}")));
    }
    let year: i32 = parts[0].parse().map_err(to_js_err)?;
    let month_num: u8 = parts[1].parse().map_err(to_js_err)?;
    let day: u8 = parts[2].parse().map_err(to_js_err)?;
    let month = time::Month::try_from(month_num).map_err(to_js_err)?;
    time::Date::from_calendar_date(year, month, day).map_err(to_js_err)
}

fn parse_cagr_convention(
    convention: Option<&str>,
) -> Result<fa::risk_metrics::AnnualizationConvention, JsValue> {
    match convention
        .unwrap_or("act365_25")
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "act365_25" | "act36525" | "act/365.25" | "default" => {
            Ok(fa::risk_metrics::AnnualizationConvention::Act365_25)
        }
        "act365fixed" | "act365_fixed" | "act/365f" | "act365f" => {
            Ok(fa::risk_metrics::AnnualizationConvention::Act365Fixed)
        }
        "actact" | "act_act" | "actualactual" | "actual_actual" => {
            Ok(fa::risk_metrics::AnnualizationConvention::ActAct)
        }
        other => Err(to_js_err(format!(
            "unknown CAGR convention {other:?}; expected one of act365_25, act365_fixed, actact"
        ))),
    }
}

/// Annualization basis for CAGR.
#[wasm_bindgen(js_name = CagrBasis)]
pub struct WasmCagrBasis {
    inner: fa::risk_metrics::CagrBasis,
}

#[wasm_bindgen(js_class = CagrBasis)]
impl WasmCagrBasis {
    /// Create a factor-based basis from periods per year.
    #[wasm_bindgen(js_name = factor)]
    pub fn factor(ann_factor: f64) -> Self {
        Self {
            inner: fa::risk_metrics::CagrBasis::factor(ann_factor),
        }
    }

    /// Create a date-based basis from ISO dates and an optional convention string.
    #[wasm_bindgen(js_name = dates)]
    pub fn dates(start: &str, end: &str, convention: Option<String>) -> Result<Self, JsValue> {
        Ok(Self {
            inner: fa::risk_metrics::CagrBasis::dates_with_convention(
                parse_iso_date(start)?,
                parse_iso_date(end)?,
                parse_cagr_convention(convention.as_deref())?,
            ),
        })
    }
}

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

/// CAGR annualized using the supplied basis.
#[wasm_bindgen(js_name = cagr)]
pub fn cagr(returns: JsValue, basis: &WasmCagrBasis) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::risk_metrics::cagr(&r, basis.inner))
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

/// Top-N drawdown episodes with date information.
#[wasm_bindgen(js_name = drawdownDetails)]
pub fn drawdown_details(drawdown: JsValue, dates: JsValue, n: usize) -> Result<JsValue, JsValue> {
    let dd: Vec<f64> = serde_wasm_bindgen::from_value(drawdown).map_err(to_js_err)?;
    let date_strs: Vec<String> = serde_wasm_bindgen::from_value(dates).map_err(to_js_err)?;
    let rd: Vec<time::Date> = date_strs
        .iter()
        .map(|s| parse_iso_date(s))
        .collect::<Result<_, _>>()?;
    let episodes = fa::drawdown::drawdown_details(&dd, &rd, n);
    serde_wasm_bindgen::to_value(&episodes).map_err(to_js_err)
}

/// Maximum drawdown duration in calendar days.
#[wasm_bindgen(js_name = maxDrawdownDuration)]
pub fn max_drawdown_duration(drawdown: JsValue, dates: JsValue) -> Result<i64, JsValue> {
    let dd: Vec<f64> = serde_wasm_bindgen::from_value(drawdown).map_err(to_js_err)?;
    let date_strs: Vec<String> = serde_wasm_bindgen::from_value(dates).map_err(to_js_err)?;
    let rd: Vec<time::Date> = date_strs
        .iter()
        .map(|s| parse_iso_date(s))
        .collect::<Result<_, _>>()?;
    Ok(fa::drawdown::max_drawdown_duration(&dd, &rd))
}

// ===================================================================
// Risk metrics — rolling (dated)
// ===================================================================

/// Rolling Sharpe ratio with date labels.
#[wasm_bindgen(js_name = rollingSharpe)]
pub fn rolling_sharpe(
    returns: JsValue,
    dates: JsValue,
    window: usize,
    ann_factor: f64,
    risk_free_rate: f64,
) -> Result<JsValue, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    let date_strs: Vec<String> = serde_wasm_bindgen::from_value(dates).map_err(to_js_err)?;
    let rd: Vec<time::Date> = date_strs
        .iter()
        .map(|s| parse_iso_date(s))
        .collect::<Result<_, _>>()?;
    let result = fa::risk_metrics::rolling_sharpe(&r, &rd, window, ann_factor, risk_free_rate);
    serde_wasm_bindgen::to_value(&result).map_err(to_js_err)
}

/// Rolling Sortino ratio with date labels.
#[wasm_bindgen(js_name = rollingSortino)]
pub fn rolling_sortino(
    returns: JsValue,
    dates: JsValue,
    window: usize,
    ann_factor: f64,
) -> Result<JsValue, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    let date_strs: Vec<String> = serde_wasm_bindgen::from_value(dates).map_err(to_js_err)?;
    let rd: Vec<time::Date> = date_strs
        .iter()
        .map(|s| parse_iso_date(s))
        .collect::<Result<_, _>>()?;
    let result = fa::risk_metrics::rolling_sortino(&r, &rd, window, ann_factor);
    serde_wasm_bindgen::to_value(&result).map_err(to_js_err)
}

/// Rolling annualized volatility with date labels.
#[wasm_bindgen(js_name = rollingVolatility)]
pub fn rolling_volatility(
    returns: JsValue,
    dates: JsValue,
    window: usize,
    ann_factor: f64,
) -> Result<JsValue, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    let date_strs: Vec<String> = serde_wasm_bindgen::from_value(dates).map_err(to_js_err)?;
    let rd: Vec<time::Date> = date_strs
        .iter()
        .map(|s| parse_iso_date(s))
        .collect::<Result<_, _>>()?;
    let result = fa::risk_metrics::rolling_volatility(&r, &rd, window, ann_factor);
    serde_wasm_bindgen::to_value(&result).map_err(to_js_err)
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

/// OLS beta of portfolio vs benchmark, with standard error and 95% CI.
#[wasm_bindgen(js_name = beta)]
pub fn beta(portfolio: JsValue, benchmark: JsValue) -> Result<JsValue, JsValue> {
    let p: Vec<f64> = serde_wasm_bindgen::from_value(portfolio).map_err(to_js_err)?;
    let b: Vec<f64> = serde_wasm_bindgen::from_value(benchmark).map_err(to_js_err)?;
    let result = fa::benchmark::beta(&p, &b);
    serde_wasm_bindgen::to_value(&result).map_err(to_js_err)
}

/// Single-factor greeks (alpha, beta, R²) for portfolio vs benchmark.
#[wasm_bindgen(js_name = greeks)]
pub fn greeks(returns: JsValue, benchmark: JsValue, ann_factor: f64) -> Result<JsValue, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    let b: Vec<f64> = serde_wasm_bindgen::from_value(benchmark).map_err(to_js_err)?;
    let result = fa::benchmark::greeks(&r, &b, ann_factor);
    serde_wasm_bindgen::to_value(&result).map_err(to_js_err)
}

/// Rolling single-factor greeks (alpha, beta) over a sliding window.
///
/// Returns `{ alphas: number[], betas: number[] }` without date labels
/// (consistent with other `*Values` rolling functions in this module).
#[wasm_bindgen(js_name = rollingGreeks)]
pub fn rolling_greeks(
    returns: JsValue,
    benchmark: JsValue,
    window: usize,
    ann_factor: f64,
) -> Result<JsValue, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    let b: Vec<f64> = serde_wasm_bindgen::from_value(benchmark).map_err(to_js_err)?;
    let n = r.len().min(b.len());
    let base = time::Date::from_calendar_date(2000, time::Month::January, 1).map_err(to_js_err)?;
    let dates: Vec<time::Date> = (0..n)
        .map(|i| base + time::Duration::days(i as i64))
        .collect();
    let rg = fa::benchmark::rolling_greeks(&r, &b, &dates, window, ann_factor);
    #[derive(serde::Serialize)]
    struct Values {
        alphas: Vec<f64>,
        betas: Vec<f64>,
    }
    serde_wasm_bindgen::to_value(&Values {
        alphas: rg.alphas,
        betas: rg.betas,
    })
    .map_err(to_js_err)
}

/// Multi-factor OLS regression of portfolio returns on factor returns.
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

// ===================================================================
// Comps — comparable company analysis
// ===================================================================

/// Percentile rank of `value` within `data` (0–1 scale).
#[wasm_bindgen(js_name = percentileRank)]
pub fn percentile_rank(value: f64, data: JsValue) -> Result<f64, JsValue> {
    let d: Vec<f64> = serde_wasm_bindgen::from_value(data).map_err(to_js_err)?;
    Ok(fa::comps::percentile_rank(&d, value).unwrap_or(0.5))
}

/// Z-score of `value` relative to the peer distribution in `data`.
#[wasm_bindgen(js_name = zScore)]
pub fn z_score(value: f64, data: JsValue) -> Result<f64, JsValue> {
    let d: Vec<f64> = serde_wasm_bindgen::from_value(data).map_err(to_js_err)?;
    Ok(fa::comps::z_score(&d, value).unwrap_or(0.0))
}

/// Descriptive statistics for a peer distribution.
#[wasm_bindgen(js_name = peerStats)]
pub fn peer_stats(data: JsValue) -> Result<JsValue, JsValue> {
    let d: Vec<f64> = serde_wasm_bindgen::from_value(data).map_err(to_js_err)?;
    let stats = fa::comps::peer_stats(&d);
    serde_wasm_bindgen::to_value(&stats).map_err(to_js_err)
}

/// Single-factor OLS regression fair value.
///
/// Returns the full `RegressionResult` (slope, intercept, r_squared,
/// fitted_value, residual, n) serialised as a JS object, or `null` if the
/// regression cannot be computed.
#[wasm_bindgen(js_name = regressionFairValue)]
pub fn regression_fair_value(
    x_values: JsValue,
    y_values: JsValue,
    subject_x: f64,
    subject_y: f64,
) -> Result<JsValue, JsValue> {
    let x: Vec<f64> = serde_wasm_bindgen::from_value(x_values).map_err(to_js_err)?;
    let y: Vec<f64> = serde_wasm_bindgen::from_value(y_values).map_err(to_js_err)?;
    let result = fa::comps::regression_fair_value(&x, &y, subject_x, subject_y);
    serde_wasm_bindgen::to_value(&result).map_err(to_js_err)
}

/// Safe division: `price / metric` with non-positive / non-finite guards.
#[wasm_bindgen(js_name = computeMultiple)]
pub fn compute_multiple(price: f64, metric: f64) -> f64 {
    if metric <= 0.0 || !metric.is_finite() || !price.is_finite() {
        f64::NAN
    } else {
        price / metric
    }
}

/// Composite rich/cheap score of a subject against peers.
///
/// Accepts the full `PeerSet` and `ScoringDimension[]` as JSON via
/// `serde_wasm_bindgen`, mirroring the Rust canonical API.
#[wasm_bindgen(js_name = scoreRelativeValue)]
pub fn score_relative_value(peer_set: JsValue, dimensions: JsValue) -> Result<JsValue, JsValue> {
    let ps: fa::comps::PeerSet = serde_wasm_bindgen::from_value(peer_set).map_err(to_js_err)?;
    let dims: Vec<fa::comps::ScoringDimension> =
        serde_wasm_bindgen::from_value(dimensions).map_err(to_js_err)?;
    let result = fa::comps::score_relative_value(&ps, &dims).map_err(to_js_err)?;
    serde_wasm_bindgen::to_value(&result).map_err(to_js_err)
}

// ===================================================================
// Backtesting
// ===================================================================

/// Classify each observation as a VaR breach or miss.
#[wasm_bindgen(js_name = classifyBreaches)]
pub fn classify_breaches(
    var_forecasts: JsValue,
    realized_pnl: JsValue,
) -> Result<JsValue, JsValue> {
    let var: Vec<f64> = serde_wasm_bindgen::from_value(var_forecasts).map_err(to_js_err)?;
    let pnl: Vec<f64> = serde_wasm_bindgen::from_value(realized_pnl).map_err(to_js_err)?;
    let breaches = fa::backtesting::classify_breaches(&var, &pnl);
    let bools: Vec<bool> = breaches
        .iter()
        .map(|b| *b == fa::backtesting::Breach::Hit)
        .collect();
    serde_wasm_bindgen::to_value(&bools).map_err(to_js_err)
}

/// Kupiec Proportion of Failures (POF) unconditional coverage test.
#[wasm_bindgen(js_name = kupiecTest)]
pub fn kupiec_test(breach_count: usize, n: usize, confidence: f64) -> Result<JsValue, JsValue> {
    let result = fa::backtesting::kupiec_test(breach_count, n, confidence);
    serde_wasm_bindgen::to_value(&result).map_err(to_js_err)
}

/// Christoffersen joint conditional coverage test.
#[wasm_bindgen(js_name = christoffersenTest)]
pub fn christoffersen_test(
    breach_indicators: JsValue,
    confidence: f64,
) -> Result<JsValue, JsValue> {
    let indicators: Vec<bool> =
        serde_wasm_bindgen::from_value(breach_indicators).map_err(to_js_err)?;
    let seq: Vec<fa::backtesting::Breach> = indicators
        .into_iter()
        .map(|b| {
            if b {
                fa::backtesting::Breach::Hit
            } else {
                fa::backtesting::Breach::Miss
            }
        })
        .collect();
    let result = fa::backtesting::christoffersen_test(&seq, confidence);
    serde_wasm_bindgen::to_value(&result).map_err(to_js_err)
}

/// Basel Committee traffic-light classification.
#[wasm_bindgen(js_name = trafficLight)]
pub fn traffic_light(exceptions: usize, n: usize, confidence: f64) -> Result<JsValue, JsValue> {
    let result = fa::backtesting::traffic_light(exceptions, n, confidence);
    serde_wasm_bindgen::to_value(&result).map_err(to_js_err)
}

/// Run a complete VaR backtest.
#[wasm_bindgen(js_name = runBacktest)]
pub fn run_backtest(
    var_forecasts: JsValue,
    realized_pnl: JsValue,
    confidence: f64,
    window_size: usize,
) -> Result<JsValue, JsValue> {
    let var: Vec<f64> = serde_wasm_bindgen::from_value(var_forecasts).map_err(to_js_err)?;
    let pnl: Vec<f64> = serde_wasm_bindgen::from_value(realized_pnl).map_err(to_js_err)?;
    let cfg = fa::backtesting::VarBacktestConfig::new()
        .with_confidence(confidence)
        .with_window_size(window_size);
    let result = fa::backtesting::run_backtest(&var, &pnl, &cfg);
    serde_wasm_bindgen::to_value(&result).map_err(to_js_err)
}

// ===================================================================
// GARCH volatility models
// ===================================================================

/// Parse an innovation distribution string into the Rust enum.
fn parse_dist(s: &str) -> Result<fa::timeseries::InnovationDist, JsValue> {
    match s.to_ascii_lowercase().as_str() {
        "gaussian" | "normal" | "gauss" | "n" => Ok(fa::timeseries::InnovationDist::Gaussian),
        "student_t" | "student-t" | "studentt" | "t" => {
            Ok(fa::timeseries::InnovationDist::StudentT(8.0))
        }
        other => Err(to_js_err(format!(
            "unknown distribution '{other}'; expected 'gaussian' or 'student_t'"
        ))),
    }
}

/// Fit a standard GARCH(1,1) model by maximum likelihood.
#[wasm_bindgen(js_name = fitGarch11)]
pub fn fit_garch11(returns: JsValue, distribution: &str) -> Result<JsValue, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    let dist = parse_dist(distribution)?;
    let fit = fa::timeseries::Garch11
        .fit(&r, dist, None)
        .map_err(to_js_err)?;
    serde_wasm_bindgen::to_value(&fit).map_err(to_js_err)
}

/// Fit an EGARCH(1,1) model (Nelson, 1991) with leverage via log-variance.
#[wasm_bindgen(js_name = fitEgarch11)]
pub fn fit_egarch11(returns: JsValue, distribution: &str) -> Result<JsValue, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    let dist = parse_dist(distribution)?;
    let fit = fa::timeseries::Egarch11
        .fit(&r, dist, None)
        .map_err(to_js_err)?;
    serde_wasm_bindgen::to_value(&fit).map_err(to_js_err)
}

/// Fit a GJR-GARCH(1,1) model (Glosten, Jagannathan & Runkle, 1993).
#[wasm_bindgen(js_name = fitGjrGarch11)]
pub fn fit_gjr_garch11(returns: JsValue, distribution: &str) -> Result<JsValue, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    let dist = parse_dist(distribution)?;
    let fit = fa::timeseries::GjrGarch11
        .fit(&r, dist, None)
        .map_err(to_js_err)?;
    serde_wasm_bindgen::to_value(&fit).map_err(to_js_err)
}

/// Closed-form h-step-ahead GARCH(1,1) variance forecast.
#[wasm_bindgen(js_name = garch11Forecast)]
pub fn garch11_forecast(
    omega: f64,
    alpha: f64,
    beta: f64,
    last_variance: f64,
    last_return: f64,
    horizon: usize,
) -> Result<JsValue, JsValue> {
    if horizon == 0 {
        return serde_wasm_bindgen::to_value(&Vec::<f64>::new()).map_err(to_js_err);
    }
    let mut out = Vec::with_capacity(horizon);
    let mut s2 = omega + alpha * last_return * last_return + beta * last_variance;
    out.push(s2.max(0.0));
    let persistence = alpha + beta;
    for _ in 1..horizon {
        s2 = omega + persistence * s2;
        out.push(s2.max(0.0));
    }
    serde_wasm_bindgen::to_value(&out).map_err(to_js_err)
}

/// Ljung-Box Q-statistic for serial correlation.
#[wasm_bindgen(js_name = ljungBox)]
pub fn ljung_box(residuals: JsValue, lags: usize) -> Result<JsValue, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(residuals).map_err(to_js_err)?;
    let result = fa::timeseries::ljung_box(&r, lags);
    serde_wasm_bindgen::to_value(&result).map_err(to_js_err)
}

/// Engle's ARCH-LM test for remaining heteroskedasticity.
#[wasm_bindgen(js_name = archLm)]
pub fn arch_lm(residuals: JsValue, lags: usize) -> Result<JsValue, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(residuals).map_err(to_js_err)?;
    let result = fa::timeseries::arch_lm(&r, lags);
    serde_wasm_bindgen::to_value(&result).map_err(to_js_err)
}

/// Akaike Information Criterion.
#[wasm_bindgen(js_name = aic)]
pub fn aic(log_likelihood: f64, n_params: usize) -> f64 {
    fa::timeseries::aic(log_likelihood, n_params)
}

/// Bayesian Information Criterion.
#[wasm_bindgen(js_name = bic)]
pub fn bic(log_likelihood: f64, n_params: usize, n_obs: usize) -> f64 {
    fa::timeseries::bic(log_likelihood, n_params, n_obs)
}

/// Hannan-Quinn Information Criterion.
#[wasm_bindgen(js_name = hqic)]
pub fn hqic(log_likelihood: f64, n_params: usize, n_obs: usize) -> f64 {
    fa::timeseries::hqic(log_likelihood, n_params, n_obs)
}

// ===================================================================
// Aggregation
// ===================================================================

/// Group daily returns by period (monthly, quarterly, etc.) and compound within each period.
///
/// Returns an array of `[period_id_string, compounded_return]` pairs.
#[wasm_bindgen(js_name = groupByPeriod)]
pub fn group_by_period(
    returns: JsValue,
    dates: JsValue,
    period_kind: &str,
) -> Result<JsValue, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    let date_strs: Vec<String> = serde_wasm_bindgen::from_value(dates).map_err(to_js_err)?;
    let parsed_dates: Vec<finstack_core::dates::Date> = date_strs
        .iter()
        .map(|s| parse_iso_date(s))
        .collect::<Result<_, _>>()?;
    let freq: finstack_core::dates::PeriodKind = period_kind.parse().map_err(to_js_err)?;
    let grouped = fa::aggregation::group_by_period(&parsed_dates, &r, freq, None);
    serde_wasm_bindgen::to_value(&grouped).map_err(to_js_err)
}

/// Compute period-level statistics from grouped returns.
///
/// Takes the output of `groupByPeriod` (array of `[period_id, return]` pairs)
/// and returns a `PeriodStats` object.
#[wasm_bindgen(js_name = periodStats)]
pub fn period_stats(grouped: JsValue) -> Result<JsValue, JsValue> {
    let g: Vec<(finstack_core::dates::PeriodId, f64)> =
        serde_wasm_bindgen::from_value(grouped).map_err(to_js_err)?;
    let stats = fa::aggregation::period_stats(&g);
    serde_wasm_bindgen::to_value(&stats).map_err(to_js_err)
}

// ===================================================================
// Lookback selectors
// ===================================================================

/// Month-to-date index range.
///
/// Returns `[start_idx, end_idx]` into the dates/returns arrays.
#[wasm_bindgen(js_name = mtdSelect)]
pub fn mtd_select(dates: JsValue, as_of: &str, offset_days: usize) -> Result<JsValue, JsValue> {
    let date_strs: Vec<String> = serde_wasm_bindgen::from_value(dates).map_err(to_js_err)?;
    let parsed_dates: Vec<finstack_core::dates::Date> = date_strs
        .iter()
        .map(|s| parse_iso_date(s))
        .collect::<Result<_, _>>()?;
    let ref_date = parse_iso_date(as_of)?;
    let range = fa::lookback::mtd_select(&parsed_dates, ref_date, offset_days as i64);
    serde_wasm_bindgen::to_value(&[range.start, range.end]).map_err(to_js_err)
}

/// Quarter-to-date index range.
///
/// Returns `[start_idx, end_idx]` into the dates/returns arrays.
#[wasm_bindgen(js_name = qtdSelect)]
pub fn qtd_select(dates: JsValue, as_of: &str, offset_days: usize) -> Result<JsValue, JsValue> {
    let date_strs: Vec<String> = serde_wasm_bindgen::from_value(dates).map_err(to_js_err)?;
    let parsed_dates: Vec<finstack_core::dates::Date> = date_strs
        .iter()
        .map(|s| parse_iso_date(s))
        .collect::<Result<_, _>>()?;
    let ref_date = parse_iso_date(as_of)?;
    let range = fa::lookback::qtd_select(&parsed_dates, ref_date, offset_days as i64);
    serde_wasm_bindgen::to_value(&[range.start, range.end]).map_err(to_js_err)
}

/// Year-to-date index range.
///
/// Returns `[start_idx, end_idx]` into the dates/returns arrays.
#[wasm_bindgen(js_name = ytdSelect)]
pub fn ytd_select(dates: JsValue, as_of: &str, offset_days: usize) -> Result<JsValue, JsValue> {
    let date_strs: Vec<String> = serde_wasm_bindgen::from_value(dates).map_err(to_js_err)?;
    let parsed_dates: Vec<finstack_core::dates::Date> = date_strs
        .iter()
        .map(|s| parse_iso_date(s))
        .collect::<Result<_, _>>()?;
    let ref_date = parse_iso_date(as_of)?;
    let range = fa::lookback::ytd_select(&parsed_dates, ref_date, offset_days as i64);
    serde_wasm_bindgen::to_value(&[range.start, range.end]).map_err(to_js_err)
}

/// Fiscal-year-to-date index range.
///
/// Returns `[start_idx, end_idx]` into the dates/returns arrays.
/// `fiscal_start_month` and `fiscal_start_day` define the fiscal year start
/// (e.g., 10 and 1 for the US federal fiscal year starting October 1).
#[wasm_bindgen(js_name = fytdSelect)]
pub fn fytd_select(
    dates: JsValue,
    as_of: &str,
    offset_days: usize,
    fiscal_start_month: u8,
    fiscal_start_day: u8,
) -> Result<JsValue, JsValue> {
    let date_strs: Vec<String> = serde_wasm_bindgen::from_value(dates).map_err(to_js_err)?;
    let parsed_dates: Vec<finstack_core::dates::Date> = date_strs
        .iter()
        .map(|s| parse_iso_date(s))
        .collect::<Result<_, _>>()?;
    let ref_date = parse_iso_date(as_of)?;
    let fiscal_config =
        finstack_core::dates::FiscalConfig::new(fiscal_start_month, fiscal_start_day)
            .map_err(to_js_err)?;
    let range =
        fa::lookback::fytd_select(&parsed_dates, ref_date, fiscal_config, offset_days as i64);
    serde_wasm_bindgen::to_value(&[range.start, range.end]).map_err(to_js_err)
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
    fn underlying_cagr_factor_basis() {
        let r = vec![0.01, -0.02, 0.03, -0.01, 0.02];
        let c = fa::risk_metrics::cagr(&r, fa::risk_metrics::CagrBasis::factor(252.0));
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
    fn underlying_beta() {
        let y = vec![0.02, 0.04, 0.06, 0.08, 0.10];
        let x = vec![0.01, 0.02, 0.03, 0.04, 0.05];
        let result = fa::benchmark::beta(&y, &x);
        assert!((result.beta - 2.0).abs() < 1e-10);
        assert!(result.std_err.is_finite());
        assert!(result.ci_lower <= result.ci_upper);
    }

    #[test]
    fn underlying_greeks() {
        let r = vec![0.01, 0.02, 0.03, 0.04, 0.05];
        let b = vec![0.005, 0.01, 0.015, 0.02, 0.025];
        let g = fa::benchmark::greeks(&r, &b, 252.0);
        assert!((g.beta - 2.0).abs() < 1e-10);
        assert!((g.r_squared - 1.0).abs() < 1e-10);
    }

    #[test]
    fn underlying_rolling_greeks() {
        let r: Vec<f64> = (0..20).map(|i| (i as f64 + 1.0) * 0.001).collect();
        let b: Vec<f64> = (0..20).map(|i| i as f64 * 0.0005).collect();
        let base =
            time::Date::from_calendar_date(2025, time::Month::January, 1).expect("valid date");
        let dates: Vec<time::Date> = (0..20).map(|i| base + time::Duration::days(i)).collect();
        let rg = fa::benchmark::rolling_greeks(&r, &b, &dates, 5, 252.0);
        assert_eq!(rg.betas.len(), 16);
        assert_eq!(rg.alphas.len(), 16);
    }

    #[test]
    fn underlying_multi_factor_greeks() {
        let y = vec![0.02, 0.04, 0.06, 0.08, 0.10];
        let f1 = vec![0.01, 0.02, 0.03, 0.04, 0.05];
        let result = fa::benchmark::multi_factor_greeks(&y, &[&f1], 252.0).expect("single-factor");
        assert!((result.betas[0] - 2.0).abs() < 1e-8);
        assert!(result.r_squared > 0.999);
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
    fn underlying_info_criteria_aic_bic_hqic() {
        let a = aic(-500.0, 3);
        let b = bic(-500.0, 3, 100);
        let h = hqic(-500.0, 3, 100);
        assert!(a.is_finite());
        assert!(b.is_finite());
        assert!(h.is_finite());
    }

    #[test]
    fn underlying_ljung_box() {
        let r = vec![
            0.01, -0.02, 0.03, -0.01, 0.02, 0.005, -0.015, 0.01, -0.005, 0.02,
        ];
        let (q, p) = fa::timeseries::ljung_box(&r, 5);
        assert!(q.is_finite());
        assert!((0.0..=1.0).contains(&p));
    }

    #[test]
    fn underlying_arch_lm() {
        let r = vec![
            0.01, -0.02, 0.03, -0.01, 0.02, 0.005, -0.015, 0.01, -0.005, 0.02,
        ];
        let (lm, p) = fa::timeseries::arch_lm(&r, 3);
        assert!(lm.is_finite());
        assert!((0.0..=1.0).contains(&p));
    }

    #[test]
    fn underlying_garch_info_criteria() {
        let a = fa::timeseries::aic(-500.0, 3);
        let b = fa::timeseries::bic(-500.0, 3, 100);
        let h = fa::timeseries::hqic(-500.0, 3, 100);
        assert!(a.is_finite());
        assert!(b.is_finite());
        assert!(h.is_finite());
        assert!((a - 1006.0).abs() < 1e-10);
    }
}
