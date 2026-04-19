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

#[wasm_bindgen(js_name = forecastGarchFit)]
pub fn forecast_garch_fit(
    fit: JsValue,
    horizons: JsValue,
    trading_days_per_year: f64,
    terminal_residual: Option<f64>,
) -> Result<JsValue, JsValue> {
    let fit: fa::timeseries::GarchFit = serde_wasm_bindgen::from_value(fit).map_err(to_js_err)?;
    let horizons: Vec<usize> = serde_wasm_bindgen::from_value(horizons).map_err(to_js_err)?;
    let forecasts = fa::timeseries::forecast_garch_fit(
        &fit,
        &horizons,
        trading_days_per_year,
        terminal_residual,
    );
    serde_wasm_bindgen::to_value(&forecasts).map_err(to_js_err)
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
