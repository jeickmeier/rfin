use crate::core::utils::{js_array_from_iter, js_error};
use finstack_core::currency::Currency;
use std::str::FromStr;
use strum::IntoEnumIterator;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

#[wasm_bindgen(js_name = Currency)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct JsCurrency {
    inner: Currency,
}

impl JsCurrency {
    pub(crate) fn from_inner(inner: Currency) -> Self {
        Self { inner }
    }

    pub(crate) fn inner(&self) -> Currency {
        self.inner
    }
}

#[wasm_bindgen(js_class = Currency)]
impl JsCurrency {
    #[wasm_bindgen(constructor)]
    pub fn new(code: &str) -> Result<JsCurrency, JsValue> {
        Currency::from_str(code)
            .map(Self::from_inner)
            .map_err(|_| js_error(format!("Unknown currency code: {code}")))
    }

    #[wasm_bindgen(js_name = fromNumeric)]
    pub fn from_numeric(numeric: u16) -> Result<JsCurrency, JsValue> {
        Currency::try_from(numeric)
            .map(Self::from_inner)
            .map_err(|_| js_error(format!("Unknown currency numeric code: {numeric}")))
    }

    #[wasm_bindgen(getter)]
    pub fn code(&self) -> String {
        self.inner.to_string()
    }

    #[wasm_bindgen(getter)]
    pub fn numeric(&self) -> u16 {
        self.inner as u16
    }

    #[wasm_bindgen(getter)]
    pub fn decimals(&self) -> u8 {
        self.inner.decimals()
    }

    #[wasm_bindgen(js_name = toTuple)]
    pub fn to_tuple(&self) -> js_sys::Array {
        let tuple = js_sys::Array::new();
        tuple.push(&JsValue::from(self.code()));
        tuple.push(&JsValue::from_f64(self.numeric() as f64));
        tuple.push(&JsValue::from_f64(self.decimals() as f64));
        tuple
    }

    #[wasm_bindgen(js_name = all)]
    pub fn all() -> js_sys::Array {
        let currencies = Currency::iter().map(JsCurrency::from_inner);
        js_array_from_iter(currencies.map(JsValue::from))
    }
}
