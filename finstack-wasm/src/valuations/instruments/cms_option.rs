use crate::valuations::instruments::InstrumentWrapper;
use finstack_valuations::instruments::cms_option::CmsOption;
use finstack_valuations::pricer::InstrumentType;
use serde_json;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = CmsOption)]
#[derive(Clone, Debug)]
pub struct JsCmsOption(CmsOption);

impl InstrumentWrapper for JsCmsOption {
    type Inner = CmsOption;
    fn from_inner(inner: CmsOption) -> Self {
        JsCmsOption(inner)
    }
    fn inner(&self) -> CmsOption {
        self.0.clone()
    }
}

#[wasm_bindgen(js_class = CmsOption)]
impl JsCmsOption {
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(json_str: &str) -> Result<JsCmsOption, JsValue> {
        use crate::core::error::js_error;
        serde_json::from_str(json_str)
            .map(JsCmsOption::from_inner)
            .map_err(|e| js_error(e.to_string()))
    }

    #[wasm_bindgen(getter, js_name = instrumentId)]
    pub fn instrument_id(&self) -> String {
        self.0.id.as_str().to_string()
    }

    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<String, JsValue> {
        use crate::core::error::js_error;
        serde_json::to_string_pretty(&self.0).map_err(|e| js_error(e.to_string()))
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::CmsOption as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!("CmsOption(id='{}')", self.0.id.as_str())
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsCmsOption {
        JsCmsOption::from_inner(self.0.clone())
    }
}


