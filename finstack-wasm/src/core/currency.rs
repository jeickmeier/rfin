use crate::core::error::unknown_currency;
use crate::core::utils::js_array_from_iter;
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
    /// Construct a currency from a three-letter ISO code (case-insensitive).
    ///
    /// @param {string} code - Three-letter ISO currency code such as "USD" or "eur"
    /// @returns {Currency} Currency instance corresponding to the code
    /// @throws {Error} If the currency code is not recognized
    ///
    /// @example
    /// ```javascript
    /// const usd = new Currency("USD");
    /// console.log(usd.code);      // "USD"
    /// console.log(usd.numeric);   // 840
    /// console.log(usd.decimals);  // 2
    ///
    /// const eur = new Currency("eur");  // case-insensitive
    /// console.log(eur.code);      // "EUR"
    /// ```
    #[wasm_bindgen(constructor)]
    pub fn new(code: &str) -> Result<JsCurrency, JsValue> {
        Currency::from_str(code)
            .map(Self::from_inner)
            .map_err(|_| unknown_currency(code))
    }

    /// Construct from an ISO numeric currency code.
    ///
    /// @param {number} numeric - ISO-4217 numeric currency code (e.g., 840 for USD)
    /// @returns {Currency} Currency instance associated with the numeric code
    /// @throws {Error} If the numeric code is not recognized
    ///
    /// @example
    /// ```javascript
    /// const gbp = Currency.fromNumeric(826);
    /// console.log(gbp.code);  // "GBP"
    /// ```
    #[wasm_bindgen(js_name = fromNumeric)]
    pub fn from_numeric(numeric: u16) -> Result<JsCurrency, JsValue> {
        Currency::try_from(numeric)
            .map(Self::from_inner)
            .map_err(|_| unknown_currency(&format!("numeric:{numeric}")))
    }

    /// Three-letter currency code (always upper-case).
    ///
    /// @type {string}
    /// @readonly
    ///
    /// @example
    /// ```javascript
    /// const usd = new Currency("usd");
    /// console.log(usd.code);  // "USD" (normalized to uppercase)
    /// ```
    #[wasm_bindgen(getter)]
    pub fn code(&self) -> String {
        self.inner.to_string()
    }

    /// ISO-4217 numeric currency code.
    ///
    /// @type {number}
    /// @readonly
    ///
    /// @example
    /// ```javascript
    /// const usd = new Currency("USD");
    /// console.log(usd.numeric);  // 840
    ///
    /// const eur = new Currency("EUR");
    /// console.log(eur.numeric);  // 978
    /// ```
    #[wasm_bindgen(getter)]
    pub fn numeric(&self) -> u16 {
        self.inner as u16
    }

    /// Number of decimal places (minor units) for the currency.
    ///
    /// @type {number}
    /// @readonly
    ///
    /// @example
    /// ```javascript
    /// const usd = new Currency("USD");
    /// console.log(usd.decimals);  // 2 (cents)
    ///
    /// const jpy = new Currency("JPY");
    /// console.log(jpy.decimals);  // 0 (no subdivision)
    /// ```
    #[wasm_bindgen(getter)]
    pub fn decimals(&self) -> u8 {
        self.inner.decimals()
    }

    /// Return this currency as an array of [code, numeric, decimals].
    ///
    /// @returns {Array} Tuple containing [string, number, number]
    ///
    /// @example
    /// ```javascript
    /// const usd = new Currency("USD");
    /// const [code, numeric, decimals] = usd.toTuple();
    /// console.log(code);      // "USD"
    /// console.log(numeric);   // 840
    /// console.log(decimals);  // 2
    /// ```
    #[wasm_bindgen(js_name = toTuple)]
    pub fn to_tuple(&self) -> js_sys::Array {
        let tuple = js_sys::Array::new();
        tuple.push(&JsValue::from(self.code()));
        tuple.push(&JsValue::from_f64(self.numeric() as f64));
        tuple.push(&JsValue::from_f64(self.decimals() as f64));
        tuple
    }

    /// List all ISO currencies compiled into the bindings.
    ///
    /// @returns {Array<Currency>} Array of all currencies recognized by finstack
    ///
    /// @example
    /// ```javascript
    /// const allCurrencies = Currency.all();
    /// console.log(allCurrencies.length);  // 157 (ISO-4217 currencies)
    ///
    /// // Find a specific currency
    /// const chf = allCurrencies.find(c => c.code === "CHF");
    /// ```
    #[wasm_bindgen(js_name = all)]
    pub fn all() -> js_sys::Array {
        let currencies = Currency::iter().map(JsCurrency::from_inner);
        js_array_from_iter(currencies.map(JsValue::from))
    }
}
