//! Portfolio valuation for WASM.

use crate::core::config::JsFinstackConfig;
use crate::core::market_data::context::JsMarketContext;
use crate::core::money::JsMoney;
use crate::portfolio::positions::JsPortfolio;
use crate::valuations::results::JsValuationResult;
use finstack_portfolio::valuation::{
    value_portfolio_with_options, PortfolioValuation, PortfolioValuationOptions, PositionValue,
};
use js_sys::Object;
use std::cell::RefCell;
use wasm_bindgen::prelude::*;

/// Options controlling portfolio valuation behaviour.
#[wasm_bindgen(js_name = PortfolioValuationOptions)]
pub struct JsPortfolioValuationOptions {
    inner: PortfolioValuationOptions,
}

#[wasm_bindgen(js_class = PortfolioValuationOptions)]
impl JsPortfolioValuationOptions {
    /// Create valuation options.
    #[wasm_bindgen(constructor)]
    pub fn new(strict_risk: bool) -> JsPortfolioValuationOptions {
        JsPortfolioValuationOptions {
            inner: PortfolioValuationOptions {
                strict_risk,
                additional_metrics: None,
                replace_standard_metrics: false,
            },
        }
    }

    /// Whether missing risk metrics are treated as errors.
    #[wasm_bindgen(getter, js_name = strictRisk)]
    pub fn strict_risk(&self) -> bool {
        self.inner.strict_risk
    }
}

impl JsPortfolioValuationOptions {
    pub(crate) fn inner(&self) -> &PortfolioValuationOptions {
        &self.inner
    }
}

/// Result of valuing a single position.
///
/// Holds both native-currency and base-currency valuations.
///
/// # Examples
///
/// ```javascript
/// const positionValue = portfolioValuation.getPositionValue("POS_1");
/// console.log(positionValue.valueNative);
/// console.log(positionValue.valueBase);
/// ```
#[wasm_bindgen]
pub struct JsPositionValue {
    inner: PositionValue,
}

#[wasm_bindgen]
impl JsPositionValue {
    /// Get the position identifier.
    ///
    /// # Returns
    ///
    /// Position ID as string
    #[wasm_bindgen(getter, js_name = positionId)]
    pub fn position_id(&self) -> String {
        self.inner.position_id.to_string()
    }

    /// Get the entity identifier.
    ///
    /// # Returns
    ///
    /// Entity ID as string
    #[wasm_bindgen(getter, js_name = entityId)]
    pub fn entity_id(&self) -> String {
        self.inner.entity_id.to_string()
    }

    /// Get the value in the instrument's native currency.
    ///
    /// # Returns
    ///
    /// Money value in native currency
    #[wasm_bindgen(getter, js_name = valueNative)]
    pub fn value_native(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.value_native)
    }

    /// Get the value converted to portfolio base currency.
    ///
    /// # Returns
    ///
    /// Money value in base currency
    #[wasm_bindgen(getter, js_name = valueBase)]
    pub fn value_base(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.value_base)
    }

    /// Get the full valuation result with metrics.
    ///
    /// # Returns
    ///
    /// ValuationResult if available, undefined otherwise
    #[wasm_bindgen(getter, js_name = valuationResult)]
    pub fn valuation_result(&self) -> Option<JsValuationResult> {
        self.inner
            .valuation_result
            .as_ref()
            .map(|r| JsValuationResult::new(r.clone()))
    }

    /// Create from JSON representation.
    ///
    /// Note: ValuationResult field is excluded from serialization.
    ///
    /// # Arguments
    ///
    /// * `value` - JavaScript object
    ///
    /// # Returns
    ///
    /// PositionValue instance
    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> Result<JsPositionValue, JsValue> {
        serde_wasm_bindgen::from_value(value)
            .map(|inner| JsPositionValue { inner })
            .map_err(|e| JsValue::from_str(&format!("Failed to deserialize PositionValue: {}", e)))
    }

    /// Convert to JSON representation.
    ///
    /// Note: ValuationResult field is excluded.
    ///
    /// # Returns
    ///
    /// JavaScript object
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize PositionValue: {}", e)))
    }
}

impl JsPositionValue {
    pub(crate) fn from_inner(inner: PositionValue) -> Self {
        Self { inner }
    }
}

/// Complete portfolio valuation results.
///
/// Provides per-position valuations, totals by entity, and the grand total.
///
/// # Examples
///
/// ```javascript
/// const valuation = valuePortfolio(portfolio, marketContext, config);
/// console.log(valuation.totalBaseCcy);
/// console.log(valuation.byEntity["ENTITY_A"]);
/// ```
#[wasm_bindgen]
pub struct JsPortfolioValuation {
    pub(crate) inner: PortfolioValuation,
    #[wasm_bindgen(skip)]
    position_values_cache: RefCell<Option<JsValue>>,
}

#[wasm_bindgen]
impl JsPortfolioValuation {
    /// Get values for each position as a JavaScript object.
    ///
    /// # Returns
    ///
    /// JavaScript object mapping position IDs to PositionValue instances
    #[wasm_bindgen(getter, js_name = positionValues)]
    pub fn position_values(&self) -> Result<JsValue, JsValue> {
        if let Some(cached) = self.position_values_cache.borrow().as_ref() {
            return Ok(cached.clone());
        }

        let obj = Object::new();
        for (id, value) in &self.inner.position_values {
            let js_value = JsPositionValue::from_inner(value.clone());
            js_sys::Reflect::set(
                &obj,
                &JsValue::from_str(id.as_str()),
                &JsValue::from(js_value),
            )?;
        }

        let js_obj = JsValue::from(obj);
        *self.position_values_cache.borrow_mut() = Some(js_obj.clone());
        Ok(js_obj)
    }

    /// Get the total portfolio value in base currency.
    ///
    /// # Returns
    ///
    /// Total value as Money
    #[wasm_bindgen(getter, js_name = totalBaseCcy)]
    pub fn total_base_ccy(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.total_base_ccy)
    }

    /// Get aggregated values by entity as a JavaScript object.
    ///
    /// # Returns
    ///
    /// JavaScript object mapping entity IDs to Money values
    #[wasm_bindgen(getter, js_name = byEntity)]
    pub fn by_entity(&self) -> Result<JsValue, JsValue> {
        let obj = Object::new();
        for (id, money) in &self.inner.by_entity {
            let js_money = JsMoney::from_inner(*money);
            js_sys::Reflect::set(
                &obj,
                &JsValue::from_str(id.as_str()),
                &JsValue::from(js_money),
            )?;
        }
        Ok(JsValue::from(obj))
    }

    /// Get the value for a specific position.
    ///
    /// # Arguments
    ///
    /// * `position_id` - Identifier to query
    ///
    /// # Returns
    ///
    /// The position value if found, undefined otherwise
    #[wasm_bindgen(js_name = getPositionValue)]
    pub fn get_position_value(&self, position_id: &str) -> Option<JsPositionValue> {
        self.inner
            .get_position_value(position_id)
            .map(|v| JsPositionValue::from_inner(v.clone()))
    }

    /// Get the total value for a specific entity.
    ///
    /// # Arguments
    ///
    /// * `entity_id` - Entity identifier to query
    ///
    /// # Returns
    ///
    /// The entity's total value if found, undefined otherwise
    #[wasm_bindgen(js_name = getEntityValue)]
    pub fn get_entity_value(&self, entity_id: &str) -> Option<JsMoney> {
        self.inner
            .get_entity_value(entity_id)
            .map(|m| JsMoney::from_inner(*m))
    }

    /// Create from JSON representation.
    ///
    /// Note: ValuationResult fields are excluded from serialization.
    ///
    /// # Arguments
    ///
    /// * `value` - JavaScript object
    ///
    /// # Returns
    ///
    /// PortfolioValuation instance
    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> Result<JsPortfolioValuation, JsValue> {
        serde_wasm_bindgen::from_value(value)
            .map(|inner| JsPortfolioValuation {
                inner,
                position_values_cache: RefCell::new(None),
            })
            .map_err(|e| {
                JsValue::from_str(&format!("Failed to deserialize PortfolioValuation: {}", e))
            })
    }

    /// Convert to JSON representation.
    ///
    /// Note: ValuationResult fields are excluded.
    ///
    /// # Returns
    ///
    /// JavaScript object
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner).map_err(|e| {
            JsValue::from_str(&format!("Failed to serialize PortfolioValuation: {}", e))
        })
    }
}

impl JsPortfolioValuation {
    pub(crate) fn from_inner(inner: PortfolioValuation) -> Self {
        Self {
            inner,
            position_values_cache: RefCell::new(None),
        }
    }
}

/// Value a complete portfolio.
///
/// # Arguments
///
/// * `portfolio` - Portfolio to value
/// * `market_context` - Market data context
/// * `config` - Finstack configuration (optional, uses default if not provided)
///
/// # Returns
///
/// Complete valuation results
///
/// # Throws
///
/// Error if valuation fails
///
/// # Examples
///
/// ```javascript
/// const valuation = valuePortfolio(portfolio, marketContext, config);
/// ```
#[wasm_bindgen(js_name = valuePortfolio)]
pub fn js_value_portfolio(
    portfolio: &JsPortfolio,
    market_context: &JsMarketContext,
    config: Option<JsFinstackConfig>,
) -> Result<JsPortfolioValuation, JsValue> {
    let default_cfg = finstack_core::config::FinstackConfig::default();
    let cfg_ref = match &config {
        Some(c) => c.inner(),
        None => &default_cfg,
    };

    let market_ctx = market_context.inner();

    finstack_portfolio::value_portfolio(&portfolio.inner, market_ctx, cfg_ref)
        .map(JsPortfolioValuation::from_inner)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Value a portfolio with explicit valuation options.
#[wasm_bindgen(js_name = valuePortfolioWithOptions)]
pub fn js_value_portfolio_with_options(
    portfolio: &JsPortfolio,
    market_context: &JsMarketContext,
    options: &JsPortfolioValuationOptions,
    config: Option<JsFinstackConfig>,
) -> Result<JsPortfolioValuation, JsValue> {
    let default_cfg = finstack_core::config::FinstackConfig::default();
    let cfg_ref = match &config {
        Some(c) => c.inner(),
        None => &default_cfg,
    };

    let market_ctx = market_context.inner();

    value_portfolio_with_options(&portfolio.inner, market_ctx, cfg_ref, options.inner())
        .map(JsPortfolioValuation::from_inner)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}
