use crate::core::common::parse::ParseFromString;
use crate::core::dates::date::JsDate;
use crate::core::error::core_to_js;
use finstack_core::dates::BusinessDayConvention;
use wasm_bindgen::prelude::*;

/// Adjust a date to be a business day on both base and quote currency calendars.
///
/// @param {FsDate} date - Date to adjust
/// @param {string} convention - Business day convention (e.g. "following")
/// @param {string | null} baseCalId - Base currency calendar ID (e.g. "nyse")
/// @param {string | null} quoteCalId - Quote currency calendar ID (e.g. "gblo")
/// @returns {FsDate} Adjusted date
#[wasm_bindgen(js_name = adjustJointCalendar)]
pub fn adjust_joint_calendar(
    date: &JsDate,
    convention: &str,
    base_cal_id: Option<String>,
    quote_cal_id: Option<String>,
) -> Result<JsDate, JsValue> {
    let bdc = BusinessDayConvention::parse_from_string(convention)?;
    let result = finstack_core::dates::fx::adjust_joint_calendar(
        date.inner(),
        bdc,
        base_cal_id.as_deref(),
        quote_cal_id.as_deref(),
    )
    .map_err(core_to_js)?;
    Ok(JsDate::from_core(result))
}

/// Add N joint business days (business days on both calendars).
///
/// @param {FsDate} start - Starting date
/// @param {number} nDays - Number of joint business days to add
/// @param {string} convention - Business day convention
/// @param {string | null} baseCalId - Base currency calendar ID
/// @param {string | null} quoteCalId - Quote currency calendar ID
/// @returns {FsDate} Date after N joint business days
#[wasm_bindgen(js_name = addJointBusinessDays)]
pub fn add_joint_business_days(
    start: &JsDate,
    n_days: u32,
    convention: &str,
    base_cal_id: Option<String>,
    quote_cal_id: Option<String>,
) -> Result<JsDate, JsValue> {
    let bdc = BusinessDayConvention::parse_from_string(convention)?;
    let result = finstack_core::dates::fx::add_joint_business_days(
        start.inner(),
        n_days,
        bdc,
        base_cal_id.as_deref(),
        quote_cal_id.as_deref(),
    )
    .map_err(core_to_js)?;
    Ok(JsDate::from_core(result))
}

/// Roll a trade date to spot using joint business day counting.
///
/// @param {FsDate} tradeDate - Trade execution date
/// @param {number} spotLagDays - Business days to spot (typically 2)
/// @param {string} convention - Business day convention
/// @param {string | null} baseCalId - Base currency calendar ID
/// @param {string | null} quoteCalId - Quote currency calendar ID
/// @returns {FsDate} Spot settlement date
#[wasm_bindgen(js_name = rollSpotDate)]
pub fn roll_spot_date(
    trade_date: &JsDate,
    spot_lag_days: u32,
    convention: &str,
    base_cal_id: Option<String>,
    quote_cal_id: Option<String>,
) -> Result<JsDate, JsValue> {
    let bdc = BusinessDayConvention::parse_from_string(convention)?;
    let result = finstack_core::dates::fx::roll_spot_date(
        trade_date.inner(),
        spot_lag_days,
        bdc,
        base_cal_id.as_deref(),
        quote_cal_id.as_deref(),
    )
    .map_err(core_to_js)?;
    Ok(JsDate::from_core(result))
}

/// Resolve a calendar ID, returning whether the resolution succeeds.
///
/// @param {string | null} calId - Calendar ID to resolve (null uses weekends-only)
/// @returns {boolean} True if the calendar can be resolved
#[wasm_bindgen(js_name = canResolveCalendar)]
pub fn can_resolve_calendar(cal_id: Option<String>) -> bool {
    finstack_core::dates::fx::resolve_calendar(cal_id.as_deref()).is_ok()
}
