//! Per-domain golden runners.

use crate::golden::schema::GoldenFixture;

pub mod pricing_common;

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
