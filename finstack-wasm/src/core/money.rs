use crate::core::config::JsFinstackConfig;
use crate::core::currency::JsCurrency;
use crate::core::utils::js_error;
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
    #[wasm_bindgen(constructor)]
    pub fn new(amount: f64, currency: &JsCurrency) -> JsMoney {
        Self::from_inner(Money::new(amount, currency.inner()))
    }

    #[wasm_bindgen(js_name = zero)]
    pub fn zero(currency: &JsCurrency) -> JsMoney {
        Self::from_inner(Money::new(0.0, currency.inner()))
    }

    #[wasm_bindgen(getter)]
    pub fn amount(&self) -> f64 {
        self.inner.amount()
    }

    #[wasm_bindgen(getter)]
    pub fn currency(&self) -> JsCurrency {
        JsCurrency::from_inner(self.inner.currency())
    }

    #[wasm_bindgen(js_name = toTuple)]
    pub fn to_tuple(&self) -> Array {
        let tuple = Array::new();
        tuple.push(&JsValue::from_f64(self.amount()));
        tuple.push(&JsValue::from(JsCurrency::from_inner(
            self.inner.currency(),
        )));
        tuple
    }

    #[wasm_bindgen(js_name = fromTuple)]
    pub fn from_tuple(value: &JsValue) -> Result<JsMoney, JsValue> {
        money_from_tuple(value).map(Self::from_inner)
    }

    #[wasm_bindgen(js_name = fromConfig)]
    pub fn from_config(amount: f64, currency: &JsCurrency, config: &JsFinstackConfig) -> JsMoney {
        JsMoney::from_inner(Money::new_with_config(
            amount,
            currency.inner(),
            config.inner(),
        ))
    }

    #[wasm_bindgen(js_name = format)]
    pub fn format(&self) -> String {
        format!("{}", self.inner)
    }

    #[wasm_bindgen(js_name = fromCode)]
    pub fn from_code(amount: f64, code: &str) -> Result<JsMoney, JsValue> {
        let currency = Currency::from_str(code)
            .map_err(|_| js_error(format!("Unknown currency code: {code}")))?;
        Ok(Self::from_inner(Money::new(amount, currency)))
    }
}
