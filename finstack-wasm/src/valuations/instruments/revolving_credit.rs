use crate::utils::json::to_js_value;
use crate::valuations::instruments::InstrumentWrapper;
use finstack_valuations::instruments::fixed_income::revolving_credit::RevolvingCredit;
use finstack_valuations::pricer::InstrumentType;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = RevolvingCredit)]
#[derive(Clone, Debug)]
pub struct JsRevolvingCredit {
    pub(crate) inner: RevolvingCredit,
}

impl InstrumentWrapper for JsRevolvingCredit {
    type Inner = RevolvingCredit;
    fn from_inner(inner: RevolvingCredit) -> Self {
        JsRevolvingCredit { inner }
    }
    fn inner(&self) -> RevolvingCredit {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = RevolvingCredit)]
impl JsRevolvingCredit {
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(json_str: &str) -> Result<JsRevolvingCredit, JsValue> {
        use crate::core::error::js_error;
        serde_json::from_str(json_str)
            .map(JsRevolvingCredit::from_inner)
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

    #[wasm_bindgen(js_name = toJsonString)]
    pub fn to_json_string(&self) -> Result<String, JsValue> {
        use crate::core::error::js_error;
        serde_json::to_string_pretty(&self.inner).map_err(|e| js_error(e.to_string()))
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::RevolvingCredit as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!("RevolvingCredit(id='{}')", self.inner.id.as_str())
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsRevolvingCredit {
        JsRevolvingCredit::from_inner(self.inner.clone())
    }
}
