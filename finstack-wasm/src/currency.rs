//! WASM bindings for Currency type.

use finstack_core::currency::Currency as CoreCurrency;
use wasm_bindgen::prelude::*;

/// WASM wrapper for the Currency enum
#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct Currency {
    inner: CoreCurrency,
}

#[wasm_bindgen]
impl Currency {
    /// Create a new currency from a string code
    #[wasm_bindgen(constructor)]
    pub fn new(code: String) -> Result<Currency, JsValue> {
        let currency = code
            .parse::<CoreCurrency>()
            .map_err(|e| JsValue::from_str(&format!("Invalid currency code: {}", e)))?;
        Ok(Currency { inner: currency })
    }

    /// Get the currency code as a string
    #[wasm_bindgen(getter)]
    pub fn code(&self) -> String {
        format!("{}", self.inner)
    }

    /// Get the ISO 4217 numeric code
    #[wasm_bindgen(getter, js_name = "numericCode")]
    pub fn numeric_code(&self) -> u16 {
        self.inner as u16
    }

    /// Get the number of decimal places defined by ISO-4217.
    #[wasm_bindgen(getter, js_name = "decimals")]
    pub fn decimals(&self) -> u8 {
        self.inner.decimals()
    }

    /// Convert to string
    #[wasm_bindgen(js_name = "toString")]
    pub fn to_string_js(&self) -> String {
        format!("{}", self.inner)
    }

    /// Check equality with another Currency
    #[wasm_bindgen]
    pub fn equals(&self, other: &Currency) -> bool {
        self.inner == other.inner
    }
}

impl Currency {
    /// Create a new Currency from CoreCurrency (internal use)
    pub fn from_inner(inner: CoreCurrency) -> Self {
        Self { inner }
    }

    /// Get the inner Currency enum
    pub fn inner(&self) -> CoreCurrency {
        self.inner
    }
}
