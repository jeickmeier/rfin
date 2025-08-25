//! WASM bindings for Money type.

use super::currency::Currency;
use js_sys::Array;
use finstack_core::error::Error;
use finstack_core::money::Money as CoreMoney;
use wasm_bindgen::prelude::*;

/// Money representation
#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct Money {
    inner: CoreMoney,
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

    /// Get the amount
    #[wasm_bindgen(getter)]
    pub fn amount(&self) -> f64 {
        self.inner.amount()
    }

    /// Get the currency
    #[wasm_bindgen(getter)]
    pub fn currency(&self) -> Currency {
        Currency::from_inner(self.inner.currency())
    }

    /// Add two Money values (same currency required)
    #[wasm_bindgen]
    pub fn add(&self, other: &Money) -> Result<Money, JsValue> {
        match self.inner.checked_add(other.inner) {
            Ok(result) => Ok(Money { inner: result }),
            Err(Error::CurrencyMismatch { expected, actual }) => Err(JsValue::from_str(&format!(
                "Currency mismatch: expected {}, got {}",
                expected, actual
            ))),
            Err(err) => Err(JsValue::from_str(&format!(
                "Money addition failed: {}",
                err
            ))),
        }
    }

    /// Subtract two Money values (same currency required)
    #[wasm_bindgen]
    pub fn subtract(&self, other: &Money) -> Result<Money, JsValue> {
        match self.inner.checked_sub(other.inner) {
            Ok(result) => Ok(Money { inner: result }),
            Err(Error::CurrencyMismatch { expected, actual }) => Err(JsValue::from_str(&format!(
                "Currency mismatch: expected {}, got {}",
                expected, actual
            ))),
            Err(err) => Err(JsValue::from_str(&format!(
                "Money subtraction failed: {}",
                err
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

    /// Deprecated alias for backward compatibility: returns same as `toParts`.
    #[wasm_bindgen(js_name = "intoParts")]
    #[allow(clippy::wrong_self_convention)]
    pub fn into_parts_alias(&self) -> Array {
        self.to_parts()
    }

    /// Add two Money values with explicit error handling
    #[wasm_bindgen(js_name = "checkedAdd")]
    pub fn checked_add(&self, other: &Money) -> Result<Money, JsValue> {
        match self.inner.checked_add(other.inner) {
            Ok(result) => Ok(Money { inner: result }),
            Err(Error::CurrencyMismatch { expected, actual }) => Err(JsValue::from_str(&format!(
                "Currency mismatch: expected {}, got {}",
                expected, actual
            ))),
            Err(err) => Err(JsValue::from_str(&format!("Addition failed: {}", err))),
        }
    }

    /// Subtract two Money values with explicit error handling
    #[wasm_bindgen(js_name = "checkedSubtract")]
    pub fn checked_subtract(&self, other: &Money) -> Result<Money, JsValue> {
        match self.inner.checked_sub(other.inner) {
            Ok(result) => Ok(Money { inner: result }),
            Err(Error::CurrencyMismatch { expected, actual }) => Err(JsValue::from_str(&format!(
                "Currency mismatch: expected {}, got {}",
                expected, actual
            ))),
            Err(err) => Err(JsValue::from_str(&format!("Subtraction failed: {}", err))),
        }
    }
}

impl Money {
    /// Get the inner Money type
    pub fn inner(&self) -> CoreMoney {
        self.inner
    }
}
