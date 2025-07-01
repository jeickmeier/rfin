//! WASM bindings for primitives module.

use rfin_core::primitives::currency::{Currency as CoreCurrency};
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
        let currency = code.parse::<CoreCurrency>()
            .map_err(|e| JsValue::from_str(&format!("Invalid currency code: {}", e)))?;
        Ok(Currency { inner: currency })
    }

    /// Create Currency::USD
    #[wasm_bindgen(js_name = "USD")]
    pub fn usd() -> Currency {
        Currency { inner: CoreCurrency::USD }
    }

    /// Create Currency::EUR
    #[wasm_bindgen(js_name = "EUR")]
    pub fn eur() -> Currency {
        Currency { inner: CoreCurrency::EUR }
    }

    /// Create Currency::GBP
    #[wasm_bindgen(js_name = "GBP")]
    pub fn gbp() -> Currency {
        Currency { inner: CoreCurrency::GBP }
    }

    /// Create Currency::JPY
    #[wasm_bindgen(js_name = "JPY")]
    pub fn jpy() -> Currency {
        Currency { inner: CoreCurrency::JPY }
    }

    /// Create Currency::CHF
    #[wasm_bindgen(js_name = "CHF")]
    pub fn chf() -> Currency {
        Currency { inner: CoreCurrency::CHF }
    }

    /// Create Currency::AUD
    #[wasm_bindgen(js_name = "AUD")]
    pub fn aud() -> Currency {
        Currency { inner: CoreCurrency::AUD }
    }

    /// Create Currency::CAD
    #[wasm_bindgen(js_name = "CAD")]
    pub fn cad() -> Currency {
        Currency { inner: CoreCurrency::CAD }
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

    /// Get the number of minor units (decimal places)
    #[wasm_bindgen(getter, js_name = "minorUnits")]
    pub fn minor_units(&self) -> u8 {
        self.inner.minor_units()
    }

    /// Convert to string
    #[wasm_bindgen(js_name = "toString")]
    pub fn to_string(&self) -> String {
        format!("{}", self.inner)
    }

    /// Check equality with another Currency
    #[wasm_bindgen]
    pub fn equals(&self, other: &Currency) -> bool {
        self.inner == other.inner
    }
}

impl Currency {
    /// Get the inner Currency enum
    pub fn inner(&self) -> CoreCurrency {
        self.inner
    }
}

/// Money representation
#[wasm_bindgen]
pub struct Money {
    // TODO: Wrap rfin_core::primitives::money::Money
    amount: f64,
    currency: Currency,
}

#[wasm_bindgen]
impl Money {
    /// Create new money instance
    #[wasm_bindgen(constructor)]
    pub fn new(amount: f64, currency: Currency) -> Self {
        Money { amount, currency }
    }

    /// Get the amount
    #[wasm_bindgen(getter)]
    pub fn amount(&self) -> f64 {
        self.amount
    }

    /// Get the currency
    #[wasm_bindgen(getter)]
    pub fn currency(&self) -> Currency {
        self.currency.clone()
    }
}
