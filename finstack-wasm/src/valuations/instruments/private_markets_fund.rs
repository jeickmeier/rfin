use crate::core::currency::JsCurrency;
use crate::core::error::js_error;
use serde_json;
use finstack_valuations::instruments::private_markets_fund::PrivateMarketsFund;
use finstack_valuations::pricer::InstrumentType;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = PrivateMarketsFund)]
#[derive(Clone, Debug)]
pub struct JsPrivateMarketsFund {
    inner: PrivateMarketsFund,
}

impl JsPrivateMarketsFund {
    pub(crate) fn from_inner(inner: PrivateMarketsFund) -> Self {
        Self { inner }
    }

    pub(crate) fn inner(&self) -> PrivateMarketsFund {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = PrivateMarketsFund)]
impl JsPrivateMarketsFund {
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(json_str: &str) -> Result<JsPrivateMarketsFund, JsValue> {
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
    pub fn to_json(&self) -> Result<String, JsValue> {
        serde_json::to_string_pretty(&self.inner).map_err(|e| js_error(e.to_string()))
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::PrivateMarketsFund as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!("PrivateMarketsFund(id='{}', events={})", self.inner.id, self.inner.events.len())
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsPrivateMarketsFund {
        JsPrivateMarketsFund::from_inner(self.inner.clone())
    }
}

