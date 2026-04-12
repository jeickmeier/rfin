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

    /// Multiply by a scalar; errors if `factor` is not finite.
    #[wasm_bindgen(js_name = mulScalar)]
    pub fn mul_scalar(&self, factor: f64) -> Result<Money, JsValue> {
        if !factor.is_finite() {
            return Err(to_js_err(format!(
                "mul_scalar factor must be finite, got {factor}"
            )));
        }
        Ok(Money {
            inner: self.inner * factor,
        })
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

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn usd() -> Currency {
        Currency::new("USD").expect("USD")
    }

    #[test]
    fn construct_and_getters() {
        let m = Money::new(10.0, &usd()).expect("valid");
        assert!((m.amount() - 10.0).abs() < 1e-10);
        assert_eq!(m.currency().code(), "USD");
    }

    #[test]
    fn add_same_currency() {
        let a = Money::new(10.0, &usd()).expect("valid");
        let b = Money::new(5.0, &usd()).expect("valid");
        let c = a.add(&b).expect("add");
        assert!((c.amount() - 15.0).abs() < 1e-10);
    }

    #[test]
    fn sub_same_currency() {
        let a = Money::new(10.0, &usd()).expect("valid");
        let b = Money::new(3.0, &usd()).expect("valid");
        let c = a.sub(&b).expect("sub");
        assert!((c.amount() - 7.0).abs() < 1e-10);
    }

    #[test]
    fn mul_scalar() {
        let m = Money::new(10.0, &usd()).expect("valid");
        let scaled = m.mul_scalar(2.5).expect("finite factor");
        assert!((scaled.amount() - 25.0).abs() < 1e-10);
    }

    // mul_scalar error-path tests live in tests/wasm_*.rs (requires wasm32)
    // because Err(JsValue) panics on native targets.

    #[test]
    fn div_scalar() {
        let m = Money::new(10.0, &usd()).expect("valid");
        let half = m.div_scalar(2.0).expect("div");
        assert!((half.amount() - 5.0).abs() < 1e-10);
    }

    #[test]
    fn negate() {
        let m = Money::new(10.0, &usd()).expect("valid");
        let neg = m.negate();
        assert!((neg.amount() + 10.0).abs() < 1e-10);
    }

    #[test]
    fn to_string_format() {
        let m = Money::new(10.0, &usd()).expect("valid");
        let s = m.to_string();
        assert!(s.contains("USD"), "expected USD in: {s}");
        assert!(s.contains("10"), "expected 10 in: {s}");
    }

    #[test]
    fn sub_different_via_inner() {
        let a = RustMoney::try_new(10.0, finstack_core::currency::Currency::USD).expect("ok");
        let b = RustMoney::try_new(5.0, finstack_core::currency::Currency::EUR).expect("ok");
        assert!(a.checked_sub(b).is_err());
    }

    // Error paths through wasm-bindgen create JsValue, which panics on
    // native targets.  Test the underlying Rust types instead.

    #[test]
    fn new_rejects_nan() {
        assert!(RustMoney::try_new(f64::NAN, finstack_core::currency::Currency::USD).is_err());
    }

    #[test]
    fn new_rejects_infinity() {
        assert!(RustMoney::try_new(f64::INFINITY, finstack_core::currency::Currency::USD).is_err());
    }

    #[test]
    fn div_scalar_rejects_zero() {
        let m = RustMoney::try_new(10.0, finstack_core::currency::Currency::USD).expect("valid");
        assert!(m.checked_div_f64(0.0).is_err());
    }

    #[test]
    fn negate_zero() {
        let m = Money::new(0.0, &usd()).expect("valid");
        let neg = m.negate();
        assert!(neg.amount().abs() < 1e-12);
    }
}
