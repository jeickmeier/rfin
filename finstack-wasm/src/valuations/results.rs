use crate::core::dates::date::JsDate;
use crate::core::money::JsMoney;
use finstack_valuations::results::ValuationResult;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

#[wasm_bindgen(js_name = ValuationResult)]
#[derive(Clone)]
pub struct JsValuationResult {
    inner: ValuationResult,
}

impl JsValuationResult {
    pub(crate) fn new(inner: ValuationResult) -> Self {
        Self { inner }
    }

    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> &ValuationResult {
        &self.inner
    }
}

#[wasm_bindgen(js_class = ValuationResult)]
impl JsValuationResult {
    #[wasm_bindgen(getter, js_name = instrumentId)]
    pub fn instrument_id(&self) -> String {
        self.inner.instrument_id.clone()
    }

    #[wasm_bindgen(getter, js_name = asOf)]
    pub fn as_of(&self) -> JsDate {
        JsDate::from_core(self.inner.as_of)
    }

    #[wasm_bindgen(getter, js_name = presentValue)]
    pub fn present_value(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.value)
    }

    #[wasm_bindgen(js_name = metric)]
    pub fn metric(&self, name: &str) -> Option<f64> {
        self.inner.measures.get(name).copied()
    }

    #[wasm_bindgen(getter, js_name = measures)]
    pub fn measures(&self) -> js_sys::Map {
        let map = js_sys::Map::new();
        for (key, value) in &self.inner.measures {
            map.set(&JsValue::from_str(key), &JsValue::from_f64(*value));
        }
        map
    }
}
