//! Portfolio optimization helpers for WASM.
//!
//! These bindings keep the logic in Rust and only perform type conversions.

use crate::core::config::JsFinstackConfig;
use crate::core::market_data::context::JsMarketContext;
use crate::portfolio::positions::JsPortfolio;
use finstack_portfolio::optimization::optimize_max_yield_with_ccc_limit;
use wasm_bindgen::prelude::*;

/// Optimize a bond portfolio to maximize value‑weighted YTM with a CCC exposure limit.
///
/// Returns a plain JavaScript object mirroring the Rust helper result.
#[wasm_bindgen(js_name = optimizeMaxYieldWithCccLimit)]
pub fn js_optimize_max_yield_with_ccc_limit(
    portfolio: &JsPortfolio,
    market_context: &JsMarketContext,
    ccc_limit: f64,
    strict_risk: bool,
    config: Option<JsFinstackConfig>,
) -> Result<JsValue, JsValue> {
    let default_cfg = finstack_core::config::FinstackConfig::default();
    let cfg_ref = match &config {
        Some(c) => c.inner(),
        None => &default_cfg,
    };

    optimize_max_yield_with_ccc_limit(
        &portfolio.inner,
        market_context.inner(),
        cfg_ref,
        ccc_limit,
        strict_risk,
    )
    .map_err(|e| JsValue::from_str(&e.to_string()))
    .and_then(|res| {
        serde_wasm_bindgen::to_value(&res)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize result: {}", e)))
    })
}
