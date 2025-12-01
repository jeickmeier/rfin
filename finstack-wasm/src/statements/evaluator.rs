//! Evaluator for financial models.

use crate::core::dates::FsDate;
use crate::core::market_data::context::JsMarketContext;
use crate::statements::types::JsFinancialModelSpec;
use finstack_core::dates::PeriodId;
use finstack_statements::evaluator::{Evaluator, Results, ResultsMeta};
use std::str::FromStr;
use wasm_bindgen::prelude::*;

/// Metadata about evaluation results.
///
/// Contains information about the evaluation process including
/// timing, node count, and period count.
#[wasm_bindgen]
pub struct JsResultsMeta {
    inner: ResultsMeta,
}

#[wasm_bindgen]
impl JsResultsMeta {
    /// Evaluation time in milliseconds.
    #[wasm_bindgen(getter, js_name = evalTimeMs)]
    pub fn eval_time_ms(&self) -> Option<u64> {
        self.inner.eval_time_ms
    }

    /// Number of nodes evaluated.
    #[wasm_bindgen(getter, js_name = numNodes)]
    pub fn num_nodes(&self) -> usize {
        self.inner.num_nodes
    }

    /// Number of periods evaluated.
    #[wasm_bindgen(getter, js_name = numPeriods)]
    pub fn num_periods(&self) -> usize {
        self.inner.num_periods
    }

    /// Convert to string representation.
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "ResultsMeta(nodes={}, periods={}, eval_time_ms={:?})",
            self.inner.num_nodes, self.inner.num_periods, self.inner.eval_time_ms
        )
    }
}

impl JsResultsMeta {
    fn new(inner: ResultsMeta) -> Self {
        Self { inner }
    }
}

/// Results from evaluating a financial model.
///
/// Contains node values for each period and evaluation metadata.
#[wasm_bindgen]
pub struct JsResults {
    pub(crate) inner: Results,
}

#[wasm_bindgen]
impl JsResults {
    /// Get the value for a node at a specific period.
    ///
    /// # Arguments
    /// * `node_id` - Node identifier
    /// * `period_id` - Period identifier (string like "2025Q1")
    ///
    /// # Returns
    /// Value if found, null otherwise
    #[wasm_bindgen]
    pub fn get(&self, node_id: &str, period_id: &str) -> Result<Option<f64>, JsValue> {
        let pid = PeriodId::from_str(period_id)
            .map_err(|e| JsValue::from_str(&format!("Invalid period ID '{}': {}", period_id, e)))?;
        Ok(self.inner.get(node_id, &pid))
    }

    /// Get all period values for a specific node.
    ///
    /// # Arguments
    /// * `node_id` - Node identifier
    ///
    /// # Returns
    /// JavaScript object mapping period IDs to values, or null if node not found
    #[wasm_bindgen(js_name = getNode)]
    pub fn get_node(&self, node_id: &str) -> Result<JsValue, JsValue> {
        if let Some(period_map) = self.inner.get_node(node_id) {
            // Convert to JavaScript object
            let obj = js_sys::Object::new();
            for (period_id, value) in period_map {
                let period_str = period_id.to_string();
                js_sys::Reflect::set(
                    &obj,
                    &JsValue::from_str(&period_str),
                    &JsValue::from_f64(*value),
                )?;
            }
            Ok(JsValue::from(obj))
        } else {
            Ok(JsValue::NULL)
        }
    }

    /// Get value or default.
    ///
    /// # Arguments
    /// * `node_id` - Node identifier
    /// * `period_id` - Period identifier
    /// * `default` - Default value if not found
    ///
    /// # Returns
    /// Value or default
    #[wasm_bindgen(js_name = getOr)]
    pub fn get_or(&self, node_id: &str, period_id: &str, default: f64) -> Result<f64, JsValue> {
        let pid = PeriodId::from_str(period_id)
            .map_err(|e| JsValue::from_str(&format!("Invalid period ID '{}': {}", period_id, e)))?;
        Ok(self.inner.get_or(node_id, &pid, default))
    }

    /// Get all period values for a node as an array.
    ///
    /// # Arguments
    /// * `node_id` - Node identifier
    ///
    /// # Returns
    /// Array of [periodId, value] pairs
    #[wasm_bindgen(js_name = allPeriods)]
    pub fn all_periods(&self, node_id: &str) -> JsValue {
        let array = js_sys::Array::new();
        for (period_id, value) in self.inner.all_periods(node_id) {
            let pair = js_sys::Array::new();
            pair.push(&JsValue::from_str(&period_id.to_string()));
            pair.push(&JsValue::from_f64(value));
            array.push(&pair);
        }
        JsValue::from(array)
    }

    /// Get all node results.
    ///
    /// # Returns
    /// JavaScript object mapping node IDs to period value objects
    #[wasm_bindgen(getter)]
    pub fn nodes(&self) -> Result<JsValue, JsValue> {
        let obj = js_sys::Object::new();
        for (node_id, period_map) in &self.inner.nodes {
            let inner_obj = js_sys::Object::new();
            for (period_id, value) in period_map {
                js_sys::Reflect::set(
                    &inner_obj,
                    &JsValue::from_str(&period_id.to_string()),
                    &JsValue::from_f64(*value),
                )?;
            }
            js_sys::Reflect::set(&obj, &JsValue::from_str(node_id), &JsValue::from(inner_obj))?;
        }
        Ok(JsValue::from(obj))
    }

    /// Get evaluation metadata.
    #[wasm_bindgen(getter)]
    pub fn meta(&self) -> JsResultsMeta {
        JsResultsMeta::new(self.inner.meta.clone())
    }

    /// Convert to JSON representation.
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize Results: {}", e)))
    }

    /// Create from JSON representation.
    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> Result<JsResults, JsValue> {
        serde_wasm_bindgen::from_value(value)
            .map(|inner| JsResults { inner })
            .map_err(|e| JsValue::from_str(&format!("Failed to deserialize Results: {}", e)))
    }

    /// Convert to string representation.
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "Results(nodes={}, periods={})",
            self.inner.nodes.len(),
            self.inner.meta.num_periods
        )
    }
}

impl JsResults {
    pub(crate) fn new(inner: Results) -> Self {
        Self { inner }
    }
}

/// Evaluator for financial models.
///
/// The evaluator compiles formulas, resolves dependencies, and evaluates
/// nodes period-by-period according to precedence rules.
///
/// # Example
/// ```javascript
/// const evaluator = new JsEvaluator();
/// const results = evaluator.evaluate(model);
/// console.log(results.get("revenue", "2025Q1"));
/// ```
#[wasm_bindgen]
pub struct JsEvaluator {
    inner: Evaluator,
}

impl Default for JsEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl JsEvaluator {
    /// Create a new evaluator.
    #[wasm_bindgen(constructor)]
    pub fn new() -> JsEvaluator {
        JsEvaluator {
            inner: Evaluator::new(),
        }
    }

    /// Evaluate a financial model over all periods.
    ///
    /// This is a convenience method that evaluates without market context.
    /// If your model uses capital structure with cs.* references, use
    /// `evaluateWithMarketContext` instead.
    ///
    /// # Arguments
    /// * `model` - Financial model specification
    ///
    /// # Returns
    /// Evaluation results
    #[wasm_bindgen]
    pub fn evaluate(&mut self, model: &JsFinancialModelSpec) -> Result<JsResults, JsValue> {
        let results = self
            .inner
            .evaluate(&model.inner)
            .map_err(|e| JsValue::from_str(&format!("Evaluation failed: {}", e)))?;
        Ok(JsResults::new(results))
    }

    /// Evaluate a financial model with market context for pricing.
    ///
    /// This method allows you to provide market context for pricing capital
    /// structure instruments. If capital structure is defined but market context
    /// is not provided, capital structure cashflows will not be computed.
    ///
    /// # Arguments
    /// * `model` - Financial model specification
    /// * `market_ctx` - Market context for pricing instruments
    /// * `as_of` - Valuation date for pricing
    ///
    /// # Returns
    /// Evaluation results
    #[wasm_bindgen(js_name = evaluateWithMarketContext)]
    pub fn evaluate_with_market_context(
        &mut self,
        model: &JsFinancialModelSpec,
        market_ctx: &JsMarketContext,
        as_of: &FsDate,
    ) -> Result<JsResults, JsValue> {
        let results = self
            .inner
            .evaluate_with_market_context(
                &model.inner,
                Some(market_ctx.inner()),
                Some(as_of.inner()),
            )
            .map_err(|e| {
                JsValue::from_str(&format!("Evaluation with market context failed: {}", e))
            })?;
        Ok(JsResults::new(results))
    }
}
