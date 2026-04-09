use crate::core::dates::date::JsDate;
use crate::core::error::js_error;
use finstack_core::dates::DateExt;
use time::Month;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

#[wasm_bindgen(js_name = addMonths)]
pub fn add_months(date: &JsDate, months: i32) -> JsDate {
    JsDate::from_core(date.inner().add_months(months))
}

#[wasm_bindgen(js_name = lastDayOfMonth)]
pub fn last_day_of_month(date: &JsDate) -> JsDate {
    JsDate::from_core(date.inner().end_of_month())
}

#[wasm_bindgen(js_name = daysInMonth)]
pub fn days_in_month(year: i32, month: u8) -> Result<u8, JsValue> {
    if !(1..=12).contains(&month) {
        return Err(js_error(format!("Month out of range: {month}")));
    }
    let m = Month::try_from(month).map_err(|e| js_error(format!("Invalid month {month}: {e}")))?;
    Ok(m.length(year))
}

#[wasm_bindgen(js_name = isLeapYear)]
pub fn is_leap_year(year: i32) -> bool {
    time::util::is_leap_year(year)
}

#[wasm_bindgen(js_name = dateToDaysSinceEpoch)]
pub fn date_to_days_since_epoch(date: &JsDate) -> i32 {
    finstack_core::dates::days_since_epoch(date.inner())
}

#[wasm_bindgen(js_name = daysSinceEpochToDate)]
pub fn days_since_epoch_to_date(days: i32) -> JsDate {
    let date = finstack_core::dates::date_from_epoch_days(days).unwrap_or(time::Date::MIN);
    JsDate::from_core(date)
}
