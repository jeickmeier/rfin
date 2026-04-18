use crate::utils::to_js_err;
use finstack_analytics as fa;
use wasm_bindgen::prelude::*;

use super::support::{parse_iso_date, parse_iso_dates};

#[wasm_bindgen(js_name = mtdSelect)]
pub fn mtd_select(dates: JsValue, as_of: &str, offset_days: usize) -> Result<JsValue, JsValue> {
    let date_strs: Vec<String> = serde_wasm_bindgen::from_value(dates).map_err(to_js_err)?;
    let parsed_dates: Vec<finstack_core::dates::Date> = parse_iso_dates(&date_strs)?;
    let ref_date = parse_iso_date(as_of)?;
    let range = fa::lookback::mtd_select(&parsed_dates, ref_date, offset_days as i64);
    serde_wasm_bindgen::to_value(&[range.start, range.end]).map_err(to_js_err)
}

#[wasm_bindgen(js_name = qtdSelect)]
pub fn qtd_select(dates: JsValue, as_of: &str, offset_days: usize) -> Result<JsValue, JsValue> {
    let date_strs: Vec<String> = serde_wasm_bindgen::from_value(dates).map_err(to_js_err)?;
    let parsed_dates: Vec<finstack_core::dates::Date> = parse_iso_dates(&date_strs)?;
    let ref_date = parse_iso_date(as_of)?;
    let range = fa::lookback::qtd_select(&parsed_dates, ref_date, offset_days as i64);
    serde_wasm_bindgen::to_value(&[range.start, range.end]).map_err(to_js_err)
}

#[wasm_bindgen(js_name = ytdSelect)]
pub fn ytd_select(dates: JsValue, as_of: &str, offset_days: usize) -> Result<JsValue, JsValue> {
    let date_strs: Vec<String> = serde_wasm_bindgen::from_value(dates).map_err(to_js_err)?;
    let parsed_dates: Vec<finstack_core::dates::Date> = parse_iso_dates(&date_strs)?;
    let ref_date = parse_iso_date(as_of)?;
    let range = fa::lookback::ytd_select(&parsed_dates, ref_date, offset_days as i64);
    serde_wasm_bindgen::to_value(&[range.start, range.end]).map_err(to_js_err)
}

#[wasm_bindgen(js_name = fytdSelect)]
pub fn fytd_select(
    dates: JsValue,
    as_of: &str,
    offset_days: usize,
    fiscal_start_month: u8,
    fiscal_start_day: u8,
) -> Result<JsValue, JsValue> {
    let date_strs: Vec<String> = serde_wasm_bindgen::from_value(dates).map_err(to_js_err)?;
    let parsed_dates: Vec<finstack_core::dates::Date> = parse_iso_dates(&date_strs)?;
    let ref_date = parse_iso_date(as_of)?;
    let fiscal_config =
        finstack_core::dates::FiscalConfig::new(fiscal_start_month, fiscal_start_day)
            .map_err(to_js_err)?;
    let range =
        fa::lookback::fytd_select(&parsed_dates, ref_date, fiscal_config, offset_days as i64);
    serde_wasm_bindgen::to_value(&[range.start, range.end]).map_err(to_js_err)
}
