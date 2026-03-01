//! Plan-driven calibration bindings for WASM.

use crate::core::market_data::context::JsMarketContext;
use crate::utils::json::from_js_value;
use crate::valuations::calibration::report::JsCalibrationReport;
use finstack_core::market_data::context::MarketContext;
use finstack_valuations::calibration::api::engine as calib_engine;
use finstack_valuations::calibration::api::schema::CalibrationEnvelope;
use js_sys::{Array, Object, Reflect};
use wasm_bindgen::prelude::*;

/// Execute a plan-driven calibration.
///
/// This is the canonical WASM entrypoint for calibration. It accepts a JS object
/// matching the calibration schema (`CalibrationEnvelope`) and returns a JS object
/// matching `CalibrationResultEnvelope` (final market snapshot + reports).
///
/// @param {any} envelope - JSON-like object conforming to the calibration schema
/// @returns {any} Tuple [MarketContext, CalibrationReport, Record<string, CalibrationReport>]
/// @throws {Error} If deserialization fails or calibration fails
#[wasm_bindgen(js_name = executeCalibration)]
pub fn execute_calibration(envelope: JsValue) -> Result<JsValue, JsValue> {
    let env: CalibrationEnvelope = from_js_value(envelope)?;
    let result = calib_engine::execute(&env).map_err(|e| JsValue::from_str(&e.to_string()))?;

    let market = MarketContext::try_from(result.result.final_market.clone())
        .map_err(|e| JsValue::from_str(&format!("Failed to build MarketContext: {e}")))?;
    let market_js = JsMarketContext::from_owned(market);
    let report_js = JsCalibrationReport::from_inner(result.result.report.clone());

    let step_reports = Object::new();
    for (k, v) in result.result.step_reports.iter() {
        Reflect::set(
            &step_reports,
            &JsValue::from_str(k),
            &JsCalibrationReport::from_inner(v.clone()).into(),
        )?;
    }

    let out = Array::new();
    out.push(&market_js.into());
    out.push(&report_js.into());
    out.push(&step_reports.into());
    Ok(out.into())
}
