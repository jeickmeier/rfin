//! Dynamic metric registry for statements.

use finstack_statements::registry::{MetricDefinition, MetricRegistry as RustMetricRegistry, Registry, UnitType};
use wasm_bindgen::prelude::*;

/// Metric unit type.
///
/// Defines the unit of measurement for a metric.
#[wasm_bindgen]
#[derive(Clone, Copy, Debug)]
pub struct JsUnitType {
    inner: UnitType,
}

#[wasm_bindgen]
#[allow(non_snake_case)]
impl JsUnitType {
    /// Currency amount.
    #[wasm_bindgen(getter)]
    pub fn CURRENCY() -> JsUnitType {
        JsUnitType {
            inner: UnitType::Currency,
        }
    }

    /// Percentage (0-100 scale).
    #[wasm_bindgen(getter)]
    pub fn PERCENTAGE() -> JsUnitType {
        JsUnitType {
            inner: UnitType::Percentage,
        }
    }

    /// Ratio (decimal scale).
    #[wasm_bindgen(getter)]
    pub fn RATIO() -> JsUnitType {
        JsUnitType {
            inner: UnitType::Ratio,
        }
    }

    /// Count (integer quantity).
    #[wasm_bindgen(getter)]
    pub fn COUNT() -> JsUnitType {
        JsUnitType {
            inner: UnitType::Count,
        }
    }

    /// Time period (days, months, years).
    #[wasm_bindgen(getter)]
    pub fn TIME_PERIOD() -> JsUnitType {
        JsUnitType {
            inner: UnitType::TimePeriod,
        }
    }

    /// Convert to string representation.
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!("{:?}", self.inner)
    }
}

/// Metric definition.
///
/// Defines a reusable financial metric with formula, unit, and metadata.
#[wasm_bindgen]
pub struct JsMetricDefinition {
    inner: MetricDefinition,
}

#[wasm_bindgen]
impl JsMetricDefinition {
    /// Create from JSON representation.
    ///
    /// # Arguments
    /// * `value` - JavaScript object
    ///
    /// # Returns
    /// Metric definition
    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> Result<JsMetricDefinition, JsValue> {
        serde_wasm_bindgen::from_value(value)
            .map(|inner| JsMetricDefinition { inner })
            .map_err(|e| {
                JsValue::from_str(&format!("Failed to deserialize MetricDefinition: {}", e))
            })
    }

    /// Convert to JSON representation.
    ///
    /// # Returns
    /// JavaScript object
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner)
            .map_err(|e| {
                JsValue::from_str(&format!("Failed to serialize MetricDefinition: {}", e))
            })
    }

    /// Get metric identifier.
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.inner.id.clone()
    }

    /// Get metric name.
    #[wasm_bindgen(getter)]
    pub fn name(&self) -> String {
        self.inner.name.clone()
    }

    /// Get metric formula.
    #[wasm_bindgen(getter)]
    pub fn formula(&self) -> String {
        self.inner.formula.clone()
    }

    /// Get metric description.
    #[wasm_bindgen(getter)]
    pub fn description(&self) -> Option<String> {
        self.inner.description.clone()
    }

    /// Convert to string representation.
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!("MetricDefinition(id='{}', name='{}')", self.inner.id, self.inner.name)
    }
}

impl JsMetricDefinition {
    pub(crate) fn new(inner: MetricDefinition) -> Self {
        Self { inner }
    }
}

/// Metric registry.
///
/// Container for a collection of metric definitions.
#[wasm_bindgen]
pub struct JsMetricRegistry {
    inner: RustMetricRegistry,
}

#[wasm_bindgen]
impl JsMetricRegistry {
    /// Create from JSON representation.
    ///
    /// # Arguments
    /// * `value` - JavaScript object
    ///
    /// # Returns
    /// Metric registry
    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> Result<JsMetricRegistry, JsValue> {
        serde_wasm_bindgen::from_value(value)
            .map(|inner| JsMetricRegistry { inner })
            .map_err(|e| {
                JsValue::from_str(&format!("Failed to deserialize MetricRegistry: {}", e))
            })
    }

    /// Convert to JSON representation.
    ///
    /// # Returns
    /// JavaScript object
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner)
            .map_err(|e| {
                JsValue::from_str(&format!("Failed to serialize MetricRegistry: {}", e))
            })
    }

    /// Get registry namespace.
    #[wasm_bindgen(getter)]
    pub fn namespace(&self) -> String {
        self.inner.namespace.clone()
    }

    /// Get schema version.
    #[wasm_bindgen(getter, js_name = schemaVersion)]
    pub fn schema_version(&self) -> u32 {
        self.inner.schema_version
    }

    /// Get number of metrics.
    #[wasm_bindgen(js_name = metricCount)]
    pub fn metric_count(&self) -> usize {
        self.inner.metrics.len()
    }
}

impl JsMetricRegistry {
    pub(crate) fn new(inner: RustMetricRegistry) -> Self {
        Self { inner }
    }
}

/// Dynamic metric registry.
///
/// Allows loading reusable metric definitions from JSON,
/// enabling analysts to define standard financial metrics without recompiling.
///
/// # Example
/// ```javascript
/// const registry = new JsRegistry();
/// registry.loadBuiltins();
/// const metric = registry.get("fin.gross_margin");
/// console.log(metric.formula); // "gross_profit / revenue"
/// ```
#[wasm_bindgen]
pub struct JsRegistry {
    inner: Registry,
}

#[wasm_bindgen]
impl JsRegistry {
    /// Create a new registry.
    #[wasm_bindgen(constructor)]
    pub fn new() -> JsRegistry {
        JsRegistry {
            inner: Registry::new(),
        }
    }

    /// Load built-in metrics (fin.* namespace).
    ///
    /// Loads 22 standard financial metrics including:
    /// - fin.gross_margin
    /// - fin.operating_margin
    /// - fin.net_margin
    /// - And more...
    #[wasm_bindgen(js_name = loadBuiltins)]
    pub fn load_builtins(&mut self) -> Result<(), JsValue> {
        self.inner
            .load_builtins()
            .map_err(|e| JsValue::from_str(&format!("Failed to load built-ins: {}", e)))
    }

    /// Load metrics from a JSON string.
    ///
    /// # Arguments
    /// * `json_str` - JSON string containing metric registry
    ///
    /// # Returns
    /// Loaded registry
    #[wasm_bindgen(js_name = loadFromJsonStr)]
    pub fn load_from_json_str(&mut self, json_str: &str) -> Result<JsMetricRegistry, JsValue> {
        let registry = self
            .inner
            .load_from_json_str(json_str)
            .map_err(|e| JsValue::from_str(&format!("Failed to load from JSON: {}", e)))?;
        Ok(JsMetricRegistry::new(registry))
    }

    /// Get a metric definition by ID.
    ///
    /// # Arguments
    /// * `metric_id` - Metric identifier (e.g., "fin.gross_margin")
    ///
    /// # Returns
    /// Metric definition
    #[wasm_bindgen]
    pub fn get(&self, metric_id: &str) -> Result<JsMetricDefinition, JsValue> {
        self.inner
            .get(metric_id)
            .map(|stored| JsMetricDefinition::new(stored.definition.clone()))
            .map_err(|e| JsValue::from_str(&format!("Failed to get metric: {}", e)))
    }

    /// List available metrics.
    ///
    /// # Arguments
    /// * `namespace` - Optional filter by namespace (e.g., "fin")
    ///
    /// # Returns
    /// Array of metric IDs
    #[wasm_bindgen(js_name = listMetrics)]
    pub fn list_metrics(&self, namespace: Option<String>) -> Vec<String> {
        if let Some(ns) = namespace {
            self.inner
                .namespace(&ns)
                .map(|(id, _)| id.to_string())
                .collect()
        } else {
            self.inner
                .all_metrics()
                .map(|(id, _)| id.to_string())
                .collect()
        }
    }

    /// Check if a metric exists.
    ///
    /// # Arguments
    /// * `metric_id` - Metric identifier
    ///
    /// # Returns
    /// True if metric exists
    #[wasm_bindgen(js_name = hasMetric)]
    pub fn has_metric(&self, metric_id: &str) -> bool {
        self.inner.has(metric_id)
    }

    /// Get number of metrics in registry.
    #[wasm_bindgen(js_name = metricCount)]
    pub fn metric_count(&self) -> usize {
        self.inner.len()
    }

    /// Convert to string representation.
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!("Registry(metrics={})", self.inner.len())
    }
}

