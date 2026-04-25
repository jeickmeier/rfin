use crate::utils::to_js_err;
use finstack_analytics as fa;
use wasm_bindgen::prelude::*;

use super::support::{parse_cagr_convention, parse_iso_date, parse_iso_dates};

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

/// Definition of a ruin event for Monte Carlo ruin estimation.
#[wasm_bindgen(js_name = RuinDefinition)]
pub struct WasmRuinDefinition {
    inner: fa::risk_metrics::RuinDefinition,
}

#[wasm_bindgen(js_class = RuinDefinition)]
impl WasmRuinDefinition {
    /// Ruin if wealth falls below `floor_fraction` of initial wealth.
    #[wasm_bindgen(js_name = wealthFloor)]
    pub fn wealth_floor(floor_fraction: f64) -> Self {
        Self {
            inner: fa::risk_metrics::RuinDefinition::WealthFloor { floor_fraction },
        }
    }

    /// Ruin if terminal wealth is below `floor_fraction` of initial wealth.
    #[wasm_bindgen(js_name = terminalFloor)]
    pub fn terminal_floor(floor_fraction: f64) -> Self {
        Self {
            inner: fa::risk_metrics::RuinDefinition::TerminalFloor { floor_fraction },
        }
    }

    /// Ruin if drawdown exceeds `max_drawdown` (positive threshold).
    #[wasm_bindgen(js_name = drawdownBreach)]
    pub fn drawdown_breach(max_drawdown: f64) -> Self {
        Self {
            inner: fa::risk_metrics::RuinDefinition::DrawdownBreach { max_drawdown },
        }
    }
}

/// Configuration for Monte Carlo ruin estimation.
#[wasm_bindgen(js_name = RuinModel)]
pub struct WasmRuinModel {
    inner: fa::risk_metrics::RuinModel,
}

#[wasm_bindgen(js_class = RuinModel)]
impl WasmRuinModel {
    /// Construct a ruin simulation model with optional overrides for defaults.
    #[wasm_bindgen(constructor)]
    pub fn new(
        horizon_periods: Option<usize>,
        n_paths: Option<usize>,
        block_size: Option<usize>,
        seed: Option<u64>,
        confidence_level: Option<f64>,
    ) -> Self {
        let defaults = fa::risk_metrics::RuinModel::default();
        Self {
            inner: fa::risk_metrics::RuinModel {
                horizon_periods: horizon_periods.unwrap_or(defaults.horizon_periods),
                n_paths: n_paths.unwrap_or(defaults.n_paths),
                block_size: block_size.unwrap_or(defaults.block_size),
                seed: seed.unwrap_or(defaults.seed),
                confidence_level: confidence_level.unwrap_or(defaults.confidence_level),
            },
        }
    }
}

/// Sharpe ratio from pre-computed annualized return and volatility.
#[wasm_bindgen(js_name = sharpe)]
pub fn sharpe(ann_return: f64, ann_vol: f64, risk_free_rate: f64) -> f64 {
    fa::risk_metrics::sharpe(ann_return, ann_vol, risk_free_rate)
}

/// Sortino ratio (excess return per unit of downside deviation).
#[wasm_bindgen(js_name = sortino)]
pub fn sortino(
    returns: JsValue,
    annualize: bool,
    ann_factor: f64,
    mar: f64,
) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::risk_metrics::sortino(&r, annualize, ann_factor, mar))
}

/// Volatility (standard deviation of returns), optionally annualized.
#[wasm_bindgen(js_name = volatility)]
pub fn volatility(returns: JsValue, annualize: bool, ann_factor: f64) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::risk_metrics::volatility(&r, annualize, ann_factor))
}

/// Arithmetic mean return, optionally annualized.
#[wasm_bindgen(js_name = meanReturn)]
pub fn mean_return(returns: JsValue, annualize: bool, ann_factor: f64) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::risk_metrics::mean_return(&r, annualize, ann_factor))
}

/// Compound annual growth rate using the supplied annualization basis.
#[wasm_bindgen(js_name = cagr)]
pub fn cagr(returns: JsValue, basis: &WasmCagrBasis) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::risk_metrics::cagr(&r, basis.inner))
}

/// Downside deviation relative to a minimum acceptable return.
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

/// Omega ratio (probability-weighted gains / probability-weighted losses).
#[wasm_bindgen(js_name = omegaRatio)]
pub fn omega_ratio(returns: JsValue, threshold: f64) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::risk_metrics::omega_ratio(&r, threshold))
}

/// Gain-to-pain ratio (sum of positive returns / sum of absolute negative returns).
#[wasm_bindgen(js_name = gainToPain)]
pub fn gain_to_pain(returns: JsValue) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::risk_metrics::gain_to_pain(&r))
}

/// Modified Sharpe ratio using Cornish-Fisher VaR as the risk measure.
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

/// Historical Value-at-Risk at the given confidence level.
#[wasm_bindgen(js_name = valueAtRisk)]
pub fn value_at_risk(returns: JsValue, confidence: f64) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::risk_metrics::value_at_risk(&r, confidence))
}

/// Expected Shortfall (CVaR) at the given confidence level.
#[wasm_bindgen(js_name = expectedShortfall)]
pub fn expected_shortfall(returns: JsValue, confidence: f64) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::risk_metrics::expected_shortfall(&r, confidence))
}

/// Parametric VaR assuming a Gaussian distribution.
#[wasm_bindgen(js_name = parametricVar)]
pub fn parametric_var(
    returns: JsValue,
    confidence: f64,
    ann_factor: Option<f64>,
) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::risk_metrics::parametric_var(&r, confidence, ann_factor))
}

/// Cornish-Fisher VaR adjusting for skewness and excess kurtosis.
#[wasm_bindgen(js_name = cornishFisherVar)]
pub fn cornish_fisher_var(
    returns: JsValue,
    confidence: f64,
    ann_factor: Option<f64>,
) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::risk_metrics::cornish_fisher_var(
        &r, confidence, ann_factor,
    ))
}

/// Sample skewness of returns.
#[wasm_bindgen(js_name = skewness)]
pub fn skewness(returns: JsValue) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::risk_metrics::skewness(&r))
}

/// Excess kurtosis of returns.
#[wasm_bindgen(js_name = kurtosis)]
pub fn kurtosis(returns: JsValue) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::risk_metrics::kurtosis(&r))
}

/// Tail ratio: |upper tail| / |lower tail|.
#[wasm_bindgen(js_name = tailRatio)]
pub fn tail_ratio(returns: JsValue, confidence: f64) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::risk_metrics::tail_ratio(&r, confidence))
}

/// Fraction of returns above the upper quantile threshold (outlier wins).
#[wasm_bindgen(js_name = outlierWinRatio)]
pub fn outlier_win_ratio(returns: JsValue, confidence: f64) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::risk_metrics::outlier_win_ratio(&r, confidence))
}

/// Fraction of returns below the lower quantile threshold (outlier losses).
#[wasm_bindgen(js_name = outlierLossRatio)]
pub fn outlier_loss_ratio(returns: JsValue, confidence: f64) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::risk_metrics::outlier_loss_ratio(&r, confidence))
}

/// Monte Carlo ruin probability estimation.
#[wasm_bindgen(js_name = estimateRuin)]
pub fn estimate_ruin(
    returns: JsValue,
    definition: &WasmRuinDefinition,
    model: &WasmRuinModel,
) -> Result<JsValue, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    let estimate = fa::risk_metrics::estimate_ruin(&r, definition.inner, &model.inner);
    serde_wasm_bindgen::to_value(&estimate).map_err(to_js_err)
}

/// Rolling Sharpe ratio over a sliding window.
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
    let rd = parse_iso_dates(&date_strs)?;
    let result = fa::risk_metrics::rolling_sharpe(&r, &rd, window, ann_factor, risk_free_rate);
    serde_wasm_bindgen::to_value(&result).map_err(to_js_err)
}

/// Rolling Sortino ratio over a sliding window.
#[wasm_bindgen(js_name = rollingSortino)]
pub fn rolling_sortino(
    returns: JsValue,
    dates: JsValue,
    window: usize,
    ann_factor: f64,
) -> Result<JsValue, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    let date_strs: Vec<String> = serde_wasm_bindgen::from_value(dates).map_err(to_js_err)?;
    let rd = parse_iso_dates(&date_strs)?;
    let result = fa::risk_metrics::rolling_sortino(&r, &rd, window, ann_factor);
    serde_wasm_bindgen::to_value(&result).map_err(to_js_err)
}

/// Rolling annualized volatility over a sliding window.
#[wasm_bindgen(js_name = rollingVolatility)]
pub fn rolling_volatility(
    returns: JsValue,
    dates: JsValue,
    window: usize,
    ann_factor: f64,
) -> Result<JsValue, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    let date_strs: Vec<String> = serde_wasm_bindgen::from_value(dates).map_err(to_js_err)?;
    let rd = parse_iso_dates(&date_strs)?;
    let result = fa::risk_metrics::rolling_volatility(&r, &rd, window, ann_factor);
    serde_wasm_bindgen::to_value(&result).map_err(to_js_err)
}
