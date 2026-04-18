use crate::utils::to_js_err;
use fa::timeseries::GarchModel;
use finstack_analytics as fa;
use wasm_bindgen::prelude::*;

use super::support::parse_dist;

#[wasm_bindgen(js_name = fitGarch11)]
pub fn fit_garch11(returns: JsValue, distribution: &str) -> Result<JsValue, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    let dist = parse_dist(distribution)?;
    let fit = fa::timeseries::Garch11
        .fit(&r, dist, None)
        .map_err(to_js_err)?;
    serde_wasm_bindgen::to_value(&fit).map_err(to_js_err)
}

#[wasm_bindgen(js_name = fitEgarch11)]
pub fn fit_egarch11(returns: JsValue, distribution: &str) -> Result<JsValue, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    let dist = parse_dist(distribution)?;
    let fit = fa::timeseries::Egarch11
        .fit(&r, dist, None)
        .map_err(to_js_err)?;
    serde_wasm_bindgen::to_value(&fit).map_err(to_js_err)
}

#[wasm_bindgen(js_name = fitGjrGarch11)]
pub fn fit_gjr_garch11(returns: JsValue, distribution: &str) -> Result<JsValue, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    let dist = parse_dist(distribution)?;
    let fit = fa::timeseries::GjrGarch11
        .fit(&r, dist, None)
        .map_err(to_js_err)?;
    serde_wasm_bindgen::to_value(&fit).map_err(to_js_err)
}

#[wasm_bindgen(js_name = garch11Forecast)]
pub fn garch11_forecast(
    omega: f64,
    alpha: f64,
    beta: f64,
    last_variance: f64,
    last_return: f64,
    horizon: usize,
) -> Result<JsValue, JsValue> {
    if horizon == 0 {
        return serde_wasm_bindgen::to_value(&Vec::<f64>::new()).map_err(to_js_err);
    }
    let mut out = Vec::with_capacity(horizon);
    let mut s2 = omega + alpha * last_return * last_return + beta * last_variance;
    out.push(s2.max(0.0));
    let persistence = alpha + beta;
    for _ in 1..horizon {
        s2 = omega + persistence * s2;
        out.push(s2.max(0.0));
    }
    serde_wasm_bindgen::to_value(&out).map_err(to_js_err)
}

#[wasm_bindgen(js_name = ljungBox)]
pub fn ljung_box(residuals: JsValue, lags: usize) -> Result<JsValue, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(residuals).map_err(to_js_err)?;
    let result = fa::timeseries::ljung_box(&r, lags);
    serde_wasm_bindgen::to_value(&result).map_err(to_js_err)
}

#[wasm_bindgen(js_name = archLm)]
pub fn arch_lm(residuals: JsValue, lags: usize) -> Result<JsValue, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(residuals).map_err(to_js_err)?;
    let result = fa::timeseries::arch_lm(&r, lags);
    serde_wasm_bindgen::to_value(&result).map_err(to_js_err)
}

#[wasm_bindgen(js_name = aic)]
pub fn aic(log_likelihood: f64, n_params: usize) -> f64 {
    fa::timeseries::aic(log_likelihood, n_params)
}

#[wasm_bindgen(js_name = bic)]
pub fn bic(log_likelihood: f64, n_params: usize, n_obs: usize) -> f64 {
    fa::timeseries::bic(log_likelihood, n_params, n_obs)
}

#[wasm_bindgen(js_name = hqic)]
pub fn hqic(log_likelihood: f64, n_params: usize, n_obs: usize) -> f64 {
    fa::timeseries::hqic(log_likelihood, n_params, n_obs)
}
