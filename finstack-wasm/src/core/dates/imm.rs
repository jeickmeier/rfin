use crate::core::dates::date::JsDate;
use crate::core::error::js_error;
use finstack_core::dates::{
    imm_option_expiry, next_cds_date, next_equity_option_expiry, next_imm, next_imm_option_expiry,
    third_friday, third_wednesday,
};
use time::Month;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

#[wasm_bindgen(js_name = nextImm)]
pub fn next_imm_js(date: &JsDate) -> JsDate {
    JsDate::from_core(next_imm(date.inner()))
}

#[wasm_bindgen(js_name = nextCdsDate)]
pub fn next_cds_date_js(date: &JsDate) -> JsDate {
    JsDate::from_core(next_cds_date(date.inner()))
}

#[wasm_bindgen(js_name = nextImmOptionExpiry)]
pub fn next_imm_option_expiry_js(date: &JsDate) -> JsDate {
    JsDate::from_core(next_imm_option_expiry(date.inner()))
}

#[wasm_bindgen(js_name = immOptionExpiry)]
pub fn imm_option_expiry_js(year: i32, month: u8) -> Result<JsDate, JsValue> {
    let month_enum =
        Month::try_from(month).map_err(|_| js_error(format!("Month out of range: {month}")))?;
    Ok(JsDate::from_core(imm_option_expiry(month_enum, year)))
}

#[wasm_bindgen(js_name = nextEquityOptionExpiry)]
pub fn next_equity_option_expiry_js(date: &JsDate) -> JsDate {
    JsDate::from_core(next_equity_option_expiry(date.inner()))
}

#[wasm_bindgen(js_name = thirdFriday)]
pub fn third_friday_js(year: i32, month: u8) -> Result<JsDate, JsValue> {
    let month_enum =
        Month::try_from(month).map_err(|_| js_error(format!("Month out of range: {month}")))?;
    Ok(JsDate::from_core(third_friday(month_enum, year)))
}

#[wasm_bindgen(js_name = thirdWednesday)]
pub fn third_wednesday_js(year: i32, month: u8) -> Result<JsDate, JsValue> {
    let month_enum =
        Month::try_from(month).map_err(|_| js_error(format!("Month out of range: {month}")))?;
    Ok(JsDate::from_core(third_wednesday(month_enum, year)))
}
