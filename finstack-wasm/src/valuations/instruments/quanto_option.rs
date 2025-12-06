use crate::valuations::instruments::InstrumentWrapper;
use finstack_valuations::instruments::quanto_option::QuantoOption;
use finstack_valuations::pricer::InstrumentType;
use serde_json;
use wasm_bindgen::prelude::*;

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
    pub fn to_json(&self) -> Result<String, JsValue> {
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
