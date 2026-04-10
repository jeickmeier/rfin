//! Portfolio results for WASM.

use crate::core::money::JsMoney;
use crate::portfolio::metrics::JsPortfolioMetrics;
use crate::portfolio::valuation::JsPortfolioValuation;
use finstack_portfolio::results::PortfolioResult;
use wasm_bindgen::prelude::*;

/// Complete results from portfolio evaluation.
///
/// Contains valuation, metrics, and metadata about the calculation.
///
/// # Examples
///
/// ```javascript
/// const results = new PortfolioResult(valuation, metrics, meta);
/// console.log(results.totalValue());
/// console.log(results.getMetric("dv01"));
/// ```
#[wasm_bindgen]
pub struct JsPortfolioResult {
    inner: PortfolioResult,
}

#[wasm_bindgen]
impl JsPortfolioResult {
    /// Create a new portfolio results instance.
    ///
    /// # Arguments
    ///
    /// * `valuation` - Portfolio valuation component
    /// * `metrics` - Portfolio metrics component
    /// * `meta` - Metadata describing calculation context
    ///
    /// # Returns
    ///
    /// New PortfolioResult instance
    #[wasm_bindgen(constructor)]
    pub fn new(
        valuation: JsValue,
        metrics: JsValue,
        meta: JsValue,
    ) -> Result<JsPortfolioResult, JsValue> {
        let valuation_inner: finstack_portfolio::valuation::PortfolioValuation =
            serde_wasm_bindgen::from_value(valuation)
                .map_err(|e| JsValue::from_str(&format!("Failed to parse valuation: {}", e)))?;

        let metrics_inner: finstack_portfolio::metrics::PortfolioMetrics =
            serde_wasm_bindgen::from_value(metrics)
                .map_err(|e| JsValue::from_str(&format!("Failed to parse metrics: {}", e)))?;

        let meta_inner = serde_wasm_bindgen::from_value(meta)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse metadata: {}", e)))?;

        Ok(JsPortfolioResult {
            inner: PortfolioResult::new(valuation_inner, metrics_inner, meta_inner),
        })
    }

    /// Get the portfolio valuation results.
    ///
    /// # Returns
    ///
    /// PortfolioValuation component
    #[wasm_bindgen(getter)]
    pub fn valuation(&self) -> JsPortfolioValuation {
        JsPortfolioValuation::from_inner(self.inner.valuation.clone())
    }

    /// Get the aggregated metrics.
    ///
    /// # Returns
    ///
    /// PortfolioMetrics component
    #[wasm_bindgen(getter)]
    pub fn metrics(&self) -> JsPortfolioMetrics {
        JsPortfolioMetrics::from_inner(self.inner.metrics.clone())
    }

    /// Get metadata about the calculation.
    ///
    /// # Returns
    ///
    /// JavaScript object with metadata
    #[wasm_bindgen(getter)]
    pub fn meta(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner.meta)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize metadata: {}", e)))
    }

    /// Get the total portfolio value.
    ///
    /// # Returns
    ///
    /// Total portfolio value in base currency
    #[wasm_bindgen(js_name = totalValue)]
    pub fn total_value(&self) -> JsMoney {
        JsMoney::from_inner(*self.inner.total_value())
    }

    /// Get a specific aggregated metric.
    ///
    /// # Arguments
    ///
    /// * `metric_id` - Identifier of the metric to retrieve
    ///
    /// # Returns
    ///
    /// Metric value if found, undefined otherwise
    #[wasm_bindgen(js_name = getMetric)]
    pub fn get_metric(&self, metric_id: &str) -> JsValue {
        self.inner
            .get_metric(metric_id)
            .map(JsValue::from_f64)
            .unwrap_or(JsValue::UNDEFINED)
    }

    /// Create from JSON representation.
    ///
    /// # Arguments
    ///
    /// * `value` - JavaScript object
    ///
    /// # Returns
    ///
    /// PortfolioResult instance
    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> Result<JsPortfolioResult, JsValue> {
        serde_wasm_bindgen::from_value(value)
            .map(|inner| JsPortfolioResult { inner })
            .map_err(|e| {
                JsValue::from_str(&format!("Failed to deserialize PortfolioResult: {}", e))
            })
    }

    /// Convert to JSON representation.
    ///
    /// # Returns
    ///
    /// JavaScript object
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize PortfolioResult: {}", e)))
    }
}
