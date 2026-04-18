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
    match convention
        .unwrap_or("act365_25")
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "act365_25" | "act36525" | "act/365.25" | "default" => {
            Ok(fa::risk_metrics::AnnualizationConvention::Act365_25)
        }
        "act365fixed" | "act365_fixed" | "act/365f" | "act365f" => {
            Ok(fa::risk_metrics::AnnualizationConvention::Act365Fixed)
        }
        "actact" | "act_act" | "actualactual" | "actual_actual" => {
            Ok(fa::risk_metrics::AnnualizationConvention::ActAct)
        }
        other => Err(to_js_err(format!(
            "unknown CAGR convention {other:?}; expected one of act365_25, act365_fixed, actact"
        ))),
    }
}

pub(super) fn parse_dist(s: &str) -> Result<fa::timeseries::InnovationDist, JsValue> {
    match s.to_ascii_lowercase().as_str() {
        "gaussian" | "normal" | "gauss" | "n" => Ok(fa::timeseries::InnovationDist::Gaussian),
        "student_t" | "student-t" | "studentt" | "t" => {
            Ok(fa::timeseries::InnovationDist::StudentT(8.0))
        }
        other => Err(to_js_err(format!(
            "unknown distribution '{other}'; expected 'gaussian' or 'student_t'"
        ))),
    }
}
