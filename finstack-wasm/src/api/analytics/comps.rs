use crate::utils::to_js_err;
use finstack_analytics as fa;
use std::collections::BTreeMap;
use wasm_bindgen::prelude::*;

fn map_to_company_metrics(values: BTreeMap<String, f64>) -> fa::comps::CompanyMetrics {
    let mut metrics = fa::comps::CompanyMetrics::new("subject");
    for (name, value) in values {
        match name.as_str() {
            "enterprise_value" => metrics.enterprise_value = Some(value),
            "market_cap" => metrics.market_cap = Some(value),
            "share_price" => metrics.share_price = Some(value),
            "oas_bps" => metrics.oas_bps = Some(value),
            "yield_pct" => metrics.yield_pct = Some(value),
            "ebitda" => metrics.ebitda = Some(value),
            "revenue" => metrics.revenue = Some(value),
            "ebit" => metrics.ebit = Some(value),
            "ufcf" => metrics.ufcf = Some(value),
            "lfcf" => metrics.lfcf = Some(value),
            "net_income" => metrics.net_income = Some(value),
            "book_value" => metrics.book_value = Some(value),
            "tangible_book_value" => metrics.tangible_book_value = Some(value),
            "dividends_per_share" => metrics.dividends_per_share = Some(value),
            "leverage" => metrics.leverage = Some(value),
            "interest_coverage" => metrics.interest_coverage = Some(value),
            "revenue_growth" => metrics.revenue_growth = Some(value),
            "ebitda_margin" => metrics.ebitda_margin = Some(value),
            _ => {
                metrics.custom.insert(name, value);
            }
        }
    }
    metrics
}

#[wasm_bindgen(js_name = percentileRank)]
pub fn percentile_rank(value: f64, data: JsValue) -> Result<f64, JsValue> {
    let d: Vec<f64> = serde_wasm_bindgen::from_value(data).map_err(to_js_err)?;
    Ok(fa::comps::percentile_rank(&d, value).unwrap_or(0.5))
}

#[wasm_bindgen(js_name = zScore)]
pub fn z_score(value: f64, data: JsValue) -> Result<f64, JsValue> {
    let d: Vec<f64> = serde_wasm_bindgen::from_value(data).map_err(to_js_err)?;
    Ok(fa::comps::z_score(&d, value).unwrap_or(0.0))
}

#[wasm_bindgen(js_name = peerStats)]
pub fn peer_stats(data: JsValue) -> Result<JsValue, JsValue> {
    let d: Vec<f64> = serde_wasm_bindgen::from_value(data).map_err(to_js_err)?;
    let stats = fa::comps::peer_stats(&d);
    serde_wasm_bindgen::to_value(&stats).map_err(to_js_err)
}

#[wasm_bindgen(js_name = regressionFairValue)]
pub fn regression_fair_value(
    x_values: JsValue,
    y_values: JsValue,
    subject_x: f64,
    subject_y: f64,
) -> Result<JsValue, JsValue> {
    let x: Vec<f64> = serde_wasm_bindgen::from_value(x_values).map_err(to_js_err)?;
    let y: Vec<f64> = serde_wasm_bindgen::from_value(y_values).map_err(to_js_err)?;
    let result = fa::comps::regression_fair_value(&x, &y, subject_x, subject_y);
    serde_wasm_bindgen::to_value(&result).map_err(to_js_err)
}

#[wasm_bindgen(js_name = computeMultiple)]
pub fn compute_multiple(company_metrics: JsValue, multiple: &str) -> Result<JsValue, JsValue> {
    let metrics_map: BTreeMap<String, f64> =
        serde_wasm_bindgen::from_value(company_metrics).map_err(to_js_err)?;
    let metrics = map_to_company_metrics(metrics_map);
    let multiple = multiple.parse::<fa::comps::Multiple>().map_err(to_js_err)?;
    let result = fa::comps::compute_multiple(&metrics, multiple);
    serde_wasm_bindgen::to_value(&result).map_err(to_js_err)
}

#[wasm_bindgen(js_name = scoreRelativeValue)]
pub fn score_relative_value(peer_set: JsValue, dimensions: JsValue) -> Result<JsValue, JsValue> {
    let ps: fa::comps::PeerSet = serde_wasm_bindgen::from_value(peer_set).map_err(to_js_err)?;
    let dims: Vec<fa::comps::ScoringDimension> =
        serde_wasm_bindgen::from_value(dimensions).map_err(to_js_err)?;
    let result = fa::comps::score_relative_value(&ps, &dims).map_err(to_js_err)?;
    serde_wasm_bindgen::to_value(&result).map_err(to_js_err)
}
