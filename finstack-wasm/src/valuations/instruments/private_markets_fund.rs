use crate::core::currency::JsCurrency;
use crate::core::error::js_error;
use crate::utils::json::to_js_value;
use crate::valuations::instruments::InstrumentWrapper;
use finstack_valuations::instruments::equity::pe_fund::PrivateMarketsFund;
use finstack_valuations::pricer::InstrumentType;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = PrivateMarketsFundBuilder)]
#[derive(Clone, Debug, Default)]
pub struct JsPrivateMarketsFundBuilder {
    json_str: Option<String>,
}

#[wasm_bindgen(js_class = PrivateMarketsFundBuilder)]
impl JsPrivateMarketsFundBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new() -> JsPrivateMarketsFundBuilder {
        JsPrivateMarketsFundBuilder { json_str: None }
    }

    #[wasm_bindgen(js_name = jsonString)]
    pub fn json_string(mut self, json_str: String) -> JsPrivateMarketsFundBuilder {
        self.json_str = Some(json_str);
        self
    }

    #[wasm_bindgen(js_name = build)]
    pub fn build(self) -> Result<JsPrivateMarketsFund, JsValue> {
        let json_str = self.json_str.as_deref().ok_or_else(|| {
            JsValue::from_str("PrivateMarketsFundBuilder: jsonString is required")
        })?;
        JsPrivateMarketsFund::from_json(json_str)
    }
}

/// Private markets fund with event schedule / waterfalls (JSON-serializable).
///
/// This instrument is configured via a JSON payload (matching the Rust model schema).
/// Use `fromJson()` to construct it and `toJsonString()` to inspect the canonical representation.
#[wasm_bindgen(js_name = PrivateMarketsFund)]
#[derive(Clone, Debug)]
pub struct JsPrivateMarketsFund {
    pub(crate) inner: PrivateMarketsFund,
}

impl InstrumentWrapper for JsPrivateMarketsFund {
    type Inner = PrivateMarketsFund;
    fn from_inner(inner: PrivateMarketsFund) -> Self {
        JsPrivateMarketsFund { inner }
    }
    fn inner(&self) -> PrivateMarketsFund {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = PrivateMarketsFund)]
impl JsPrivateMarketsFund {
    /// Parse a private markets fund from a JSON string.
    ///
    /// @param json_str - JSON payload matching the fund schema
    /// @returns A new `PrivateMarketsFund`
    /// @throws {Error} If the JSON cannot be parsed or is invalid
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(json_str: &str) -> Result<JsPrivateMarketsFund, JsValue> {
        web_sys::console::warn_1(&JsValue::from_str(
            "PrivateMarketsFund.fromJson is deprecated; use PrivateMarketsFundBuilder instead.",
        ));
        serde_json::from_str(json_str)
            .map(JsPrivateMarketsFund::from_inner)
            .map_err(|e| js_error(e.to_string()))
    }

    #[wasm_bindgen(getter, js_name = instrumentId)]
    pub fn instrument_id(&self) -> String {
        self.inner.id.as_str().to_string()
    }

    #[wasm_bindgen(getter)]
    pub fn currency(&self) -> JsCurrency {
        JsCurrency::from_inner(self.inner.currency)
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
        serde_json::to_string_pretty(&self.inner).map_err(|e| js_error(e.to_string()))
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> String {
        InstrumentType::PrivateMarketsFund.to_string()
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "PrivateMarketsFund(id='{}', events={})",
            self.inner.id,
            self.inner.events.len()
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsPrivateMarketsFund {
        JsPrivateMarketsFund::from_inner(self.inner.clone())
    }
}
