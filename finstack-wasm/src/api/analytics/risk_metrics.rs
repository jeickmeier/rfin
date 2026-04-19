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

#[wasm_bindgen(js_name = RuinDefinition)]
pub struct WasmRuinDefinition {
    inner: fa::risk_metrics::RuinDefinition,
}

#[wasm_bindgen(js_class = RuinDefinition)]
impl WasmRuinDefinition {
    #[wasm_bindgen(js_name = wealthFloor)]
    pub fn wealth_floor(floor_fraction: f64) -> Self {
        Self {
            inner: fa::risk_metrics::RuinDefinition::WealthFloor { floor_fraction },
        }
    }

    #[wasm_bindgen(js_name = terminalFloor)]
    pub fn terminal_floor(floor_fraction: f64) -> Self {
        Self {
            inner: fa::risk_metrics::RuinDefinition::TerminalFloor { floor_fraction },
        }
    }

    #[wasm_bindgen(js_name = drawdownBreach)]
    pub fn drawdown_breach(max_drawdown: f64) -> Self {
        Self {
            inner: fa::risk_metrics::RuinDefinition::DrawdownBreach { max_drawdown },
        }
    }
}

#[wasm_bindgen(js_name = RuinModel)]
pub struct WasmRuinModel {
    inner: fa::risk_metrics::RuinModel,
}

#[wasm_bindgen(js_class = RuinModel)]
impl WasmRuinModel {
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

#[wasm_bindgen(js_name = sharpe)]
pub fn sharpe(ann_return: f64, ann_vol: f64, risk_free_rate: f64) -> f64 {
    fa::risk_metrics::sharpe(ann_return, ann_vol, risk_free_rate)
}

#[wasm_bindgen(js_name = sortino)]
pub fn sortino(returns: JsValue, annualize: bool, ann_factor: f64) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::risk_metrics::sortino(&r, annualize, ann_factor))
}

#[wasm_bindgen(js_name = volatility)]
pub fn volatility(returns: JsValue, annualize: bool, ann_factor: f64) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::risk_metrics::volatility(&r, annualize, ann_factor))
}

#[wasm_bindgen(js_name = meanReturn)]
pub fn mean_return(returns: JsValue, annualize: bool, ann_factor: f64) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::risk_metrics::mean_return(&r, annualize, ann_factor))
}

#[wasm_bindgen(js_name = cagr)]
pub fn cagr(returns: JsValue, basis: &WasmCagrBasis) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::risk_metrics::cagr(&r, basis.inner))
}

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

#[wasm_bindgen(js_name = geometricMean)]
pub fn geometric_mean(returns: JsValue) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::risk_metrics::geometric_mean(&r))
}

#[wasm_bindgen(js_name = omegaRatio)]
pub fn omega_ratio(returns: JsValue, threshold: f64) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::risk_metrics::omega_ratio(&r, threshold))
}

#[wasm_bindgen(js_name = gainToPain)]
pub fn gain_to_pain(returns: JsValue) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::risk_metrics::gain_to_pain(&r))
}

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

#[wasm_bindgen(js_name = valueAtRisk)]
pub fn value_at_risk(
    returns: JsValue,
    confidence: f64,
    ann_factor: Option<f64>,
) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::risk_metrics::value_at_risk(&r, confidence, ann_factor))
}

#[wasm_bindgen(js_name = expectedShortfall)]
pub fn expected_shortfall(
    returns: JsValue,
    confidence: f64,
    ann_factor: Option<f64>,
) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::risk_metrics::expected_shortfall(
        &r, confidence, ann_factor,
    ))
}

#[wasm_bindgen(js_name = parametricVar)]
pub fn parametric_var(
    returns: JsValue,
    confidence: f64,
    ann_factor: Option<f64>,
) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::risk_metrics::parametric_var(&r, confidence, ann_factor))
}

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

#[wasm_bindgen(js_name = skewness)]
pub fn skewness(returns: JsValue) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::risk_metrics::skewness(&r))
}

#[wasm_bindgen(js_name = kurtosis)]
pub fn kurtosis(returns: JsValue) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::risk_metrics::kurtosis(&r))
}

#[wasm_bindgen(js_name = tailRatio)]
pub fn tail_ratio(returns: JsValue, confidence: f64) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::risk_metrics::tail_ratio(&r, confidence))
}

#[wasm_bindgen(js_name = outlierWinRatio)]
pub fn outlier_win_ratio(returns: JsValue, confidence: f64) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::risk_metrics::outlier_win_ratio(&r, confidence))
}

#[wasm_bindgen(js_name = outlierLossRatio)]
pub fn outlier_loss_ratio(returns: JsValue, confidence: f64) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::risk_metrics::outlier_loss_ratio(&r, confidence))
}

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
