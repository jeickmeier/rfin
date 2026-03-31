//! WASM bindings for lookback period selectors (MTD, QTD, YTD).
//!
//! These functions return index ranges into date/return arrays for
//! standard lookback periods.

use crate::core::dates::FsDate;
use crate::core::error::js_error;
use wasm_bindgen::prelude::*;

/// Month-to-date index range.
///
/// Returns `[start, end)` indices into a date array for the MTD window
/// ending at `refDate`.
///
/// @param {FsDate[]} dates - Sorted observation dates
/// @param {FsDate} refDate - Reference date (typically most recent business day)
/// @returns {number[]} Tuple of [start, end] indices
#[wasm_bindgen(js_name = mtdSelect)]
pub fn mtd_select(dates: Vec<FsDate>, ref_date: &FsDate) -> Result<Vec<usize>, JsValue> {
    let core_dates: Vec<finstack_core::dates::Date> = dates.iter().map(|d| d.inner()).collect();
    let range = finstack_analytics::lookback::mtd_select(&core_dates, ref_date.inner(), 0);
    Ok(vec![range.start, range.end])
}

/// Quarter-to-date index range.
///
/// Returns `[start, end)` indices into a date array for the QTD window
/// ending at `refDate`.
///
/// @param {FsDate[]} dates - Sorted observation dates
/// @param {FsDate} refDate - Reference date
/// @returns {number[]} Tuple of [start, end] indices
#[wasm_bindgen(js_name = qtdSelect)]
pub fn qtd_select(dates: Vec<FsDate>, ref_date: &FsDate) -> Result<Vec<usize>, JsValue> {
    let core_dates: Vec<finstack_core::dates::Date> = dates.iter().map(|d| d.inner()).collect();
    let range = finstack_analytics::lookback::qtd_select(&core_dates, ref_date.inner(), 0);
    Ok(vec![range.start, range.end])
}

/// Year-to-date index range.
///
/// Returns `[start, end)` indices into a date array for the YTD window
/// ending at `refDate`.
///
/// @param {FsDate[]} dates - Sorted observation dates
/// @param {FsDate} refDate - Reference date
/// @returns {number[]} Tuple of [start, end] indices
#[wasm_bindgen(js_name = ytdSelect)]
pub fn ytd_select(dates: Vec<FsDate>, ref_date: &FsDate) -> Result<Vec<usize>, JsValue> {
    let core_dates: Vec<finstack_core::dates::Date> = dates.iter().map(|d| d.inner()).collect();
    let range = finstack_analytics::lookback::ytd_select(&core_dates, ref_date.inner(), 0);
    Ok(vec![range.start, range.end])
}

/// Compounded lookback returns for standard periods (MTD, QTD, YTD).
///
/// Convenience function that computes compounded returns for each standard
/// lookback period from aligned date and return arrays.
///
/// @param {FsDate[]} dates - Sorted observation dates
/// @param {Float64Array} returns - Return series (aligned with dates)
/// @param {FsDate} refDate - Reference date
/// @returns {{ mtd: number, qtd: number, ytd: number }}
#[wasm_bindgen(js_name = lookbackReturns)]
pub fn lookback_returns(
    dates: Vec<FsDate>,
    returns: &[f64],
    ref_date: &FsDate,
) -> Result<JsValue, JsValue> {
    let core_dates: Vec<finstack_core::dates::Date> = dates.iter().map(|d| d.inner()).collect();
    let rd = ref_date.inner();

    let mtd_range = finstack_analytics::lookback::mtd_select(&core_dates, rd, 0);
    let qtd_range = finstack_analytics::lookback::qtd_select(&core_dates, rd, 0);
    let ytd_range = finstack_analytics::lookback::ytd_select(&core_dates, rd, 0);

    let mtd = finstack_analytics::returns::comp_total(
        &returns[mtd_range.start..mtd_range.end.min(returns.len())],
    );
    let qtd = finstack_analytics::returns::comp_total(
        &returns[qtd_range.start..qtd_range.end.min(returns.len())],
    );
    let ytd = finstack_analytics::returns::comp_total(
        &returns[ytd_range.start..ytd_range.end.min(returns.len())],
    );

    let obj = js_sys::Object::new();
    js_sys::Reflect::set(&obj, &"mtd".into(), &mtd.into())
        .map_err(|_| js_error("Failed to set mtd"))?;
    js_sys::Reflect::set(&obj, &"qtd".into(), &qtd.into())
        .map_err(|_| js_error("Failed to set qtd"))?;
    js_sys::Reflect::set(&obj, &"ytd".into(), &ytd.into())
        .map_err(|_| js_error("Failed to set ytd"))?;
    Ok(obj.into())
}
