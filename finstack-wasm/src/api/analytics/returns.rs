use crate::utils::to_js_err;
use finstack_analytics as fa;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = simpleReturns)]
pub fn simple_returns(prices: JsValue) -> Result<JsValue, JsValue> {
    let p: Vec<f64> = serde_wasm_bindgen::from_value(prices).map_err(to_js_err)?;
    serde_wasm_bindgen::to_value(&fa::returns::simple_returns(&p)).map_err(to_js_err)
}

#[wasm_bindgen(js_name = compSum)]
pub fn comp_sum(returns: JsValue) -> Result<JsValue, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    serde_wasm_bindgen::to_value(&fa::returns::comp_sum(&r)).map_err(to_js_err)
}

#[wasm_bindgen(js_name = compTotal)]
pub fn comp_total(returns: JsValue) -> Result<f64, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    Ok(fa::returns::comp_total(&r))
}

#[wasm_bindgen(js_name = cleanReturns)]
pub fn clean_returns(returns: JsValue) -> Result<JsValue, JsValue> {
    let mut r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    fa::returns::clean_returns(&mut r);
    serde_wasm_bindgen::to_value(&r).map_err(to_js_err)
}

#[wasm_bindgen(js_name = convertToPrices)]
pub fn convert_to_prices(returns: JsValue, base: f64) -> Result<JsValue, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    serde_wasm_bindgen::to_value(&fa::returns::convert_to_prices(&r, base)).map_err(to_js_err)
}

#[wasm_bindgen(js_name = rebase)]
pub fn rebase(prices: JsValue, base: f64) -> Result<JsValue, JsValue> {
    let p: Vec<f64> = serde_wasm_bindgen::from_value(prices).map_err(to_js_err)?;
    serde_wasm_bindgen::to_value(&fa::returns::rebase(&p, base)).map_err(to_js_err)
}

#[wasm_bindgen(js_name = excessReturns)]
pub fn excess_returns(returns: JsValue, rf: JsValue) -> Result<JsValue, JsValue> {
    let r: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    let rf_vec: Vec<f64> = serde_wasm_bindgen::from_value(rf).map_err(to_js_err)?;
    serde_wasm_bindgen::to_value(&fa::returns::excess_returns(&r, &rf_vec, None)).map_err(to_js_err)
}
