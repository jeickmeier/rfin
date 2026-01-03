use crate::core::common::parse::parse_rounding_mode;
use crate::core::currency::JsCurrency;
use finstack_core::config::{FinstackConfig, RoundingMode};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

#[wasm_bindgen(js_name = RoundingMode)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JsRoundingMode {
    Bankers,
    AwayFromZero,
    TowardZero,
    Floor,
    Ceil,
}

impl From<JsRoundingMode> for RoundingMode {
    fn from(value: JsRoundingMode) -> Self {
        match value {
            JsRoundingMode::Bankers => RoundingMode::Bankers,
            JsRoundingMode::AwayFromZero => RoundingMode::AwayFromZero,
            JsRoundingMode::TowardZero => RoundingMode::TowardZero,
            JsRoundingMode::Floor => RoundingMode::Floor,
            JsRoundingMode::Ceil => RoundingMode::Ceil,
        }
    }
}

impl From<RoundingMode> for JsRoundingMode {
    fn from(value: RoundingMode) -> Self {
        match value {
            RoundingMode::Bankers => JsRoundingMode::Bankers,
            RoundingMode::AwayFromZero => JsRoundingMode::AwayFromZero,
            RoundingMode::TowardZero => JsRoundingMode::TowardZero,
            RoundingMode::Floor => JsRoundingMode::Floor,
            RoundingMode::Ceil => JsRoundingMode::Ceil,
            _ => JsRoundingMode::Bankers,
        }
    }
}

#[wasm_bindgen(js_name = FinstackConfig)]
#[derive(Clone, Debug)]
pub struct JsFinstackConfig {
    inner: FinstackConfig,
}

impl Default for JsFinstackConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen(js_class = FinstackConfig)]
impl JsFinstackConfig {
    #[wasm_bindgen(constructor)]
    pub fn new() -> JsFinstackConfig {
        JsFinstackConfig {
            inner: FinstackConfig::default(),
        }
    }

    #[wasm_bindgen(js_name = copy)]
    pub fn copy(&self) -> JsFinstackConfig {
        JsFinstackConfig {
            inner: self.inner.clone(),
        }
    }

    #[wasm_bindgen(getter, js_name = roundingMode)]
    pub fn rounding_mode(&self) -> JsRoundingMode {
        self.inner.rounding.mode.into()
    }

    #[wasm_bindgen(js_name = setRoundingMode)]
    pub fn set_rounding_mode(&mut self, mode: JsRoundingMode) {
        self.inner.rounding.mode = mode.into();
    }

    #[wasm_bindgen(js_name = setRoundingModeLabel)]
    pub fn set_rounding_mode_label(&mut self, label: &str) -> Result<(), JsValue> {
        let mode = parse_rounding_mode(label)?;
        self.inner.rounding.mode = mode;
        Ok(())
    }

    #[wasm_bindgen(js_name = ingestScale)]
    pub fn ingest_scale(&self, currency: &JsCurrency) -> u32 {
        self.inner.ingest_scale(currency.inner())
    }

    #[wasm_bindgen(js_name = setIngestScale)]
    pub fn set_ingest_scale(&mut self, currency: &JsCurrency, decimals: u32) {
        self.inner
            .rounding
            .ingest_scale
            .overrides
            .insert(currency.inner(), decimals);
    }

    #[wasm_bindgen(js_name = outputScale)]
    pub fn output_scale(&self, currency: &JsCurrency) -> u32 {
        self.inner.output_scale(currency.inner())
    }

    #[wasm_bindgen(js_name = setOutputScale)]
    pub fn set_output_scale(&mut self, currency: &JsCurrency, decimals: u32) {
        self.inner
            .rounding
            .output_scale
            .overrides
            .insert(currency.inner(), decimals);
    }

    /// Set an extension section in the configuration.
    ///
    /// @param {string} key - Extension key (e.g., "valuations.calibration.v2")
    /// @param {any} value - Extension configuration value (must be JSON-serializable)
    #[wasm_bindgen(js_name = setExtension)]
    pub fn set_extension(&mut self, key: &str, value: JsValue) -> Result<(), JsValue> {
        use serde_json::Value as JsonValue;
        let json_value: JsonValue = serde_wasm_bindgen::from_value(value)
            .map_err(|e| JsValue::from_str(&format!("Failed to convert value to JSON: {}", e)))?;
        self.inner.extensions.insert(key.to_string(), json_value);
        Ok(())
    }

    /// Serialize the configuration to a JavaScript object.
    ///
    /// Converts the entire configuration (rounding policies, tolerances, extensions)
    /// to a JavaScript object that can be directly manipulated or used for roundtripping.
    ///
    /// # Returns
    /// JavaScript object representation of the configuration.
    ///
    /// # Example
    /// ```javascript
    /// const config = new FinstackConfig();
    /// config.setRoundingMode(RoundingMode.Bankers);
    /// const obj = config.toJson();
    /// // Later: const restored = FinstackConfig.fromState(obj);
    /// ```
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize config: {}", e)))
    }

    /// Serialize the configuration to a JSON string.
    ///
    /// Converts the entire configuration (rounding policies, tolerances, extensions)
    /// to a JSON representation for storage or transmission.
    ///
    /// # Returns
    /// JSON string representation of the configuration.
    ///
    /// # Example
    /// ```javascript
    /// const config = new FinstackConfig();
    /// config.setRoundingMode(RoundingMode.Bankers);
    /// const json = config.toJsonString();
    /// // Later: const restored = FinstackConfig.fromJson(json);
    /// ```
    #[wasm_bindgen(js_name = toJsonString)]
    pub fn to_json_string(&self) -> Result<String, JsValue> {
        serde_json::to_string_pretty(&self.inner)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize config: {}", e)))
    }

    /// Build a `FinstackConfig` from a JSON string.
    ///
    /// Deserializes a JSON string representation (created by `toJsonString()`) back into
    /// a fully functional configuration object.
    ///
    /// # Arguments
    /// * `json_str` - JSON string containing serialized configuration
    ///
    /// # Returns
    /// A new `FinstackConfig` instance with all settings restored.
    ///
    /// # Errors
    /// Returns an error if the JSON is invalid or cannot be deserialized.
    ///
    /// # Example
    /// ```javascript
    /// const json = '{"rounding": {"mode": "bankers"}, ...}';
    /// const config = FinstackConfig.fromJson(json);
    /// ```
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(json_str: &str) -> Result<JsFinstackConfig, JsValue> {
        let inner: FinstackConfig = serde_json::from_str(json_str)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse config JSON: {}", e)))?;
        Ok(JsFinstackConfig { inner })
    }

    /// Build a `FinstackConfig` from a JavaScript object.
    ///
    /// Accepts a structured JavaScript object and converts it to a configuration.
    ///
    /// # Arguments
    /// * `value` - JavaScript object containing configuration data
    ///
    /// # Returns
    /// A new `FinstackConfig` instance.
    #[wasm_bindgen(js_name = fromState)]
    pub fn from_state(value: JsValue) -> Result<JsFinstackConfig, JsValue> {
        let inner: FinstackConfig = serde_wasm_bindgen::from_value(value)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse config state: {}", e)))?;
        Ok(JsFinstackConfig { inner })
    }
}

impl JsFinstackConfig {
    pub(crate) fn inner(&self) -> &FinstackConfig {
        &self.inner
    }
}
