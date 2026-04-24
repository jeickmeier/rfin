//! ISO-8601 date parsing/formatting helpers shared by WASM bindings.
//!
//! Centralizing these avoids duplicate `YYYY-MM-DD` parsers across the
//! `core/market_data`, `analytics`, and other domain modules.

use time::Date;
use wasm_bindgen::JsValue;

use super::to_js_err;

/// Parse an ISO date string (`"YYYY-MM-DD"`) into a [`time::Date`].
pub fn parse_iso_date(s: &str) -> Result<Date, JsValue> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 {
        return Err(to_js_err(format!("expected YYYY-MM-DD, got {s:?}")));
    }
    let year: i32 = parts[0].parse().map_err(to_js_err)?;
    let month_num: u8 = parts[1].parse().map_err(to_js_err)?;
    let day: u8 = parts[2].parse().map_err(to_js_err)?;
    let month = time::Month::try_from(month_num).map_err(to_js_err)?;
    Date::from_calendar_date(year, month, day).map_err(to_js_err)
}

/// Format a [`time::Date`] as `"YYYY-MM-DD"`.
pub fn date_to_iso(d: Date) -> String {
    format!("{:04}-{:02}-{:02}", d.year(), d.month() as u8, d.day())
}

/// Parse a slice of ISO date strings.
pub fn parse_iso_dates(date_strs: &[String]) -> Result<Vec<Date>, JsValue> {
    date_strs
        .iter()
        .map(|s| parse_iso_date(s))
        .collect::<Result<_, _>>()
}
