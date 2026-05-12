//! Shared helpers for the WASM `Performance`-centric analytics binding.

use std::str::FromStr;

pub(super) use crate::utils::parse_iso_date;
use crate::utils::to_js_err;
use finstack_analytics as fa;
use js_sys::{Array, Float64Array};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

pub(super) fn parse_cagr_convention(
    convention: Option<&str>,
) -> Result<fa::risk_metrics::AnnualizationConvention, JsValue> {
    fa::risk_metrics::AnnualizationConvention::from_str(convention.unwrap_or("act365_25"))
        .map_err(to_js_err)
}

pub(super) fn parse_f64_vec(value: JsValue) -> Result<Vec<f64>, JsValue> {
    if value.is_instance_of::<Float64Array>() {
        Ok(Float64Array::new(&value).to_vec())
    } else {
        serde_wasm_bindgen::from_value(value).map_err(to_js_err)
    }
}

pub(super) fn parse_f64_matrix(value: JsValue) -> Result<Vec<Vec<f64>>, JsValue> {
    if value.is_instance_of::<Array>() {
        let array = Array::from(&value);
        let mut rows = Vec::with_capacity(array.length() as usize);
        let mut all_typed = true;
        for row in array.iter() {
            if row.is_instance_of::<Float64Array>() {
                rows.push(Float64Array::new(&row).to_vec());
            } else {
                all_typed = false;
                break;
            }
        }
        if all_typed {
            return Ok(rows);
        }
    }
    serde_wasm_bindgen::from_value(value).map_err(to_js_err)
}
