use crate::utils::json::to_js_value;
use crate::valuations::instruments::InstrumentWrapper;
use finstack_valuations::instruments::exotics::lookback_option::{LookbackOption, LookbackType};
use finstack_valuations::pricer::InstrumentType;
use wasm_bindgen::prelude::*;

/// Lookback type for lookback options.
#[wasm_bindgen(js_name = LookbackType)]
#[derive(Clone, Copy, Debug)]
pub enum JsLookbackType {
    FixedStrike,
    FloatingStrike,
}

impl From<LookbackType> for JsLookbackType {
    fn from(lookback_type: LookbackType) -> Self {
        match lookback_type {
            LookbackType::FixedStrike => JsLookbackType::FixedStrike,
            LookbackType::FloatingStrike => JsLookbackType::FloatingStrike,
        }
    }
}

impl From<JsLookbackType> for LookbackType {
    fn from(lookback_type: JsLookbackType) -> Self {
        match lookback_type {
            JsLookbackType::FixedStrike => LookbackType::FixedStrike,
            JsLookbackType::FloatingStrike => LookbackType::FloatingStrike,
        }
    }
}

/// Lookback option (path-dependent option) (JSON-serializable).
///
/// This instrument is configured via a JSON payload (matching the Rust model schema).
/// Use `fromJson()` to construct it and `toJsonString()` to inspect the canonical representation.
#[wasm_bindgen(js_name = LookbackOption)]
#[derive(Clone, Debug)]
pub struct JsLookbackOption {
    pub(crate) inner: LookbackOption,
}

impl InstrumentWrapper for JsLookbackOption {
    type Inner = LookbackOption;
    fn from_inner(inner: LookbackOption) -> Self {
        JsLookbackOption { inner }
    }
    fn inner(&self) -> LookbackOption {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = LookbackOption)]
impl JsLookbackOption {
    /// Parse a lookback option from a JSON string.
    ///
    /// @param json_str - JSON payload matching the lookback option schema
    /// @returns A new `LookbackOption`
    /// @throws {Error} If the JSON cannot be parsed or is invalid
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(json_str: &str) -> Result<JsLookbackOption, JsValue> {
        use crate::core::error::js_error;
        serde_json::from_str(json_str)
            .map(JsLookbackOption::from_inner)
            .map_err(|e| js_error(e.to_string()))
    }

    #[wasm_bindgen(getter, js_name = instrumentId)]
    pub fn instrument_id(&self) -> String {
        self.inner.id.as_str().to_string()
    }

    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }

    /// Serialize this instrument to a pretty-printed JSON string.
    ///
    /// @returns JSON string
    /// @throws {Error} If serialization fails
    #[wasm_bindgen(js_name = toJsonString)]
    pub fn to_json_string(&self) -> Result<String, JsValue> {
        use crate::core::error::js_error;
        serde_json::to_string_pretty(&self.inner).map_err(|e| js_error(e.to_string()))
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::LookbackOption as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!("LookbackOption(id='{}')", self.inner.id.as_str())
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsLookbackOption {
        JsLookbackOption::from_inner(self.inner.clone())
    }
}
