use crate::utils::json::to_js_value;
use crate::valuations::instruments::InstrumentWrapper;
use finstack_valuations::instruments::fx::fx_barrier_option::FxBarrierOption;
use finstack_valuations::pricer::InstrumentType;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = FxBarrierOptionBuilder)]
#[derive(Clone, Debug, Default)]
pub struct JsFxBarrierOptionBuilder {
    json_str: Option<String>,
}

#[wasm_bindgen(js_class = FxBarrierOptionBuilder)]
impl JsFxBarrierOptionBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new() -> JsFxBarrierOptionBuilder {
        JsFxBarrierOptionBuilder { json_str: None }
    }

    #[wasm_bindgen(js_name = jsonString)]
    pub fn json_string(mut self, json_str: String) -> JsFxBarrierOptionBuilder {
        self.json_str = Some(json_str);
        self
    }

    #[wasm_bindgen(js_name = build)]
    pub fn build(self) -> Result<JsFxBarrierOption, JsValue> {
        let json_str = self
            .json_str
            .as_deref()
            .ok_or_else(|| JsValue::from_str("FxBarrierOptionBuilder: jsonString is required"))?;
        JsFxBarrierOption::from_json(json_str)
    }
}

/// FX barrier option (JSON-serializable).
///
/// This instrument is configured via a JSON payload (matching the Rust model schema).
/// Use `fromJson()` to construct it and `toJsonString()` to inspect the canonical representation.
#[wasm_bindgen(js_name = FxBarrierOption)]
#[derive(Clone, Debug)]
pub struct JsFxBarrierOption {
    pub(crate) inner: FxBarrierOption,
}

impl InstrumentWrapper for JsFxBarrierOption {
    type Inner = FxBarrierOption;
    fn from_inner(inner: FxBarrierOption) -> Self {
        JsFxBarrierOption { inner }
    }
    fn inner(&self) -> FxBarrierOption {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = FxBarrierOption)]
impl JsFxBarrierOption {
    /// Parse an FX barrier option from a JSON string.
    ///
    /// @param json_str - JSON payload matching the FX barrier option schema
    /// @returns A new `FxBarrierOption`
    /// @throws {Error} If the JSON cannot be parsed or is invalid
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(json_str: &str) -> Result<JsFxBarrierOption, JsValue> {
        web_sys::console::warn_1(&JsValue::from_str(
            "FxBarrierOption.fromJson is deprecated; use FxBarrierOptionBuilder instead.",
        ));
        use crate::core::error::js_error;
        serde_json::from_str(json_str)
            .map(JsFxBarrierOption::from_inner)
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
        InstrumentType::FxBarrierOption as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!("FxBarrierOption(id='{}')", self.inner.id.as_str())
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsFxBarrierOption {
        JsFxBarrierOption::from_inner(self.inner.clone())
    }
}
