use crate::core::config::JsFinstackConfig;
use crate::core::currency::JsCurrency;
use crate::core::error::js_error;
use finstack_core::currency::Currency;
use finstack_core::money::Money;
use js_sys::Array;
use std::str::FromStr;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

#[wasm_bindgen(js_name = Money)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JsMoney {
    inner: Money,
}

impl JsMoney {
    pub(crate) fn from_inner(inner: Money) -> Self {
        Self { inner }
    }

    pub(crate) fn inner(&self) -> Money {
        self.inner
    }
}

fn money_from_tuple(value: &JsValue) -> Result<Money, JsValue> {
    if !js_sys::Array::is_array(value) {
        return Err(js_error(
            "Money tuple must be provided as [amount, currencyCode]",
        ));
    }

    let array = Array::from(value);
    if array.length() != 2 {
        return Err(js_error(
            "Money tuple must have exactly two elements: [amount, currencyCode]",
        ));
    }

    let amount = array
        .get(0)
        .as_f64()
        .ok_or_else(|| js_error("Money tuple amount must be a number"))?;

    let code = array
        .get(1)
        .as_string()
        .ok_or_else(|| js_error("Money tuple currency must be an ISO code string"))?;
    let currency = Currency::from_str(&code)
        .map_err(|_| js_error(format!("Unknown currency code: {code}")))?;

    Ok(Money::new(amount, currency))
}

#[wasm_bindgen(js_class = Money)]
impl JsMoney {
    /// Create a money amount with the provided currency.
    ///
    /// @param {number} amount - Numeric value expressed in the currency's units
    /// @param {Currency} currency - Currency instance defining the legal tender
    /// @returns {Money} Money instance representing the amount in the given currency
    ///
    /// @example
    /// ```javascript
    /// const usd = new Currency("USD");
    /// const amount = new Money(1234.567, usd);
    /// console.log(amount.format());  // "USD 1234.57" (rounded to 2 decimals)
    /// console.log(amount.amount);    // 1234.567
    /// ```
    #[wasm_bindgen(constructor)]
    pub fn new(amount: f64, currency: &JsCurrency) -> JsMoney {
        Self::from_inner(Money::new(amount, currency.inner()))
    }

    /// Create a zero amount in the requested currency.
    ///
    /// @param {Currency} currency - Currency for the zero amount
    /// @returns {Money} Money instance with amount 0 in the given currency
    ///
    /// @example
    /// ```javascript
    /// const usd = new Currency("USD");
    /// const zero = Money.zero(usd);
    /// console.log(zero.amount);     // 0
    /// console.log(zero.format());   // "USD 0.00"
    /// ```
    #[wasm_bindgen(js_name = zero)]
    pub fn zero(currency: &JsCurrency) -> JsMoney {
        Self::from_inner(Money::new(0.0, currency.inner()))
    }

    /// Amount as a floating-point value.
    ///
    /// @type {number}
    /// @readonly
    ///
    /// @example
    /// ```javascript
    /// const money = Money.fromCode(123.45, "USD");
    /// console.log(money.amount);  // 123.45
    /// ```
    #[wasm_bindgen(getter)]
    pub fn amount(&self) -> f64 {
        self.inner.amount()
    }

    /// Currency of this amount.
    ///
    /// @type {Currency}
    /// @readonly
    ///
    /// @example
    /// ```javascript
    /// const money = Money.fromCode(100, "EUR");
    /// console.log(money.currency.code);  // "EUR"
    /// ```
    #[wasm_bindgen(getter)]
    pub fn currency(&self) -> JsCurrency {
        JsCurrency::from_inner(self.inner.currency())
    }

    /// Return [amount, currency] tuple for serialization or interop.
    ///
    /// @returns {Array} Tuple containing [number, Currency]
    ///
    /// @example
    /// ```javascript
    /// const money = Money.fromCode(250.50, "GBP");
    /// const [amt, curr] = money.toTuple();
    /// console.log(amt);         // 250.5
    /// console.log(curr.code);   // "GBP"
    /// ```
    #[wasm_bindgen(js_name = toTuple)]
    pub fn to_tuple(&self) -> Array {
        let tuple = Array::new();
        tuple.push(&JsValue::from_f64(self.amount()));
        tuple.push(&JsValue::from(JsCurrency::from_inner(
            self.inner.currency(),
        )));
        tuple
    }

    /// Construct from an [amount, Currency] or [amount, currencyCode] array.
    ///
    /// @param {Array} value - Array containing [number, Currency | string]
    /// @returns {Money} Money instance matching the input
    /// @throws {Error} If the array format is invalid or currency code is unknown
    ///
    /// @example
    /// ```javascript
    /// // From currency object
    /// const usd = new Currency("USD");
    /// const money1 = Money.fromTuple([100, usd]);
    ///
    /// // From currency code string
    /// const money2 = Money.fromTuple([100, "USD"]);
    ///
    /// console.log(money1.amount);  // 100
    /// ```
    #[wasm_bindgen(js_name = fromTuple)]
    pub fn from_tuple(value: &JsValue) -> Result<JsMoney, JsValue> {
        money_from_tuple(value).map(Self::from_inner)
    }

    /// Construct a money value using a configuration for ingest rounding.
    ///
    /// @param {number} amount - Raw monetary value
    /// @param {Currency} currency - Currency instance
    /// @param {FinstackConfig} config - Configuration controlling ingest rounding/scale
    /// @returns {Money} Money instance respecting custom ingest rules
    ///
    /// @example
    /// ```javascript
    /// const cfg = new FinstackConfig();
    /// cfg.setIngestScale(new Currency("JPY"), 4);  // Allow 4 decimals for pipettes
    ///
    /// const jpy = new Currency("JPY");
    /// const precise = Money.fromConfig(123.4567, jpy, cfg);
    /// console.log(precise.amount);  // 123.4567 (respects custom scale)
    /// ```
    #[wasm_bindgen(js_name = fromConfig)]
    pub fn from_config(amount: f64, currency: &JsCurrency, config: &JsFinstackConfig) -> JsMoney {
        JsMoney::from_inner(Money::new_with_config(
            amount,
            currency.inner(),
            config.inner(),
        ))
    }

    /// Format using ISO minor units (e.g., "USD 10.00").
    ///
    /// @returns {string} Formatted string with ISO code prefix
    ///
    /// @example
    /// ```javascript
    /// const money = Money.fromCode(1234.567, "USD");
    /// console.log(money.format());  // "USD 1234.57" (rounded to 2 decimals)
    ///
    /// const jpy = Money.fromCode(1234.567, "JPY");
    /// console.log(jpy.format());    // "JPY 1235" (rounded to 0 decimals)
    /// ```
    #[wasm_bindgen(js_name = format)]
    pub fn format(&self) -> String {
        format!("{}", self.inner)
    }

    /// Create money directly from an amount and ISO currency code (ergonomic helper).
    ///
    /// @param {number} amount - Monetary value
    /// @param {string} code - Three-letter ISO currency code
    /// @returns {Money} Money instance with the specified amount and currency
    /// @throws {Error} If the currency code is invalid
    ///
    /// @example
    /// ```javascript
    /// // Most ergonomic way to create money
    /// const amount = Money.fromCode(42.50, "EUR");
    /// console.log(amount.format());  // "EUR 42.50"
    ///
    /// // Equivalent to:
    /// const eur = new Currency("EUR");
    /// const amount2 = new Money(42.50, eur);
    /// ```
    #[wasm_bindgen(js_name = fromCode)]
    pub fn from_code(amount: f64, code: &str) -> Result<JsMoney, JsValue> {
        let currency = Currency::from_str(code)
            .map_err(|_| js_error(format!("Unknown currency code: {code}")))?;
        Ok(Self::from_inner(Money::new(amount, currency)))
    }
}
