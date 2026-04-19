use std::str::FromStr;

use crate::utils::to_js_err;
use finstack_analytics as fa;
use wasm_bindgen::prelude::*;

pub(super) fn parse_iso_date(s: &str) -> Result<time::Date, JsValue> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 {
        return Err(to_js_err(format!("expected YYYY-MM-DD, got {s:?}")));
    }
    let year: i32 = parts[0].parse().map_err(to_js_err)?;
    let month_num: u8 = parts[1].parse().map_err(to_js_err)?;
    let day: u8 = parts[2].parse().map_err(to_js_err)?;
    let month = time::Month::try_from(month_num).map_err(to_js_err)?;
    time::Date::from_calendar_date(year, month, day).map_err(to_js_err)
}

pub(super) fn parse_iso_dates(date_strs: &[String]) -> Result<Vec<time::Date>, JsValue> {
    date_strs
        .iter()
        .map(|s| parse_iso_date(s))
        .collect::<Result<_, _>>()
}

pub(super) fn parse_cagr_convention(
    convention: Option<&str>,
) -> Result<fa::risk_metrics::AnnualizationConvention, JsValue> {
    fa::risk_metrics::AnnualizationConvention::from_str(convention.unwrap_or("act365_25"))
        .map_err(to_js_err)
}

pub(super) fn parse_dist(s: &str) -> Result<fa::timeseries::InnovationDist, JsValue> {
    fa::timeseries::InnovationDist::from_str(s).map_err(to_js_err)
}

pub(super) fn parse_var_method(s: &str) -> Result<fa::backtesting::VarMethod, JsValue> {
    fa::backtesting::VarMethod::from_str(s).map_err(to_js_err)
}
