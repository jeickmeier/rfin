use crate::core::error::js_error;
use crate::valuations::instruments::InstrumentWrapper;
use finstack_valuations::instruments::abs::Abs;
use finstack_valuations::instruments::basket::Basket;
use finstack_valuations::instruments::clo::Clo;
use finstack_valuations::instruments::cmbs::Cmbs;
use finstack_valuations::instruments::rmbs::Rmbs;
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
// ABS
// ===========================

#[wasm_bindgen(js_name = Abs)]
#[derive(Clone, Debug)]
pub struct JsAbs(Abs);

impl InstrumentWrapper for JsAbs {
    type Inner = Abs;
    fn from_inner(inner: Abs) -> Self {
        JsAbs(inner)
    }
    fn inner(&self) -> Abs {
        self.0.clone()
    }
}

#[wasm_bindgen(js_class = Abs)]
impl JsAbs {
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(json_str: &str) -> Result<JsAbs, JsValue> {
        serde_json::from_str(json_str)
            .map(JsAbs::from_inner)
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
        InstrumentType::ABS as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "Abs(id='{}', tranches={})",
            self.0.id,
            self.0.tranches.tranches.len()
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsAbs {
        JsAbs::from_inner(self.0.clone())
    }
}

// ===========================
// CLO
// ===========================

#[wasm_bindgen(js_name = Clo)]
#[derive(Clone, Debug)]
pub struct JsClo(Clo);

impl InstrumentWrapper for JsClo {
    type Inner = Clo;
    fn from_inner(inner: Clo) -> Self {
        JsClo(inner)
    }
    fn inner(&self) -> Clo {
        self.0.clone()
    }
}

#[wasm_bindgen(js_class = Clo)]
impl JsClo {
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(json_str: &str) -> Result<JsClo, JsValue> {
        serde_json::from_str(json_str)
            .map(JsClo::from_inner)
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
        InstrumentType::CLO as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "Clo(id='{}', tranches={})",
            self.0.id,
            self.0.tranches.tranches.len()
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsClo {
        JsClo::from_inner(self.0.clone())
    }
}

// ===========================
// CMBS
// ===========================

#[wasm_bindgen(js_name = Cmbs)]
#[derive(Clone, Debug)]
pub struct JsCmbs(Cmbs);

impl InstrumentWrapper for JsCmbs {
    type Inner = Cmbs;
    fn from_inner(inner: Cmbs) -> Self {
        JsCmbs(inner)
    }
    fn inner(&self) -> Cmbs {
        self.0.clone()
    }
}

#[wasm_bindgen(js_class = Cmbs)]
impl JsCmbs {
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(json_str: &str) -> Result<JsCmbs, JsValue> {
        serde_json::from_str(json_str)
            .map(JsCmbs::from_inner)
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
        InstrumentType::CMBS as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "Cmbs(id='{}', tranches={})",
            self.0.id,
            self.0.tranches.tranches.len()
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsCmbs {
        JsCmbs::from_inner(self.0.clone())
    }
}

// ===========================
// RMBS
// ===========================

#[wasm_bindgen(js_name = Rmbs)]
#[derive(Clone, Debug)]
pub struct JsRmbs(Rmbs);

impl InstrumentWrapper for JsRmbs {
    type Inner = Rmbs;
    fn from_inner(inner: Rmbs) -> Self {
        JsRmbs(inner)
    }
    fn inner(&self) -> Rmbs {
        self.0.clone()
    }
}

#[wasm_bindgen(js_class = Rmbs)]
impl JsRmbs {
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(json_str: &str) -> Result<JsRmbs, JsValue> {
        serde_json::from_str(json_str)
            .map(JsRmbs::from_inner)
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
        InstrumentType::RMBS as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "Rmbs(id='{}', tranches={})",
            self.0.id,
            self.0.tranches.tranches.len()
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsRmbs {
        JsRmbs::from_inner(self.0.clone())
    }
}
