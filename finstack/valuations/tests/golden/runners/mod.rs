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
) -> Result<Option<BTreeMap<String, f64>>, String> {
    let Some(source_validation) = fixture.inputs.get("source_validation") else {
        return Ok(None);
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
            "{runner} source-validation fixture must not keep inputs.actual_outputs; move frozen references under source_validation.reference_outputs"
        ));
    }
    let references = source_validation
        .get("reference_outputs")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| {
            format!(
                "{runner} source_validation must retain frozen references under reference_outputs"
            )
        })?;
    let mut reference_outputs = BTreeMap::new();
    for (metric, expected) in &fixture.expected_outputs {
        let reference = references.get(metric).ok_or_else(|| {
            format!(
                "{runner} source_validation.reference_outputs missing expected metric '{metric}'"
            )
        })?;
        let reference = reference.as_f64().ok_or_else(|| {
            format!("{runner} source_validation.reference_outputs['{metric}'] must be numeric")
        })?;
        if reference != *expected {
            return Err(format!(
                "{runner} source_validation.reference_outputs['{metric}']={reference:.17} does not exactly match expected_outputs['{metric}']={expected:.17}"
            ));
        }
        reference_outputs.insert(metric.clone(), reference);
    }
    for metric in references.keys() {
        if !fixture.expected_outputs.contains_key(metric) {
            return Err(format!(
                "{runner} source_validation.reference_outputs contains extra metric '{metric}'"
            ));
        }
    }
    Ok(Some(reference_outputs))
}
