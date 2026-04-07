use crate::utils::json::to_js_value;
use crate::valuations::instruments::InstrumentWrapper;
use finstack_valuations::instruments::equity::cliquet_option::CliquetOption;
use finstack_valuations::pricer::InstrumentType;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = CliquetOptionBuilder)]
#[derive(Clone, Debug, Default)]
pub struct JsCliquetOptionBuilder {
    json_str: Option<String>,
}

#[wasm_bindgen(js_class = CliquetOptionBuilder)]
impl JsCliquetOptionBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new() -> JsCliquetOptionBuilder {
        JsCliquetOptionBuilder { json_str: None }
    }

    #[wasm_bindgen(js_name = jsonString)]
    pub fn json_string(mut self, json_str: String) -> JsCliquetOptionBuilder {
        self.json_str = Some(json_str);
        self
    }

    #[wasm_bindgen(js_name = build)]
    pub fn build(self) -> Result<JsCliquetOption, JsValue> {
        let json_str = self
            .json_str
            .as_deref()
            .ok_or_else(|| JsValue::from_str("CliquetOptionBuilder: jsonString is required"))?;
        use crate::core::error::js_error;
        serde_json::from_str(json_str)
            .map(JsCliquetOption::from_inner)
            .map_err(|e| js_error(e.to_string()))
    }
}

/// Cliquet option (JSON-serializable).
///
/// This instrument is configured via a JSON payload (matching the Rust model schema).
/// Use the builder to construct it and `toJsonString()` to inspect the canonical representation.
#[wasm_bindgen(js_name = CliquetOption)]
#[derive(Clone, Debug)]
pub struct JsCliquetOption {
    pub(crate) inner: CliquetOption,
}

impl InstrumentWrapper for JsCliquetOption {
    type Inner = CliquetOption;
    fn from_inner(inner: CliquetOption) -> Self {
        JsCliquetOption { inner }
    }
    fn inner(&self) -> CliquetOption {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = CliquetOption)]
impl JsCliquetOption {
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
    pub fn instrument_type(&self) -> String {
        InstrumentType::CliquetOption.to_string()
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!("CliquetOption(id='{}')", self.inner.id.as_str())
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsCliquetOption {
        JsCliquetOption::from_inner(self.inner.clone())
    }
}
