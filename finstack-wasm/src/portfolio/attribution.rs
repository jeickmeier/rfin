//! Portfolio-level P&L attribution bindings for WASM.

use crate::core::config::JsFinstackConfig;
use crate::core::dates::FsDate;
use crate::core::market_data::context::JsMarketContext;
use crate::core::money::JsMoney;
use crate::portfolio::portfolio::JsPortfolio;
use crate::valuations::attribution::WasmAttributionMethod;
use finstack_portfolio::PortfolioAttribution;
use finstack_valuations::attribution::PnlAttribution;
use js_sys::Object;
use wasm_bindgen::prelude::*;

/// Position-level P&L attribution result.
#[wasm_bindgen(js_name = PnlAttribution)]
pub struct JsPnlAttribution {
    inner: PnlAttribution,
}

#[wasm_bindgen(js_class = PnlAttribution)]
impl JsPnlAttribution {
    #[wasm_bindgen(getter, js_name = totalPnl)]
    pub fn total_pnl(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.total_pnl)
    }

    #[wasm_bindgen(getter)]
    pub fn carry(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.carry)
    }

    #[wasm_bindgen(getter, js_name = ratesCurvesPnl)]
    pub fn rates_curves_pnl(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.rates_curves_pnl)
    }

    #[wasm_bindgen(getter, js_name = creditCurvesPnl)]
    pub fn credit_curves_pnl(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.credit_curves_pnl)
    }

    #[wasm_bindgen(getter, js_name = inflationCurvesPnl)]
    pub fn inflation_curves_pnl(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.inflation_curves_pnl)
    }

    #[wasm_bindgen(getter, js_name = correlationsPnl)]
    pub fn correlations_pnl(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.correlations_pnl)
    }

    #[wasm_bindgen(getter, js_name = fxPnl)]
    pub fn fx_pnl(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.fx_pnl)
    }

    #[wasm_bindgen(getter, js_name = volPnl)]
    pub fn vol_pnl(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.vol_pnl)
    }

    #[wasm_bindgen(getter, js_name = modelParamsPnl)]
    pub fn model_params_pnl(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.model_params_pnl)
    }

    #[wasm_bindgen(getter, js_name = marketScalarsPnl)]
    pub fn market_scalars_pnl(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.market_scalars_pnl)
    }

    #[wasm_bindgen(getter)]
    pub fn residual(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.residual)
    }

    /// Optional rates detail as a plain object.
    #[wasm_bindgen(getter, js_name = ratesDetail)]
    pub fn rates_detail(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner.rates_detail)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize rates detail: {}", e)))
    }

    /// Optional credit detail as a plain object.
    #[wasm_bindgen(getter, js_name = creditDetail)]
    pub fn credit_detail(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner.credit_detail)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize credit detail: {}", e)))
    }

    /// Optional inflation detail as a plain object.
    #[wasm_bindgen(getter, js_name = inflationDetail)]
    pub fn inflation_detail(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner.inflation_detail)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize inflation detail: {}", e)))
    }

    /// Optional correlations detail as a plain object.
    #[wasm_bindgen(getter, js_name = correlationsDetail)]
    pub fn correlations_detail(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner.correlations_detail).map_err(|e| {
            JsValue::from_str(&format!("Failed to serialize correlations detail: {}", e))
        })
    }

    /// Optional FX detail as a plain object.
    #[wasm_bindgen(getter, js_name = fxDetail)]
    pub fn fx_detail(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner.fx_detail)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize FX detail: {}", e)))
    }

    /// Optional volatility detail as a plain object.
    #[wasm_bindgen(getter, js_name = volDetail)]
    pub fn vol_detail(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner.vol_detail)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize vol detail: {}", e)))
    }

    /// Optional scalars detail as a plain object.
    #[wasm_bindgen(getter, js_name = scalarsDetail)]
    pub fn scalars_detail(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner.scalars_detail)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize scalars detail: {}", e)))
    }

    #[wasm_bindgen(js_name = toCsv)]
    pub fn to_csv(&self) -> String {
        self.inner.to_csv()
    }

    #[wasm_bindgen(js_name = explain)]
    pub fn explain(&self) -> String {
        self.inner.explain()
    }
}

impl JsPnlAttribution {
    pub(crate) fn from_inner(inner: PnlAttribution) -> Self {
        Self { inner }
    }
}

/// Portfolio-level P&L attribution result.
#[wasm_bindgen(js_name = PortfolioAttribution)]
pub struct JsPortfolioAttribution {
    inner: PortfolioAttribution,
}

#[wasm_bindgen(js_class = PortfolioAttribution)]
impl JsPortfolioAttribution {
    #[wasm_bindgen(getter, js_name = totalPnl)]
    pub fn total_pnl(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.total_pnl)
    }

    #[wasm_bindgen(getter)]
    pub fn carry(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.carry)
    }

    #[wasm_bindgen(getter, js_name = ratesCurvesPnl)]
    pub fn rates_curves_pnl(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.rates_curves_pnl)
    }

    #[wasm_bindgen(getter, js_name = creditCurvesPnl)]
    pub fn credit_curves_pnl(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.credit_curves_pnl)
    }

    #[wasm_bindgen(getter, js_name = inflationCurvesPnl)]
    pub fn inflation_curves_pnl(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.inflation_curves_pnl)
    }

    #[wasm_bindgen(getter, js_name = correlationsPnl)]
    pub fn correlations_pnl(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.correlations_pnl)
    }

    #[wasm_bindgen(getter, js_name = fxPnl)]
    pub fn fx_pnl(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.fx_pnl)
    }

    #[wasm_bindgen(getter, js_name = fxTranslationPnl)]
    pub fn fx_translation_pnl(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.fx_translation_pnl)
    }

    #[wasm_bindgen(getter, js_name = volPnl)]
    pub fn vol_pnl(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.vol_pnl)
    }

    #[wasm_bindgen(getter, js_name = modelParamsPnl)]
    pub fn model_params_pnl(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.model_params_pnl)
    }

    #[wasm_bindgen(getter, js_name = marketScalarsPnl)]
    pub fn market_scalars_pnl(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.market_scalars_pnl)
    }

    #[wasm_bindgen(getter)]
    pub fn residual(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.residual)
    }

    /// Attribution by position as `{ [positionId]: PnlAttribution }`.
    #[wasm_bindgen(getter, js_name = byPosition)]
    pub fn by_position(&self) -> Result<JsValue, JsValue> {
        let dict = Object::new();
        for (pos_id, attr) in &self.inner.by_position {
            js_sys::Reflect::set(
                &dict,
                &JsValue::from_str(pos_id.as_str()),
                &JsValue::from(JsPnlAttribution::from_inner(attr.clone())),
            )?;
        }
        Ok(JsValue::from(dict))
    }

    #[wasm_bindgen(getter, js_name = ratesDetail)]
    pub fn rates_detail(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner.rates_detail)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize rates detail: {}", e)))
    }

    #[wasm_bindgen(getter, js_name = creditDetail)]
    pub fn credit_detail(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner.credit_detail)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize credit detail: {}", e)))
    }

    #[wasm_bindgen(getter, js_name = inflationDetail)]
    pub fn inflation_detail(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner.inflation_detail)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize inflation detail: {}", e)))
    }

    #[wasm_bindgen(getter, js_name = correlationsDetail)]
    pub fn correlations_detail(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner.correlations_detail).map_err(|e| {
            JsValue::from_str(&format!("Failed to serialize correlations detail: {}", e))
        })
    }

    #[wasm_bindgen(getter, js_name = fxDetail)]
    pub fn fx_detail(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner.fx_detail)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize FX detail: {}", e)))
    }

    #[wasm_bindgen(getter, js_name = volDetail)]
    pub fn vol_detail(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner.vol_detail)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize vol detail: {}", e)))
    }

    #[wasm_bindgen(getter, js_name = scalarsDetail)]
    pub fn scalars_detail(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner.scalars_detail)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize scalars detail: {}", e)))
    }

    #[wasm_bindgen(js_name = toCsv)]
    pub fn to_csv(&self) -> String {
        self.inner.to_csv()
    }

    #[wasm_bindgen(js_name = positionDetailToCsv)]
    pub fn position_detail_to_csv(&self) -> String {
        self.inner.position_detail_to_csv()
    }

    #[wasm_bindgen(js_name = explain)]
    pub fn explain(&self) -> String {
        self.inner.explain()
    }
}

impl JsPortfolioAttribution {
    pub(crate) fn from_inner(inner: PortfolioAttribution) -> Self {
        Self { inner }
    }
}

/// Perform portfolio-level P&L attribution.
#[wasm_bindgen(js_name = attributePortfolioPnl)]
pub fn js_attribute_portfolio_pnl(
    portfolio: &JsPortfolio,
    market_t0: &JsMarketContext,
    market_t1: &JsMarketContext,
    as_of_t0: &FsDate,
    as_of_t1: &FsDate,
    method: &WasmAttributionMethod,
    config: Option<JsFinstackConfig>,
) -> Result<JsPortfolioAttribution, JsValue> {
    let default_cfg = finstack_core::config::FinstackConfig::default();
    let cfg_ref = match &config {
        Some(c) => c.inner(),
        None => &default_cfg,
    };

    finstack_portfolio::attribute_portfolio_pnl(
        &portfolio.inner,
        market_t0.inner(),
        market_t1.inner(),
        as_of_t0.inner(),
        as_of_t1.inner(),
        cfg_ref,
        method.inner.clone(),
    )
    .map(JsPortfolioAttribution::from_inner)
    .map_err(|e| JsValue::from_str(&e.to_string()))
}
