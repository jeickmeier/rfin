//! WASM bindings for Money type.

use super::currency::Currency;
use js_sys::Array;
use rfin_core::primitives::money::Money as CoreMoney;
use wasm_bindgen::prelude::*;

/// Money representation
#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct Money {
    inner: CoreMoney<f64>,
}

#[wasm_bindgen]
impl Money {
    /// Create new money instance
    #[wasm_bindgen(constructor)]
    pub fn new(amount: f64, currency: Currency) -> Self {
        Money {
            inner: CoreMoney::new(amount, currency.inner()),
        }
    }

    /// Create Money in USD
    #[wasm_bindgen(js_name = "usd")]
    pub fn usd(amount: f64) -> Self {
        Money {
            inner: CoreMoney::usd(amount),
        }
    }

    /// Create Money in EUR
    #[wasm_bindgen(js_name = "eur")]
    pub fn eur(amount: f64) -> Self {
        Money {
            inner: CoreMoney::eur(amount),
        }
    }

    /// Get the amount
    #[wasm_bindgen(getter)]
    pub fn amount(&self) -> f64 {
        *self.inner.amount()
    }

    /// Get the currency
    #[wasm_bindgen(getter)]
    pub fn currency(&self) -> Currency {
        Currency::from_inner(self.inner.currency())
    }

    /// Add two Money values (same currency required)
    #[wasm_bindgen]
    pub fn add(&self, other: &Money) -> Result<Money, JsValue> {
        match std::panic::catch_unwind(|| self.inner + other.inner) {
            Ok(result) => Ok(Money { inner: result }),
            Err(_) => Err(JsValue::from_str(&format!(
                "Cannot add money with different currencies: {} and {}",
                self.inner.currency(),
                other.inner.currency()
            ))),
        }
    }

    /// Subtract two Money values (same currency required)
    #[wasm_bindgen]
    pub fn subtract(&self, other: &Money) -> Result<Money, JsValue> {
        match std::panic::catch_unwind(|| self.inner - other.inner) {
            Ok(result) => Ok(Money { inner: result }),
            Err(_) => Err(JsValue::from_str(&format!(
                "Cannot subtract money with different currencies: {} and {}",
                self.inner.currency(),
                other.inner.currency()
            ))),
        }
    }

    /// Multiply Money by a scalar
    #[wasm_bindgen]
    pub fn multiply(&self, scalar: f64) -> Money {
        Money {
            inner: self.inner * scalar,
        }
    }

    /// Divide Money by a scalar
    #[wasm_bindgen]
    pub fn divide(&self, scalar: f64) -> Money {
        Money {
            inner: self.inner / scalar,
        }
    }

    /// Convert to string
    #[wasm_bindgen(js_name = "toString")]
    pub fn to_string_js(&self) -> String {
        format!("{}", self.inner)
    }

    /// Check equality with another Money value
    #[wasm_bindgen]
    pub fn equals(&self, other: &Money) -> bool {
        self.inner == other.inner
    }

    /// Get amount and currency as separate values
    #[wasm_bindgen(js_name = "toParts")]
    pub fn to_parts(&self) -> Array {
        let (amount, currency) = self.inner.into_parts();
        let array = Array::new();
        array.push(&JsValue::from_f64(amount));
        array.push(&Currency::from_inner(currency).into());
        array
    }
}

impl Money {
    /// Get the inner Money type
    pub fn inner(&self) -> CoreMoney<f64> {
        self.inner
    }
}
