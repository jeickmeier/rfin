use std::str::FromStr;

use crate::utils::to_js_err;
pub(super) use crate::utils::{parse_iso_date, parse_iso_dates};
use finstack_analytics as fa;
use wasm_bindgen::prelude::*;

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
