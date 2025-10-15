//! Scenario application for portfolios in WASM.
///
/// This module is only available when the `scenarios` feature is enabled.

use crate::core::config::JsFinstackConfig;
use crate::core::market_data::context::JsMarketContext;
use crate::portfolio::portfolio::JsPortfolio;
use crate::portfolio::valuation::JsPortfolioValuation;
use crate::scenarios::spec::JsScenarioSpec;
use wasm_bindgen::prelude::*;

/// Apply a scenario to a portfolio.
///
/// Transforms the portfolio by applying scenario operations. The original portfolio
/// is not modified; a new portfolio with transformed positions is returned.
///
/// # Arguments
///
/// * `portfolio` - Portfolio to transform
/// * `scenario` - Scenario specification to apply
/// * `market_context` - Market data context
///
/// # Returns
///
/// Transformed portfolio
///
/// # Throws
///
/// Error if scenario application fails
///
/// # Examples
///
/// ```javascript
/// const transformed = applyScenario(portfolio, scenario, marketContext);
/// ```
#[wasm_bindgen(js_name = applyScenario)]
pub fn js_apply_scenario(
    portfolio: &JsPortfolio,
    scenario: &JsScenarioSpec,
    market_context: &JsMarketContext,
) -> Result<JsPortfolio, JsValue> {
    let market_ctx = market_context.inner();
    
    let (transformed, _market, _report) = finstack_portfolio::apply_scenario(
        &portfolio.inner,
        &scenario.inner,
        market_ctx,
    )
    .map_err(|e| JsValue::from_str(&e.to_string()))?;

    Ok(JsPortfolio::from_inner(transformed))
}

/// Apply a scenario to a portfolio and revalue it.
///
/// Convenience function that applies a scenario and then values the resulting portfolio.
/// Equivalent to calling applyScenario followed by valuePortfolio.
///
/// # Arguments
///
/// * `portfolio` - Portfolio to transform and value
/// * `scenario` - Scenario specification to apply
/// * `market_context` - Market data context
/// * `config` - Finstack configuration (optional, uses default if not provided)
///
/// # Returns
///
/// Portfolio valuation results
///
/// # Throws
///
/// Error if scenario application or valuation fails
///
/// # Examples
///
/// ```javascript
/// const valuation = applyAndRevalue(portfolio, scenario, marketContext, config);
/// console.log(valuation.totalBaseCcy);
/// ```
#[wasm_bindgen(js_name = applyAndRevalue)]
pub fn js_apply_and_revalue(
    portfolio: &JsPortfolio,
    scenario: &JsScenarioSpec,
    market_context: &JsMarketContext,
    config: Option<JsFinstackConfig>,
) -> Result<JsPortfolioValuation, JsValue> {
    let default_cfg = finstack_core::config::FinstackConfig::default();
    let cfg_ref = match &config {
        Some(c) => c.inner(),
        None => &default_cfg,
    };

    let market_ctx = market_context.inner();

    let (valuation, _report) = finstack_portfolio::apply_and_revalue(
        &portfolio.inner,
        &scenario.inner,
        market_ctx,
        cfg_ref,
    )
    .map_err(|e| JsValue::from_str(&e.to_string()))?;

    Ok(JsPortfolioValuation::from_inner(valuation))
}

