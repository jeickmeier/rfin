//! Portfolio metrics aggregation for WASM.

use crate::portfolio::valuation::JsPortfolioValuation;
use finstack_portfolio::metrics::{AggregatedMetric, PortfolioMetrics};
use js_sys::Object;
use wasm_bindgen::prelude::*;

/// Aggregated metric across the portfolio.
///
/// Contains portfolio-wide totals as well as breakdowns by entity.
///
/// # Examples
///
/// ```javascript
/// const metric = metrics.getMetric("dv01");
/// console.log(metric.total);
/// console.log(metric.byEntity["ENTITY_A"]);
/// ```
#[wasm_bindgen]
pub struct JsAggregatedMetric {
    inner: AggregatedMetric,
}

#[wasm_bindgen]
impl JsAggregatedMetric {
    /// Get the metric identifier.
    ///
    /// # Returns
    ///
    /// Metric ID as string
    #[wasm_bindgen(getter, js_name = metricId)]
    pub fn metric_id(&self) -> String {
        self.inner.metric_id.clone()
    }

    /// Get the total value across all positions (for summable metrics).
    ///
    /// # Returns
    ///
    /// Total metric value
    #[wasm_bindgen(getter)]
    pub fn total(&self) -> f64 {
        self.inner.total
    }

    /// Get aggregated values by entity as a JavaScript object.
    ///
    /// # Returns
    ///
    /// JavaScript object mapping entity IDs to metric values
    #[wasm_bindgen(getter, js_name = byEntity)]
    pub fn by_entity(&self) -> Result<JsValue, JsValue> {
        let obj = Object::new();
        for (id, value) in &self.inner.by_entity {
            js_sys::Reflect::set(&obj, &JsValue::from_str(id), &JsValue::from_f64(*value))?;
        }
        Ok(JsValue::from(obj))
    }

    /// Create from JSON representation.
    ///
    /// # Arguments
    ///
    /// * `value` - JavaScript object
    ///
    /// # Returns
    ///
    /// AggregatedMetric instance
    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> Result<JsAggregatedMetric, JsValue> {
        serde_wasm_bindgen::from_value(value)
            .map(|inner| JsAggregatedMetric { inner })
            .map_err(|e| {
                JsValue::from_str(&format!("Failed to deserialize AggregatedMetric: {}", e))
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
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize AggregatedMetric: {}", e)))
    }
}

impl JsAggregatedMetric {
    pub(crate) fn from_inner(inner: AggregatedMetric) -> Self {
        Self { inner }
    }
}

/// Complete portfolio metrics results.
///
/// Holds both aggregated metrics and per-position values.
///
/// # Examples
///
/// ```javascript
/// const metrics = aggregateMetrics(valuation);
/// const dv01 = metrics.getMetric("dv01");
/// const posMetrics = metrics.getPositionMetrics("POS_1");
/// ```
#[wasm_bindgen]
pub struct JsPortfolioMetrics {
    inner: PortfolioMetrics,
}

#[wasm_bindgen]
impl JsPortfolioMetrics {
    /// Get aggregated metrics (summable only) as a JavaScript object.
    ///
    /// # Returns
    ///
    /// JavaScript object mapping metric IDs to AggregatedMetric instances
    #[wasm_bindgen(getter)]
    pub fn aggregated(&self) -> Result<JsValue, JsValue> {
        let obj = Object::new();
        for (id, metric) in &self.inner.aggregated {
            let js_metric = JsAggregatedMetric::from_inner(metric.clone());
            js_sys::Reflect::set(&obj, &JsValue::from_str(id), &JsValue::from(js_metric))?;
        }
        Ok(JsValue::from(obj))
    }

    /// Get raw metrics by position (all metrics) as a JavaScript object.
    ///
    /// # Returns
    ///
    /// JavaScript object mapping position IDs to metric objects
    #[wasm_bindgen(getter, js_name = byPosition)]
    pub fn by_position(&self) -> Result<JsValue, JsValue> {
        let obj = Object::new();
        for (position_id, metrics) in &self.inner.by_position {
            let metrics_obj = Object::new();
            for (metric_id, value) in metrics {
                js_sys::Reflect::set(
                    &metrics_obj,
                    &JsValue::from_str(metric_id),
                    &JsValue::from_f64(*value),
                )?;
            }
            js_sys::Reflect::set(&obj, &JsValue::from_str(position_id), &metrics_obj)?;
        }
        Ok(JsValue::from(obj))
    }

    /// Get an aggregated metric by identifier.
    ///
    /// # Arguments
    ///
    /// * `metric_id` - Identifier of the metric to look up
    ///
    /// # Returns
    ///
    /// The metric if found, undefined otherwise
    #[wasm_bindgen(js_name = getMetric)]
    pub fn get_metric(&self, metric_id: &str) -> Option<JsAggregatedMetric> {
        self.inner
            .get_metric(metric_id)
            .map(|m| JsAggregatedMetric::from_inner(m.clone()))
    }

    /// Get metrics for a specific position.
    ///
    /// # Arguments
    ///
    /// * `position_id` - Identifier of the position to query
    ///
    /// # Returns
    ///
    /// JavaScript object with metric key-value pairs, or undefined if not found
    #[wasm_bindgen(js_name = getPositionMetrics)]
    pub fn get_position_metrics(&self, position_id: &str) -> Result<JsValue, JsValue> {
        if let Some(metrics) = self.inner.get_position_metrics(position_id) {
            let obj = Object::new();
            for (metric_id, value) in metrics {
                js_sys::Reflect::set(
                    &obj,
                    &JsValue::from_str(metric_id),
                    &JsValue::from_f64(*value),
                )?;
            }
            Ok(JsValue::from(obj))
        } else {
            Ok(JsValue::UNDEFINED)
        }
    }

    /// Get the total value of a specific metric across the portfolio.
    ///
    /// # Arguments
    ///
    /// * `metric_id` - Identifier of the metric
    ///
    /// # Returns
    ///
    /// Total metric value if found, undefined otherwise
    #[wasm_bindgen(js_name = getTotal)]
    pub fn get_total(&self, metric_id: &str) -> JsValue {
        self.inner
            .get_total(metric_id)
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
    /// PortfolioMetrics instance
    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> Result<JsPortfolioMetrics, JsValue> {
        serde_wasm_bindgen::from_value(value)
            .map(|inner| JsPortfolioMetrics { inner })
            .map_err(|e| {
                JsValue::from_str(&format!("Failed to deserialize PortfolioMetrics: {}", e))
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
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize PortfolioMetrics: {}", e)))
    }
}

impl JsPortfolioMetrics {
    pub(crate) fn from_inner(inner: PortfolioMetrics) -> Self {
        Self { inner }
    }
}

/// Aggregate metrics from portfolio valuation.
///
/// Computes portfolio-wide metrics by summing position-level results where appropriate.
/// Only summable metrics (DV01, CS01, Theta, etc.) are aggregated.
///
/// # Arguments
///
/// * `valuation` - Portfolio valuation results
///
/// # Returns
///
/// Aggregated metrics results
///
/// # Throws
///
/// Error if aggregation fails
///
/// # Examples
///
/// ```javascript
/// const metrics = aggregateMetrics(valuation);
/// console.log(metrics.getTotal("dv01"));
/// ```
#[wasm_bindgen(js_name = aggregateMetrics)]
pub fn js_aggregate_metrics(
    valuation: &JsPortfolioValuation,
) -> Result<JsPortfolioMetrics, JsValue> {
    finstack_portfolio::aggregate_metrics(&valuation.inner)
        .map(JsPortfolioMetrics::from_inner)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}
