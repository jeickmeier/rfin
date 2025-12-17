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
}

impl JsFinstackConfig {
    pub(crate) fn inner(&self) -> &FinstackConfig {
        &self.inner
    }
}
