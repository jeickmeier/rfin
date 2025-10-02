use crate::core::error::js_error;
use serde_json;
use finstack_valuations::instruments::abs::Abs;
use finstack_valuations::instruments::basket::Basket;
use finstack_valuations::instruments::clo::Clo;
use finstack_valuations::instruments::cmbs::Cmbs;
use finstack_valuations::instruments::rmbs::Rmbs;
use finstack_valuations::pricer::InstrumentType;
use wasm_bindgen::prelude::*;

// ===========================
// Basket
// ===========================

#[wasm_bindgen(js_name = Basket)]
#[derive(Clone, Debug)]
pub struct JsBasket {
    inner: Basket,
}

impl JsBasket {
    pub(crate) fn from_inner(inner: Basket) -> Self {
        Self { inner }
    }

    pub(crate) fn inner(&self) -> Basket {
        self.inner.clone()
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
        self.inner.id.as_str().to_string()
    }

    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<String, JsValue> {
        serde_json::to_string_pretty(&self.inner).map_err(|e| js_error(e.to_string()))
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::Basket as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "Basket(id='{}', constituents={})",
            self.inner.id,
            self.inner.constituents.len()
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsBasket {
        JsBasket::from_inner(self.inner.clone())
    }
}

// ===========================
// ABS
// ===========================

#[wasm_bindgen(js_name = Abs)]
#[derive(Clone, Debug)]
pub struct JsAbs {
    inner: Abs,
}

impl JsAbs {
    pub(crate) fn from_inner(inner: Abs) -> Self {
        Self { inner }
    }

    pub(crate) fn inner(&self) -> Abs {
        self.inner.clone()
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
        self.inner.id.as_str().to_string()
    }

    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<String, JsValue> {
        serde_json::to_string_pretty(&self.inner).map_err(|e| js_error(e.to_string()))
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::ABS as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!("Abs(id='{}', tranches={})", self.inner.id, self.inner.tranches.tranches.len())
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsAbs {
        JsAbs::from_inner(self.inner.clone())
    }
}

// ===========================
// CLO
// ===========================

#[wasm_bindgen(js_name = Clo)]
#[derive(Clone, Debug)]
pub struct JsClo {
    inner: Clo,
}

impl JsClo {
    pub(crate) fn from_inner(inner: Clo) -> Self {
        Self { inner }
    }

    pub(crate) fn inner(&self) -> Clo {
        self.inner.clone()
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
        self.inner.id.as_str().to_string()
    }

    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<String, JsValue> {
        serde_json::to_string_pretty(&self.inner).map_err(|e| js_error(e.to_string()))
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::CLO as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!("Clo(id='{}', tranches={})", self.inner.id, self.inner.tranches.tranches.len())
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsClo {
        JsClo::from_inner(self.inner.clone())
    }
}

// ===========================
// CMBS
// ===========================

#[wasm_bindgen(js_name = Cmbs)]
#[derive(Clone, Debug)]
pub struct JsCmbs {
    inner: Cmbs,
}

impl JsCmbs {
    pub(crate) fn from_inner(inner: Cmbs) -> Self {
        Self { inner }
    }

    pub(crate) fn inner(&self) -> Cmbs {
        self.inner.clone()
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
        self.inner.id.as_str().to_string()
    }

    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<String, JsValue> {
        serde_json::to_string_pretty(&self.inner).map_err(|e| js_error(e.to_string()))
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::CMBS as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!("Cmbs(id='{}', tranches={})", self.inner.id, self.inner.tranches.tranches.len())
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsCmbs {
        JsCmbs::from_inner(self.inner.clone())
    }
}

// ===========================
// RMBS
// ===========================

#[wasm_bindgen(js_name = Rmbs)]
#[derive(Clone, Debug)]
pub struct JsRmbs {
    inner: Rmbs,
}

impl JsRmbs {
    pub(crate) fn from_inner(inner: Rmbs) -> Self {
        Self { inner }
    }

    pub(crate) fn inner(&self) -> Rmbs {
        self.inner.clone()
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
        self.inner.id.as_str().to_string()
    }

    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<String, JsValue> {
        serde_json::to_string_pretty(&self.inner).map_err(|e| js_error(e.to_string()))
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::RMBS as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!("Rmbs(id='{}', tranches={})", self.inner.id, self.inner.tranches.tranches.len())
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsRmbs {
        JsRmbs::from_inner(self.inner.clone())
    }
}

