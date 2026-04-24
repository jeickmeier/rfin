//! WASM bindings for the `finstack-cashflows` crate.

use crate::utils::to_js_err;
use wasm_bindgen::prelude::*;

/// Build a cashflow schedule from JSON and return canonical schedule JSON.
#[wasm_bindgen(js_name = buildCashflowSchedule)]
pub fn build_cashflow_schedule(
    spec_json: &str,
    market_json: Option<String>,
) -> Result<String, JsValue> {
    finstack_cashflows::build_cashflow_schedule_json(spec_json, market_json.as_deref())
        .map_err(to_js_err)
}

/// Validate a cashflow schedule JSON string.
#[wasm_bindgen(js_name = validateCashflowSchedule)]
pub fn validate_cashflow_schedule(schedule_json: &str) -> Result<String, JsValue> {
    finstack_cashflows::validate_cashflow_schedule_json(schedule_json).map_err(to_js_err)
}

/// Extract dated flows from a cashflow schedule JSON string.
#[wasm_bindgen(js_name = datedFlows)]
pub fn dated_flows(schedule_json: &str) -> Result<String, JsValue> {
    finstack_cashflows::dated_flows_json(schedule_json).map_err(to_js_err)
}

/// Compute accrued interest from a cashflow schedule JSON string.
#[wasm_bindgen(js_name = accruedInterest)]
pub fn accrued_interest(
    schedule_json: &str,
    as_of: &str,
    config_json: Option<String>,
) -> Result<f64, JsValue> {
    finstack_cashflows::accrued_interest_json(schedule_json, as_of, config_json.as_deref())
        .map_err(to_js_err)
}

/// Create tagged Bond instrument JSON from a cashflow schedule JSON string.
#[wasm_bindgen(js_name = bondFromCashflows)]
pub fn bond_from_cashflows(
    instrument_id: &str,
    schedule_json: &str,
    discount_curve_id: &str,
    quoted_clean: Option<f64>,
) -> Result<String, JsValue> {
    let schedule: finstack_cashflows::builder::CashFlowSchedule =
        serde_json::from_str(schedule_json).map_err(to_js_err)?;
    let bond = finstack_valuations::instruments::fixed_income::bond::Bond::from_cashflows(
        instrument_id,
        schedule,
        discount_curve_id,
        quoted_clean,
    )
    .map_err(to_js_err)?;
    let instrument = finstack_valuations::instruments::InstrumentJson::Bond(bond);
    serde_json::to_string(&instrument).map_err(to_js_err)
}
