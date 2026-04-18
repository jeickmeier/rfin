use crate::utils::to_js_err;
use finstack_analytics as fa;
use wasm_bindgen::prelude::*;

use super::support::parse_iso_dates;

#[wasm_bindgen(js_name = groupByPeriod)]
pub fn group_by_period(
    returns: JsValue,
    dates: JsValue,
    period_kind: &str,
) -> Result<JsValue, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    let date_strs: Vec<String> = serde_wasm_bindgen::from_value(dates).map_err(to_js_err)?;
    let parsed_dates: Vec<finstack_core::dates::Date> = parse_iso_dates(&date_strs)?;
    let freq: finstack_core::dates::PeriodKind = period_kind.parse().map_err(to_js_err)?;
    let grouped = fa::aggregation::group_by_period(&parsed_dates, &r, freq, None);
    serde_wasm_bindgen::to_value(&grouped).map_err(to_js_err)
}

#[wasm_bindgen(js_name = periodStats)]
pub fn period_stats(grouped: JsValue) -> Result<JsValue, JsValue> {
    let g: Vec<(finstack_core::dates::PeriodId, f64)> =
        serde_wasm_bindgen::from_value(grouped).map_err(to_js_err)?;
    let stats = fa::aggregation::period_stats(&g);
    serde_wasm_bindgen::to_value(&stats).map_err(to_js_err)
}
