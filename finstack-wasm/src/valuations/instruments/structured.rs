use crate::core::error::js_error;
use crate::valuations::instruments::InstrumentWrapper;
use finstack_valuations::instruments::basket::Basket;
use finstack_valuations::instruments::structured_credit::StructuredCredit;
use finstack_valuations::pricer::InstrumentType;
use serde_json;
use wasm_bindgen::prelude::*;

// ===========================
// Basket
// ===========================

#[wasm_bindgen(js_name = Basket)]
#[derive(Clone, Debug)]
pub struct JsBasket(Basket);

impl InstrumentWrapper for JsBasket {
    type Inner = Basket;
    fn from_inner(inner: Basket) -> Self {
        JsBasket(inner)
    }
    fn inner(&self) -> Basket {
        self.0.clone()
    }
}

#[wasm_bindgen(js_class = Basket)]
impl JsBasket {
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(json_str: &str) -> Result<JsBasket, JsValue> {
        serde_json::from_str(json_str)
            .map(JsBasket::from_inner)
            .map_err(|e| js_error(e.to_string()))
    }

    #[wasm_bindgen(getter, js_name = instrumentId)]
    pub fn instrument_id(&self) -> String {
        self.0.id.as_str().to_string()
    }

    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<String, JsValue> {
        serde_json::to_string_pretty(&self.0).map_err(|e| js_error(e.to_string()))
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::Basket as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "Basket(id='{}', constituents={})",
            self.0.id,
            self.0.constituents.len()
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsBasket {
        JsBasket::from_inner(self.0.clone())
    }
}

// ===========================
// Unified Structured Credit
// ===========================

#[wasm_bindgen(js_name = StructuredCredit)]
#[derive(Clone, Debug)]
pub struct JsStructuredCredit(StructuredCredit);

impl InstrumentWrapper for JsStructuredCredit {
    type Inner = StructuredCredit;
    fn from_inner(inner: StructuredCredit) -> Self {
        JsStructuredCredit(inner)
    }
    fn inner(&self) -> StructuredCredit {
        self.0.clone()
    }
}

#[wasm_bindgen(js_class = StructuredCredit)]
impl JsStructuredCredit {
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(json_str: &str) -> Result<JsStructuredCredit, JsValue> {
        serde_json::from_str(json_str)
            .map(JsStructuredCredit::from_inner)
            .map_err(|e| js_error(e.to_string()))
    }

    #[wasm_bindgen(getter, js_name = instrumentId)]
    pub fn instrument_id(&self) -> String {
        self.0.id.as_str().to_string()
    }

    #[wasm_bindgen(getter, js_name = dealType)]
    pub fn deal_type(&self) -> String {
        format!("{:?}", self.0.deal_type)
    }

    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<String, JsValue> {
        serde_json::to_string_pretty(&self.0).map_err(|e| js_error(e.to_string()))
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::StructuredCredit as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "StructuredCredit({:?}, id='{}', tranches={})",
            self.0.deal_type,
            self.0.id,
            self.0.tranches.tranches.len()
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsStructuredCredit {
        JsStructuredCredit::from_inner(self.0.clone())
    }

    #[wasm_bindgen(getter, js_name = trancheCount)]
    pub fn tranche_count(&self) -> usize {
        self.0.tranches.tranches.len()
    }
}

// ===========================
// Legacy Type Aliases for Backward Compatibility
// ===========================
// These allow existing JavaScript/TypeScript code to continue using
// Abs, Clo, Cmbs, Rmbs while internally using the unified type

pub type JsAbs = JsStructuredCredit;
pub type JsClo = JsStructuredCredit;
pub type JsCmbs = JsStructuredCredit;
pub type JsRmbs = JsStructuredCredit;
