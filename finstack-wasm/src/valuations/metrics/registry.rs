//! Metric registry bindings for WASM.
//!
//! Provides the registry system for querying available financial metrics.
//! **Note**: Actual metric computation is done through the ValuationResult
//! object after pricing an instrument.

use super::ids::JsMetricId;
use finstack_valuations::metrics::{standard_registry, MetricId, MetricRegistry};
use js_sys::Array;
use wasm_bindgen::prelude::*;

/// Registry of metric calculators.
///
/// Manages metric definitions and provides information about available metrics
/// for different instrument types. Actual computation is done through
/// ValuationResult after pricing.
///
/// @example
/// ```typescript
/// // Create standard registry with all built-in metrics
/// const registry = MetricRegistry.standard();
///
/// // Check if metric is available
/// if (registry.hasMetric("dv01")) {
///   console.log("DV01 is available");
/// }
///
/// // List all available metrics
/// const metrics = registry.availableMetrics();
/// console.log(`${metrics.length} metrics available`);
/// ```
#[wasm_bindgen(js_name = MetricRegistry)]
pub struct JsMetricRegistry {
    inner: MetricRegistry,
}

#[wasm_bindgen(js_class = MetricRegistry)]
impl JsMetricRegistry {
    /// Create a new empty metric registry.
    ///
    /// @returns {MetricRegistry} Empty registry with no metrics registered
    #[wasm_bindgen(constructor)]
    pub fn new() -> JsMetricRegistry {
        JsMetricRegistry {
            inner: MetricRegistry::new(),
        }
    }

    /// Create a standard registry with all built-in metrics.
    ///
    /// Includes metrics for bonds, swaps, deposits, options, credit, and risk.
    ///
    /// @returns {MetricRegistry} Registry with all standard metrics
    ///
    /// @example
    /// ```typescript
    /// const registry = MetricRegistry.standard();
    /// console.log(registry.hasMetric("pv")); // true
    /// console.log(registry.hasMetric("dv01")); // true
    /// ```
    #[wasm_bindgen(js_name = standard)]
    pub fn standard() -> JsMetricRegistry {
        JsMetricRegistry {
            inner: standard_registry().clone(),
        }
    }

    /// Check if a metric is registered.
    ///
    /// @param {MetricId | string} metricId - Metric to check
    /// @returns {boolean} True if metric is registered
    ///
    /// @example
    /// ```typescript
    /// if (registry.hasMetric("dv01")) {
    ///   // DV01 is available
    /// }
    /// ```
    #[wasm_bindgen(js_name = hasMetric)]
    pub fn has_metric(&self, metric_id: JsValue) -> Result<bool, JsValue> {
        let id = parse_metric_id(metric_id)?;
        Ok(self.inner.has_metric(id))
    }

    /// List all registered metrics.
    ///
    /// @returns {Array<MetricId>} Array of registered metric IDs
    ///
    /// @example
    /// ```typescript
    /// const metrics = registry.availableMetrics();
    /// console.log(`Available metrics: ${metrics.length}`);
    /// for (const metric of metrics) {
    ///   console.log(metric.name);
    /// }
    /// ```
    #[wasm_bindgen(js_name = availableMetrics)]
    pub fn available_metrics(&self) -> Array {
        let result = Array::new();
        for id in self.inner.available_metrics() {
            result.push(&JsMetricId::from_inner(id.clone()).into());
        }
        result
    }
}

impl Default for JsMetricRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Parse MetricId from JsValue (currently only supports string names).
fn parse_metric_id(value: JsValue) -> Result<MetricId, JsValue> {
    // Extract string name
    if let Some(name) = value.as_string() {
        return Ok(name.parse().unwrap_or_else(|_| MetricId::custom(name)));
    }

    Err(JsValue::from_str(
        "Expected string metric name (e.g., 'pv', 'dv01', 'duration_mod')",
    ))
}

#[cfg(all(test, target_arch = "wasm32"))]
mod tests {
    use super::*;
    use wasm_bindgen_test::wasm_bindgen_test;

    #[wasm_bindgen_test]
    fn parse_metric_id_falls_back_to_custom_metric() {
        let parsed = parse_metric_id(JsValue::from_str("my_custom_metric"))
            .expect("string metric names should parse");
        assert_eq!(parsed.as_str(), "my_custom_metric");
    }
}
