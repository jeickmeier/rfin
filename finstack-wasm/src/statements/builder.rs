//! Builder for financial models.

use crate::statements::types::{JsFinancialModelSpec, JsForecastSpec};
use finstack_core::dates::PeriodId;
use finstack_statements::builder::{ModelBuilder, NeedPeriods, Ready};
use finstack_statements::types::AmountOrScalar;
use std::str::FromStr;
use wasm_bindgen::prelude::*;

/// Builder for financial statement models.
///
/// Provides a fluent API for constructing financial models with periods,
/// nodes (values, formulas, forecasts), and metadata.
///
/// # Example
/// ```javascript
/// const builder = new JsModelBuilder("Acme Corp");
/// builder.periods("2025Q1..Q4", "2025Q2");
/// builder.value("revenue", {"2025Q1": JsAmountOrScalar.scalar(1000000)});
/// builder.compute("cogs", "revenue * 0.6");
/// const model = builder.build();
/// ```
#[wasm_bindgen]
pub struct JsModelBuilder {
    state: BuilderState,
}

enum BuilderState {
    NeedPeriods(ModelBuilder<NeedPeriods>),
    Ready(ModelBuilder<Ready>),
}

#[wasm_bindgen]
impl JsModelBuilder {
    /// Create a new model builder.
    ///
    /// # Arguments
    /// * `id` - Unique model identifier
    ///
    /// # Returns
    /// Model builder instance (you must call `periods()` before adding nodes)
    #[wasm_bindgen(constructor)]
    pub fn new(id: String) -> JsModelBuilder {
        JsModelBuilder {
            state: BuilderState::NeedPeriods(ModelBuilder::new(id)),
        }
    }

    /// Define periods using a range expression.
    ///
    /// # Arguments
    /// * `range` - Period range (e.g., "2025Q1..Q4", "2025Q1..2026Q2")
    /// * `actuals_until` - Optional cutoff for actuals (e.g., "2025Q2")
    ///
    /// # Returns
    /// Builder instance ready for node definitions
    #[wasm_bindgen]
    pub fn periods(
        mut self,
        range: &str,
        actuals_until: Option<String>,
    ) -> Result<JsModelBuilder, JsValue> {
        match self.state {
            BuilderState::NeedPeriods(builder) => {
                let ready = builder
                    .periods(range, actuals_until.as_deref())
                    .map_err(|e| JsValue::from_str(&format!("Failed to set periods: {}", e)))?;
                self.state = BuilderState::Ready(ready);
                Ok(self)
            }
            BuilderState::Ready(_) => Err(JsValue::from_str("periods() already called")),
        }
    }

    // Note: periods_explicit is not exposed in WASM as Period is not a WASM-compatible type.
    // Users should use the periods() method with string ranges instead.

    /// Add a value node with explicit period values.
    ///
    /// Value nodes contain only explicit data (actuals or assumptions).
    ///
    /// # Arguments
    /// * `node_id` - Node identifier
    /// * `values` - JavaScript object mapping period IDs to values
    ///
    /// # Returns
    /// Builder instance for chaining
    ///
    /// # Example
    /// ```javascript
    /// builder.value("revenue", {
    ///   "2025Q1": JsAmountOrScalar.scalar(1000000),
    ///   "2025Q2": JsAmountOrScalar.scalar(1100000)
    /// });
    /// ```
    #[wasm_bindgen]
    pub fn value(mut self, node_id: String, values: JsValue) -> Result<JsModelBuilder, JsValue> {
        let values_vec = parse_period_values(values)?;

        match self.state {
            BuilderState::Ready(builder) => {
                self.state = BuilderState::Ready(builder.value(node_id, &values_vec));
                Ok(self)
            }
            BuilderState::NeedPeriods(_) => Err(JsValue::from_str("Must call periods() first")),
        }
    }

    /// Add a calculated node with a formula.
    ///
    /// Calculated nodes derive their values from formulas only.
    ///
    /// # Arguments
    /// * `node_id` - Node identifier
    /// * `formula` - Formula text in statement DSL
    ///
    /// # Returns
    /// Builder instance for chaining
    ///
    /// # Example
    /// ```javascript
    /// builder.compute("gross_profit", "revenue - cogs");
    /// ```
    #[wasm_bindgen]
    pub fn compute(mut self, node_id: String, formula: String) -> Result<JsModelBuilder, JsValue> {
        match self.state {
            BuilderState::Ready(builder) => {
                let new_builder = builder.compute(node_id, formula).map_err(|e| {
                    JsValue::from_str(&format!("Failed to add compute node: {}", e))
                })?;
                self.state = BuilderState::Ready(new_builder);
                Ok(self)
            }
            BuilderState::NeedPeriods(_) => Err(JsValue::from_str("Must call periods() first")),
        }
    }

    /// Add a forecast specification to an existing node.
    ///
    /// This allows forecasting values into future periods using various methods.
    ///
    /// # Arguments
    /// * `node_id` - Node identifier
    /// * `forecast_spec` - Forecast specification
    ///
    /// # Returns
    /// Builder instance for chaining
    ///
    /// # Example
    /// ```javascript
    /// builder.forecast("revenue", JsForecastSpec.growthPct(0.05));
    /// ```
    #[wasm_bindgen]
    pub fn forecast(
        mut self,
        node_id: String,
        forecast_spec: &JsForecastSpec,
    ) -> Result<JsModelBuilder, JsValue> {
        match self.state {
            BuilderState::Ready(builder) => {
                self.state =
                    BuilderState::Ready(builder.forecast(node_id, forecast_spec.inner.clone()));
                Ok(self)
            }
            BuilderState::NeedPeriods(_) => Err(JsValue::from_str("Must call periods() first")),
        }
    }

    /// Add metadata to the model.
    ///
    /// # Arguments
    /// * `key` - Metadata key
    /// * `value` - Metadata value (must be JSON-serializable)
    ///
    /// # Returns
    /// Builder instance for chaining
    #[wasm_bindgen(js_name = withMeta)]
    pub fn with_meta(mut self, key: String, value: JsValue) -> Result<JsModelBuilder, JsValue> {
        let json_value: serde_json::Value = serde_wasm_bindgen::from_value(value)
            .map_err(|e| JsValue::from_str(&format!("Failed to convert metadata value: {}", e)))?;

        match self.state {
            BuilderState::Ready(builder) => {
                self.state = BuilderState::Ready(builder.with_meta(key, json_value));
                Ok(self)
            }
            BuilderState::NeedPeriods(_) => Err(JsValue::from_str("Must call periods() first")),
        }
    }

    /// Build the final model specification.
    ///
    /// # Returns
    /// Complete financial model specification
    #[wasm_bindgen]
    pub fn build(self) -> Result<JsFinancialModelSpec, JsValue> {
        match self.state {
            BuilderState::Ready(builder) => {
                let spec = builder
                    .build()
                    .map_err(|e| JsValue::from_str(&format!("Failed to build model: {}", e)))?;
                Ok(JsFinancialModelSpec::new(spec))
            }
            BuilderState::NeedPeriods(_) => Err(JsValue::from_str("Must call periods() first")),
        }
    }
}

/// Helper to parse period values from JavaScript object.
fn parse_period_values(values: JsValue) -> Result<Vec<(PeriodId, AmountOrScalar)>, JsValue> {
    // Try to convert to JS Object
    if !values.is_object() {
        return Err(JsValue::from_str("values must be an object"));
    }

    let obj = js_sys::Object::from(values);
    let entries = js_sys::Object::entries(&obj);
    let mut result = Vec::new();

    for i in 0..entries.length() {
        let entry = entries.get(i);
        let pair = js_sys::Array::from(&entry);
        let key = pair.get(0);
        let value_js = pair.get(1);

        // Parse period ID from string key
        let period_str = key
            .as_string()
            .ok_or_else(|| JsValue::from_str("Period ID must be a string"))?;

        let period_id = PeriodId::from_str(&period_str).map_err(|e| {
            JsValue::from_str(&format!("Invalid period ID '{}': {}", period_str, e))
        })?;

        // Convert JsValue to AmountOrScalar
        let amount_or_scalar = if value_js.is_object() && !js_sys::Array::is_array(&value_js) {
            // Try to deserialize as AmountOrScalar object
            let aos: AmountOrScalar = serde_wasm_bindgen::from_value(value_js).map_err(|e| {
                JsValue::from_str(&format!(
                    "Failed to parse value for period {}: {}",
                    period_str, e
                ))
            })?;
            aos
        } else if let Some(num) = value_js.as_f64() {
            // Plain number - treat as scalar
            AmountOrScalar::scalar(num)
        } else {
            return Err(JsValue::from_str(&format!(
                "Value for period {} must be a number or AmountOrScalar object",
                period_str
            )));
        };

        result.push((period_id, amount_or_scalar));
    }

    Ok(result)
}
