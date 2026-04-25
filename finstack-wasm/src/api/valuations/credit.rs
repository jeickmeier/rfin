//! WASM bindings for structural credit model specifications.
//!
//! Mirrors the layout of `finstack-py/src/bindings/valuations/credit.rs` so the
//! Rust-canonical → PyO3 → wasm-bindgen triplet keeps file parity. The exported
//! JS surface (under `valuations.credit.*`) is unchanged — wasm-bindgen
//! exports are flat by `js_name`, so this is a pure source reorganisation.

use crate::utils::to_js_err;
use finstack_valuations::instruments::models::credit::{
    CreditState, CreditStateVariable, DynamicRecoverySpec, EndogenousHazardSpec, MertonModel,
    OptimalToggle, ThresholdDirection, ToggleExerciseModel,
};
use wasm_bindgen::prelude::*;

/// Build a structural Merton model JSON payload.
#[wasm_bindgen(js_name = mertonModelJson)]
pub fn merton_model_json(
    asset_value: f64,
    asset_vol: f64,
    debt_barrier: f64,
    risk_free_rate: f64,
) -> Result<String, JsValue> {
    let model = MertonModel::new(asset_value, asset_vol, debt_barrier, risk_free_rate)
        .map_err(to_js_err)?;
    serde_json::to_string(&model).map_err(to_js_err)
}

/// Build a CreditGrades structural model JSON payload.
#[wasm_bindgen(js_name = creditGradesModelJson)]
pub fn credit_grades_model_json(
    equity_value: f64,
    equity_vol: f64,
    total_debt: f64,
    risk_free_rate: f64,
    barrier_uncertainty: f64,
    mean_recovery: f64,
) -> Result<String, JsValue> {
    let model = MertonModel::credit_grades(
        equity_value,
        equity_vol,
        total_debt,
        risk_free_rate,
        barrier_uncertainty,
        mean_recovery,
    )
    .map_err(to_js_err)?;
    serde_json::to_string(&model).map_err(to_js_err)
}

/// Compute structural default probability from model JSON.
#[wasm_bindgen(js_name = mertonDefaultProbability)]
pub fn merton_default_probability(model_json: &str, horizon: f64) -> Result<f64, JsValue> {
    let model: MertonModel = serde_json::from_str(model_json).map_err(to_js_err)?;
    Ok(model.default_probability(horizon))
}

/// Build a constant dynamic-recovery spec JSON payload.
#[wasm_bindgen(js_name = dynamicRecoveryConstantJson)]
pub fn dynamic_recovery_constant_json(recovery: f64) -> Result<String, JsValue> {
    let spec = DynamicRecoverySpec::constant(recovery).map_err(to_js_err)?;
    serde_json::to_string(&spec).map_err(to_js_err)
}

/// Build an endogenous hazard power-law spec JSON payload.
#[wasm_bindgen(js_name = endogenousHazardPowerLawJson)]
pub fn endogenous_hazard_power_law_json(
    base_hazard: f64,
    base_leverage: f64,
    exponent: f64,
) -> Result<String, JsValue> {
    let spec =
        EndogenousHazardSpec::power_law(base_hazard, base_leverage, exponent).map_err(to_js_err)?;
    serde_json::to_string(&spec).map_err(to_js_err)
}

/// Build a credit-state JSON payload for toggle-exercise decisions.
#[wasm_bindgen(js_name = creditStateJson)]
pub fn credit_state_json(
    hazard_rate: f64,
    leverage: f64,
    accreted_notional: f64,
    coupon_due: f64,
    distance_to_default: Option<f64>,
    asset_value: Option<f64>,
) -> Result<String, JsValue> {
    let state = CreditState {
        hazard_rate,
        distance_to_default,
        leverage,
        accreted_notional,
        coupon_due,
        asset_value,
    };
    serde_json::to_string(&state).map_err(to_js_err)
}

/// Build a threshold toggle-exercise model JSON payload.
#[wasm_bindgen(js_name = toggleExerciseThresholdJson)]
pub fn toggle_exercise_threshold_json(
    variable: &str,
    threshold: f64,
    direction: &str,
) -> Result<String, JsValue> {
    let variable = parse_credit_state_variable(variable)?;
    let direction = parse_threshold_direction(direction)?;
    let model = ToggleExerciseModel::threshold(variable, threshold, direction);
    serde_json::to_string(&model).map_err(to_js_err)
}

/// Build an optimal toggle-exercise model JSON payload.
#[wasm_bindgen(js_name = toggleExerciseOptimalJson)]
pub fn toggle_exercise_optimal_json(
    nested_paths: usize,
    equity_discount_rate: f64,
    asset_vol: f64,
    risk_free_rate: f64,
    horizon: f64,
) -> Result<String, JsValue> {
    let model = ToggleExerciseModel::OptimalExercise(OptimalToggle {
        nested_paths,
        equity_discount_rate,
        asset_vol,
        risk_free_rate,
        horizon,
    });
    serde_json::to_string(&model).map_err(to_js_err)
}

fn parse_credit_state_variable(value: &str) -> Result<CreditStateVariable, JsValue> {
    match value {
        "hazard_rate" => Ok(CreditStateVariable::HazardRate),
        "distance_to_default" => Ok(CreditStateVariable::DistanceToDefault),
        "leverage" => Ok(CreditStateVariable::Leverage),
        other => Err(JsValue::from_str(&format!(
            "unknown credit state variable: {other}"
        ))),
    }
}

fn parse_threshold_direction(value: &str) -> Result<ThresholdDirection, JsValue> {
    match value {
        "above" => Ok(ThresholdDirection::Above),
        "below" => Ok(ThresholdDirection::Below),
        other => Err(JsValue::from_str(&format!(
            "unknown threshold direction: {other}"
        ))),
    }
}
