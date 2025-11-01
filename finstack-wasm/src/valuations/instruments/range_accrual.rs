use crate::valuations::instruments::InstrumentWrapper;
use finstack_valuations::instruments::range_accrual::RangeAccrual;
use finstack_valuations::pricer::InstrumentType;
use serde_json;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = RangeAccrual)]
#[derive(Clone, Debug)]
pub struct JsRangeAccrual(RangeAccrual);

impl InstrumentWrapper for JsRangeAccrual {
    type Inner = RangeAccrual;
    fn from_inner(inner: RangeAccrual) -> Self {
        JsRangeAccrual(inner)
    }
    fn inner(&self) -> RangeAccrual {
        self.0.clone()
    }
}

#[wasm_bindgen(js_class = RangeAccrual)]
impl JsRangeAccrual {
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(json_str: &str) -> Result<JsRangeAccrual, JsValue> {
        use crate::core::error::js_error;
        serde_json::from_str(json_str)
            .map(JsRangeAccrual::from_inner)
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
        InstrumentType::RangeAccrual as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!("RangeAccrual(id='{}')", self.0.id.as_str())
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsRangeAccrual {
        JsRangeAccrual::from_inner(self.0.clone())
    }
}

