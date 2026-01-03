//! WASM bindings for statement metric normalization helpers.
//!
//! Provides types for normalizing financial metrics (e.g., EBITDA adjustments).

use crate::core::error::js_error;
use crate::statements::evaluator::JsResults;
use crate::utils::json::{from_js_value, to_js_value};
use finstack_core::dates::PeriodId;
use finstack_statements::adjustments::engine::NormalizationEngine;
use finstack_statements::adjustments::types::{
    Adjustment, AppliedAdjustment, NormalizationConfig, NormalizationResult,
};
use indexmap::IndexMap;
use std::collections::HashMap;
use std::str::FromStr;
use wasm_bindgen::prelude::*;

/// Configuration for normalizing a financial metric (e.g., EBITDA).
#[wasm_bindgen(js_name = NormalizationConfig)]
pub struct JsNormalizationConfig {
    inner: NormalizationConfig,
}

#[wasm_bindgen(js_class = NormalizationConfig)]
impl JsNormalizationConfig {
    /// Create a new normalization config for a target node.
    ///
    /// # Arguments
    /// * `target_node` - The target node to normalize (e.g., "EBITDA")
    #[wasm_bindgen(constructor)]
    pub fn new(target_node: &str) -> JsNormalizationConfig {
        JsNormalizationConfig {
            inner: NormalizationConfig::new(target_node),
        }
    }

    /// Add an adjustment to the configuration.
    #[wasm_bindgen(js_name = addAdjustment)]
    pub fn add_adjustment(&mut self, adjustment: &JsAdjustment) {
        self.inner.adjustments.push(adjustment.inner.clone());
    }

    /// Serialize to JavaScript object.
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }

    /// Serialize to JSON string.
    #[wasm_bindgen(js_name = toJsonString)]
    pub fn to_json_string(&self) -> Result<String, JsValue> {
        serde_json::to_string_pretty(&self.inner).map_err(|e| js_error(e.to_string()))
    }

    /// Deserialize from JSON string.
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(json_str: &str) -> Result<JsNormalizationConfig, JsValue> {
        let inner: NormalizationConfig =
            serde_json::from_str(json_str).map_err(|e| js_error(e.to_string()))?;
        Ok(JsNormalizationConfig { inner })
    }
}

/// Specification for a single adjustment (add-back or deduction).
#[wasm_bindgen(js_name = Adjustment)]
pub struct JsAdjustment {
    inner: Adjustment,
}

#[wasm_bindgen(js_class = Adjustment)]
impl JsAdjustment {
    /// Create a fixed adjustment with specified amounts per period.
    ///
    /// # Arguments
    /// * `id` - Unique identifier for this adjustment
    /// * `name` - Human-readable name
    /// * `amounts` - Object mapping period IDs to amounts
    #[wasm_bindgen(js_name = fixed)]
    pub fn fixed(id: &str, name: &str, amounts: JsValue) -> Result<JsAdjustment, JsValue> {
        let amounts_map: HashMap<String, f64> = from_js_value(amounts)?;
        let mut indexed_amounts: IndexMap<PeriodId, f64> = IndexMap::new();
        for (period_str, amount) in amounts_map {
            let period_id = PeriodId::from_str(&period_str)
                .map_err(|e| js_error(format!("Invalid period ID '{}': {}", period_str, e)))?;
            indexed_amounts.insert(period_id, amount);
        }
        Ok(JsAdjustment {
            inner: Adjustment::fixed(id, name, indexed_amounts),
        })
    }

    /// Create a percentage-based adjustment.
    ///
    /// # Arguments
    /// * `id` - Unique identifier for this adjustment
    /// * `name` - Human-readable name
    /// * `node_id` - The node to calculate the percentage from
    /// * `percentage` - The percentage (e.g., 0.05 for 5%)
    #[wasm_bindgen(js_name = percentage)]
    pub fn percentage(
        id: &str,
        name: &str,
        node_id: &str,
        percentage: f64,
    ) -> Result<JsAdjustment, JsValue> {
        Ok(JsAdjustment {
            inner: Adjustment::percentage(id, name, node_id, percentage),
        })
    }

    /// Add a cap to this adjustment.
    ///
    /// # Arguments
    /// * `base_node` - The node to calculate the cap against (or null for fixed cap)
    /// * `value` - The cap value (percentage of base_node, or fixed amount if base_node is null)
    #[wasm_bindgen(js_name = withCap)]
    pub fn with_cap(&self, base_node: Option<String>, value: f64) -> JsAdjustment {
        JsAdjustment {
            inner: self.inner.clone().with_cap(base_node, value),
        }
    }

    /// Serialize to JavaScript object.
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }

    /// Serialize to JSON string.
    #[wasm_bindgen(js_name = toJsonString)]
    pub fn to_json_string(&self) -> Result<String, JsValue> {
        serde_json::to_string_pretty(&self.inner).map_err(|e| js_error(e.to_string()))
    }

    /// Deserialize from JSON string.
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(json_str: &str) -> Result<JsAdjustment, JsValue> {
        let inner: Adjustment =
            serde_json::from_str(json_str).map_err(|e| js_error(e.to_string()))?;
        Ok(JsAdjustment { inner })
    }
}

/// Result of applying a single adjustment to a period.
#[wasm_bindgen(js_name = AppliedAdjustment)]
pub struct JsAppliedAdjustment {
    inner: AppliedAdjustment,
}

#[wasm_bindgen(js_class = AppliedAdjustment)]
impl JsAppliedAdjustment {
    /// Name of the adjustment.
    #[wasm_bindgen(getter)]
    pub fn name(&self) -> String {
        self.inner.name.clone()
    }

    /// Raw (uncapped) amount of the adjustment.
    #[wasm_bindgen(getter, js_name = rawAmount)]
    pub fn raw_amount(&self) -> f64 {
        self.inner.raw_amount
    }

    /// Amount after capping (if applicable).
    #[wasm_bindgen(getter, js_name = cappedAmount)]
    pub fn capped_amount(&self) -> f64 {
        self.inner.capped_amount
    }

    /// Whether the adjustment was capped.
    #[wasm_bindgen(getter, js_name = isCapped)]
    pub fn is_capped(&self) -> bool {
        self.inner.is_capped
    }
}

/// Result of normalization for a single period.
#[wasm_bindgen(js_name = NormalizationResult)]
pub struct JsNormalizationResult {
    inner: NormalizationResult,
}

#[wasm_bindgen(js_class = NormalizationResult)]
impl JsNormalizationResult {
    /// Period ID.
    #[wasm_bindgen(getter)]
    pub fn period(&self) -> String {
        self.inner.period.to_string()
    }

    /// Base value before adjustments.
    #[wasm_bindgen(getter, js_name = baseValue)]
    pub fn base_value(&self) -> f64 {
        self.inner.base_value
    }

    /// Final value after all adjustments.
    #[wasm_bindgen(getter, js_name = finalValue)]
    pub fn final_value(&self) -> f64 {
        self.inner.final_value
    }

    /// Get the applied adjustments as a JavaScript array.
    #[wasm_bindgen(getter)]
    pub fn adjustments(&self) -> js_sys::Array {
        let arr = js_sys::Array::new();
        for adj in &self.inner.adjustments {
            arr.push(&JsAppliedAdjustment { inner: adj.clone() }.into());
        }
        arr
    }
}

/// Engine for calculating normalized metrics.
#[wasm_bindgen(js_name = NormalizationEngine)]
pub struct JsNormalizationEngine;

#[wasm_bindgen(js_class = NormalizationEngine)]
impl JsNormalizationEngine {
    /// Normalize a target node across all periods in the results.
    ///
    /// # Arguments
    /// * `results` - The evaluation results
    /// * `config` - The normalization configuration
    ///
    /// # Returns
    /// Array of normalization results, one per period.
    #[wasm_bindgen(js_name = normalize)]
    pub fn normalize(
        results: &JsResults,
        config: &JsNormalizationConfig,
    ) -> Result<js_sys::Array, JsValue> {
        let norm_results = NormalizationEngine::normalize(&results.inner, &config.inner)
            .map_err(|e| js_error(e.to_string()))?;

        let arr = js_sys::Array::new();
        for nr in norm_results {
            arr.push(&JsNormalizationResult { inner: nr }.into());
        }
        Ok(arr)
    }
}
