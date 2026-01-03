use crate::utils::json::to_js_value;
use crate::valuations::instruments::InstrumentWrapper;
use finstack_valuations::instruments::fx::quanto_option::QuantoOption;
use finstack_valuations::pricer::InstrumentType;
use wasm_bindgen::prelude::*;

/// Quanto option (foreign underlying, domestic payout) (JSON-serializable).
///
/// This instrument is configured via a JSON payload (matching the Rust model schema).
/// Use `fromJson()` to construct it and `toJsonString()` to inspect the canonical representation.
#[wasm_bindgen(js_name = QuantoOption)]
#[derive(Clone, Debug)]
pub struct JsQuantoOption {
    pub(crate) inner: QuantoOption,
}

impl InstrumentWrapper for JsQuantoOption {
    type Inner = QuantoOption;
    fn from_inner(inner: QuantoOption) -> Self {
        JsQuantoOption { inner }
    }
    fn inner(&self) -> QuantoOption {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = QuantoOption)]
impl JsQuantoOption {
    /// Parse a quanto option from a JSON string.
    ///
    /// @param json_str - JSON payload matching the quanto option schema
    /// @returns A new `QuantoOption`
    /// @throws {Error} If the JSON cannot be parsed or is invalid
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(json_str: &str) -> Result<JsQuantoOption, JsValue> {
        use crate::core::error::js_error;
        serde_json::from_str(json_str)
            .map(JsQuantoOption::from_inner)
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
        InstrumentType::QuantoOption as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!("QuantoOption(id='{}')", self.inner.id.as_str())
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsQuantoOption {
        JsQuantoOption::from_inner(self.inner.clone())
    }
}
