//! WASM bindings for [`finstack_core::money::Money`].

use crate::api::core::currency::Currency;
use crate::utils::to_js_err;
use finstack_core::money::Money as RustMoney;
use wasm_bindgen::prelude::*;

/// Currency-tagged monetary amount.
#[wasm_bindgen(js_name = Money)]
pub struct Money {
    #[wasm_bindgen(skip)]
    pub(crate) inner: RustMoney,
}

#[wasm_bindgen(js_class = Money)]
impl Money {
    /// Creates a new money value using the currency’s ISO minor units and bankers rounding.
    ///
    /// Returns an error if `amount` is not finite or cannot be represented.
    #[wasm_bindgen(constructor)]
    pub fn new(amount: f64, currency: &Currency) -> Result<Money, JsValue> {
        RustMoney::try_new(amount, currency.inner)
            .map(|inner| Money { inner })
            .map_err(to_js_err)
    }

    /// Numeric amount in major units as `f64`.
    #[wasm_bindgen(getter, js_name = amount)]
    pub fn amount(&self) -> f64 {
        self.inner.amount()
    }

    /// Currency of this amount.
    #[wasm_bindgen(getter, js_name = currency)]
    pub fn currency(&self) -> Currency {
        Currency {
            inner: self.inner.currency(),
        }
    }

    /// Add two amounts; errors if currencies differ or the operation is not representable.
    #[wasm_bindgen(js_name = add)]
    pub fn add(&self, other: &Money) -> Result<Money, JsValue> {
        self.inner
            .checked_add(other.inner)
            .map(|inner| Money { inner })
            .map_err(to_js_err)
    }

    /// Subtract two amounts; errors if currencies differ or the operation is not representable.
    #[wasm_bindgen(js_name = sub)]
    pub fn sub(&self, other: &Money) -> Result<Money, JsValue> {
        self.inner
            .checked_sub(other.inner)
            .map(|inner| Money { inner })
            .map_err(to_js_err)
    }

    /// Multiply by a scalar (uses core `Mul<f64>` semantics).
    #[wasm_bindgen(js_name = mulScalar)]
    pub fn mul_scalar(&self, factor: f64) -> Money {
        Money {
            inner: self.inner * factor,
        }
    }

    /// Divide by a scalar; errors on division by zero or non-finite / non-representable values.
    #[wasm_bindgen(js_name = divScalar)]
    pub fn div_scalar(&self, divisor: f64) -> Result<Money, JsValue> {
        self.inner
            .checked_div_f64(divisor)
            .map(|inner| Money { inner })
            .map_err(to_js_err)
    }

    /// Negate the monetary amount.
    #[wasm_bindgen(js_name = negate)]
    pub fn negate(&self) -> Money {
        Money {
            inner: self.inner * -1.0,
        }
    }

    /// Default string representation (e.g. `"USD 10.00"`).
    #[wasm_bindgen(js_name = toString)]
    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        self.inner.to_string()
    }
}
