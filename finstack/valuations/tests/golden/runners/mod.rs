//! Per-domain golden runners.

use crate::golden::schema::GoldenFixture;
use std::collections::BTreeMap;

pub mod attribution_common;
pub mod calibration_common;
pub mod calibration_curves;
pub mod calibration_hazard;
pub mod calibration_inflation_curves;
pub mod calibration_swaption_vol;
pub mod calibration_vol_smile;
pub mod integration_common;
pub mod integration_credit;
pub mod integration_rates;
pub mod pricing_bond;
pub mod pricing_bond_future;
pub mod pricing_cap_floor;
pub mod pricing_cds;
pub mod pricing_cds_option;
pub mod pricing_cds_tranche;
pub mod pricing_common;
pub mod pricing_convertible;
pub mod pricing_deposit;
pub mod pricing_equity_index_future;
pub mod pricing_equity_option;
pub mod pricing_fra;
pub mod pricing_fx_option;
pub mod pricing_fx_swap;
pub mod pricing_inflation_linked_bond;
pub mod pricing_inflation_swap;
pub mod pricing_ir_future;
pub mod pricing_irs;
pub mod pricing_structured_credit;
pub mod pricing_swaption;
pub mod pricing_term_loan;

pub(crate) fn reject_flattened_outputs(
    runner: &str,
    fixture: &GoldenFixture,
) -> Result<BTreeMap<String, f64>, String> {
    let snapshot_hint = if fixture.inputs.get("actual_outputs").is_some() {
        " fixture contains inputs.actual_outputs, which is a frozen reference snapshot and not executable input."
    } else {
        ""
    };
    Err(format!(
        "{runner} requires executable inputs that build canonical API calls.{snapshot_hint} Replace the flattened placeholder with calibration/attribution inputs before enabling this golden."
    ))
}

pub(crate) fn validate_source_validation_fixture(
    runner: &str,
    fixture: &GoldenFixture,
) -> Result<bool, String> {
    let Some(source_validation) = fixture.inputs.get("source_validation") else {
        return Ok(false);
    };
    let status = source_validation
        .get("status")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| format!("{runner} source_validation must include a status"))?;
    if status != "non_executable" {
        return Err(format!(
            "{runner} source_validation status must be 'non_executable', got '{status}'"
        ));
    }
    let reason = source_validation
        .get("reason")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    if reason.trim().is_empty() {
        return Err(format!(
            "{runner} source_validation must explain why fixture is non-executable"
        ));
    }
    if fixture.inputs.get("actual_outputs").is_some() {
        return Err(format!(
            "{runner} source-validation fixture must not keep inputs.actual_outputs; expected values belong in top-level expected_outputs"
        ));
    }
    if source_validation.get("reference_outputs").is_some() {
        return Err(format!(
            "{runner} source_validation.reference_outputs is not allowed; expected values belong in top-level expected_outputs"
        ));
    }
    Ok(true)
}
