//! WASM bindings for [`finstack_core::currency::Currency`].

use crate::utils::to_js_err;
use finstack_core::currency::Currency as RustCurrency;
use std::str::FromStr;
use wasm_bindgen::prelude::*;

/// ISO 4217 currency code wrapper for JavaScript.
#[wasm_bindgen(js_name = Currency)]
pub struct Currency {
    #[wasm_bindgen(skip)]
    pub(crate) inner: RustCurrency,
}

#[wasm_bindgen(js_class = Currency)]
impl Currency {
    /// Parses a case-insensitive ISO currency code (e.g. `"USD"`).
    #[wasm_bindgen(constructor)]
    pub fn new(code: &str) -> Result<Currency, JsValue> {
        RustCurrency::from_str(code.trim())
            .map(|inner| Currency { inner })
            .map_err(to_js_err)
    }

    /// Three-letter currency code.
    #[wasm_bindgen(getter, js_name = code)]
    pub fn code(&self) -> String {
        self.inner.to_string()
    }

    /// ISO 4217 numeric code.
    #[wasm_bindgen(getter, js_name = numeric)]
    pub fn numeric(&self) -> u16 {
        self.inner as u16
    }

    /// Number of decimal places (minor units) for this currency.
    #[wasm_bindgen(getter, js_name = decimals)]
    pub fn decimals(&self) -> u8 {
        self.inner.decimals()
    }

    /// Human-readable code (same as `code`).
    #[wasm_bindgen(js_name = toString)]
    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        self.inner.to_string()
    }

    /// Serialize to a JSON string.
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<String, JsValue> {
        serde_json::to_string(&self.inner).map_err(to_js_err)
    }

    /// Deserialize from a JSON string.
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(json: &str) -> Result<Currency, JsValue> {
        let inner: RustCurrency = serde_json::from_str(json).map_err(to_js_err)?;
        Ok(Currency { inner })
    }
}
