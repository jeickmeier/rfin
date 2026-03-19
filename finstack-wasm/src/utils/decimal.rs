use crate::core::error::js_error;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use wasm_bindgen::JsValue;

pub(crate) fn decimal_from_f64(value: f64, field_name: &str) -> Result<Decimal, JsValue> {
    Decimal::from_f64_retain(value).ok_or_else(|| {
        js_error(format!(
            "{field_name}: cannot convert {value} to Decimal (NaN or Infinity)"
        ))
    })
}

pub(crate) fn decimal_to_f64(value: &Decimal, field_name: &str) -> Result<f64, JsValue> {
    value.to_f64().ok_or_else(|| {
        js_error(format!(
            "{field_name}: cannot convert Decimal {value} to f64"
        ))
    })
}

pub(crate) fn decimal_to_f64_or_warn(value: &Decimal, field_name: &str) -> f64 {
    match decimal_to_f64(value, field_name) {
        Ok(number) => number,
        Err(err) => {
            web_sys::console::warn_1(&err);
            0.0
        }
    }
}
