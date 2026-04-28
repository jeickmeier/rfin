use crate::utils::to_js_err;
use finstack_analytics as fa;
use wasm_bindgen::prelude::*;

use super::support::parse_f64_vec;

/// Compute simple returns from a price series.
#[wasm_bindgen(js_name = simpleReturns)]
pub fn simple_returns(prices: JsValue) -> Result<JsValue, JsValue> {
    let p = parse_f64_vec(prices)?;
    serde_wasm_bindgen::to_value(&fa::returns::simple_returns(&p)).map_err(to_js_err)
}

/// Cumulative compounded returns.
#[wasm_bindgen(js_name = compSum)]
pub fn comp_sum(returns: JsValue) -> Result<JsValue, JsValue> {
    let r = parse_f64_vec(returns)?;
    serde_wasm_bindgen::to_value(&fa::returns::comp_sum(&r)).map_err(to_js_err)
}

/// Total compounded return.
#[wasm_bindgen(js_name = compTotal)]
pub fn comp_total(returns: JsValue) -> Result<f64, JsValue> {
    let r = parse_f64_vec(returns)?;
    Ok(fa::returns::comp_total(&r))
}

/// Replace NaN and infinite returns with zero.
#[wasm_bindgen(js_name = cleanReturns)]
pub fn clean_returns(returns: JsValue) -> Result<JsValue, JsValue> {
    let mut r = parse_f64_vec(returns)?;
    fa::returns::clean_returns(&mut r);
    serde_wasm_bindgen::to_value(&r).map_err(to_js_err)
}

/// Convert simple returns back to a price path starting at `base`.
#[wasm_bindgen(js_name = convertToPrices)]
pub fn convert_to_prices(returns: JsValue, base: f64) -> Result<JsValue, JsValue> {
    let r = parse_f64_vec(returns)?;
    serde_wasm_bindgen::to_value(&fa::returns::convert_to_prices(&r, base)).map_err(to_js_err)
}

/// Rebase a price series so its first value equals `base`.
#[wasm_bindgen(js_name = rebase)]
pub fn rebase(prices: JsValue, base: f64) -> Result<JsValue, JsValue> {
    let p = parse_f64_vec(prices)?;
    serde_wasm_bindgen::to_value(&fa::returns::rebase(&p, base)).map_err(to_js_err)
}

/// Excess returns over a risk-free rate series.
#[wasm_bindgen(js_name = excessReturns)]
pub fn excess_returns(
    returns: JsValue,
    rf: JsValue,
    nperiods: Option<f64>,
) -> Result<JsValue, JsValue> {
    let r = parse_f64_vec(returns)?;
    let rf_vec = parse_f64_vec(rf)?;
    serde_wasm_bindgen::to_value(&fa::returns::excess_returns(&r, &rf_vec, nperiods))
        .map_err(to_js_err)
}
