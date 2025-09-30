use crate::core::dates::date::JsDate;
use crate::core::utils::js_error;
use finstack_core::dates::utils as core_utils;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

#[wasm_bindgen(js_name = addMonths)]
pub fn add_months(date: &JsDate, months: i32) -> JsDate {
    JsDate::from_core(core_utils::add_months(date.inner(), months))
}

#[wasm_bindgen(js_name = lastDayOfMonth)]
pub fn last_day_of_month(date: &JsDate) -> JsDate {
    JsDate::from_core(core_utils::last_day_of_month(date.inner()))
}

#[wasm_bindgen(js_name = daysInMonth)]
pub fn days_in_month(year: i32, month: u8) -> Result<u8, JsValue> {
    if !(1..=12).contains(&month) {
        return Err(js_error(format!("Month out of range: {month}")));
    }
    Ok(core_utils::days_in_month(year, month))
}

#[wasm_bindgen(js_name = isLeapYear)]
pub fn is_leap_year(year: i32) -> bool {
    core_utils::is_leap_year(year)
}

#[wasm_bindgen(js_name = dateToDaysSinceEpoch)]
pub fn date_to_days_since_epoch(date: &JsDate) -> i32 {
    core_utils::date_to_days_since_epoch(date.inner())
}

#[wasm_bindgen(js_name = daysSinceEpochToDate)]
pub fn days_since_epoch_to_date(days: i32) -> JsDate {
    JsDate::from_core(core_utils::days_since_epoch_to_date(days))
}
