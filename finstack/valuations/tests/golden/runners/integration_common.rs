//! Shared executable integration golden helpers.

use crate::golden::schema::GoldenFixture;
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Deserialize)]
struct IntegrationInputs {
    #[serde(default)]
    calibration: Option<serde_json::Value>,
    #[serde(default)]
    hazard_calibration: Option<serde_json::Value>,
    #[serde(default)]
    sabr_calibration: Option<serde_json::Value>,
    #[serde(default)]
    pricing_fixture: Option<String>,
    #[serde(default)]
    pricing: serde_json::Value,
    pricing_metrics: BTreeMap<String, String>,
}

pub(crate) fn run_rates_integration(
    fixture: &GoldenFixture,
) -> Result<BTreeMap<String, f64>, String> {
    let inputs: IntegrationInputs = serde_json::from_value(fixture.inputs.clone())
        .map_err(|err| format!("parse rates integration inputs: {err}"))?;
    let mut actuals = BTreeMap::new();
    if let Some(calibration) = inputs.calibration {
        actuals.extend(run_nested_calibration(fixture, calibration)?);
    }
    if let Some(sabr) = inputs.sabr_calibration {
        actuals.extend(run_nested_sabr(fixture, sabr)?);
    }
    actuals.extend(run_nested_pricing(
        fixture,
        inputs.pricing_fixture,
        inputs.pricing,
        inputs.pricing_metrics,
    )?);
    Ok(actuals)
}

pub(crate) fn run_credit_integration(
    fixture: &GoldenFixture,
) -> Result<BTreeMap<String, f64>, String> {
    let inputs: IntegrationInputs = serde_json::from_value(fixture.inputs.clone())
        .map_err(|err| format!("parse credit integration inputs: {err}"))?;
    let mut actuals = BTreeMap::new();
    if let Some(hazard) = inputs.hazard_calibration {
        actuals.extend(run_nested_hazard(fixture, hazard)?);
    }
    actuals.extend(run_nested_pricing(
        fixture,
        inputs.pricing_fixture,
        inputs.pricing,
        inputs.pricing_metrics,
    )?);
    Ok(actuals)
}

fn run_nested_calibration(
    fixture: &GoldenFixture,
    inputs: serde_json::Value,
) -> Result<BTreeMap<String, f64>, String> {
    reject_nested_source_validation("curve calibration runner", &inputs)?;
    let nested = nested_fixture(fixture, inputs, BTreeMap::new());
    crate::golden::runners::calibration_common::run_curve_fixture(&nested)
}

fn run_nested_hazard(
    fixture: &GoldenFixture,
    inputs: serde_json::Value,
) -> Result<BTreeMap<String, f64>, String> {
    reject_nested_source_validation("hazard calibration runner", &inputs)?;
    let nested = nested_fixture(fixture, inputs, BTreeMap::new());
    crate::golden::runners::calibration_common::run_hazard_fixture(&nested)
}

fn run_nested_sabr(
    fixture: &GoldenFixture,
    inputs: serde_json::Value,
) -> Result<BTreeMap<String, f64>, String> {
    reject_nested_source_validation("SABR calibration runner", &inputs)?;
    let nested = nested_fixture(fixture, inputs, BTreeMap::new());
    crate::golden::runners::calibration_common::run_sabr_cube_fixture(&nested)
}

fn reject_nested_source_validation(runner: &str, inputs: &serde_json::Value) -> Result<(), String> {
    if inputs.get("source_validation").is_some() {
        return Err(format!(
            "{runner} requires executable inputs; nested source_validation metadata cannot provide actuals"
        ));
    }
    Ok(())
}

fn run_nested_pricing(
    fixture: &GoldenFixture,
    pricing_fixture: Option<String>,
    inputs: serde_json::Value,
    metric_map: BTreeMap<String, String>,
) -> Result<BTreeMap<String, f64>, String> {
    if let Some(relative_path) = pricing_fixture {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let path = manifest_dir.join("tests/golden/data").join(relative_path);
        let raw = std::fs::read_to_string(&path)
            .map_err(|err| format!("read nested pricing fixture {path:?}: {err}"))?;
        let nested: GoldenFixture = serde_json::from_str(&raw)
            .map_err(|err| format!("parse nested pricing fixture {path:?}: {err}"))?;
        let actuals = crate::golden::runners::pricing_common::run_pricing_fixture(&nested)?;
        return remap_pricing_actuals(actuals, metric_map);
    }
    let expected_outputs = metric_map
        .keys()
        .map(|metric| (metric.clone(), 0.0))
        .collect();
    let nested = nested_fixture(fixture, inputs, expected_outputs);
    let actuals = crate::golden::runners::pricing_common::run_pricing_fixture(&nested)?;
    remap_pricing_actuals(actuals, metric_map)
}

fn remap_pricing_actuals(
    actuals: BTreeMap<String, f64>,
    metric_map: BTreeMap<String, String>,
) -> Result<BTreeMap<String, f64>, String> {
    metric_map
        .into_iter()
        .map(|(metric, output)| {
            actuals
                .get(&metric)
                .copied()
                .map(|value| (output, value))
                .ok_or_else(|| format!("pricing did not produce metric '{metric}'"))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()
}

fn nested_fixture(
    fixture: &GoldenFixture,
    inputs: serde_json::Value,
    expected_outputs: BTreeMap<String, f64>,
) -> GoldenFixture {
    GoldenFixture {
        schema_version: fixture.schema_version.clone(),
        name: fixture.name.clone(),
        domain: fixture.domain.clone(),
        description: fixture.description.clone(),
        provenance: fixture.provenance.clone(),
        inputs,
        expected_outputs,
        tolerances: BTreeMap::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_source_validation_fixture_requires_executable_inputs() {
        let fixture = GoldenFixture {
            schema_version: crate::golden::schema::SCHEMA_VERSION.to_string(),
            name: "nested_source_validation".to_string(),
            domain: "rates.integration".to_string(),
            description: "Nested source validation test".to_string(),
            provenance: crate::golden::schema::Provenance {
                as_of: "2026-04-30".to_string(),
                source: "formula".to_string(),
                source_detail: "unit test".to_string(),
                captured_by: "test".to_string(),
                captured_on: "2026-04-30".to_string(),
                last_reviewed_by: "test".to_string(),
                last_reviewed_on: "2026-04-30".to_string(),
                review_interval_months: 6,
                regen_command: String::new(),
                screenshots: Vec::new(),
            },
            inputs: serde_json::json!({
                "calibration": {
                    "source_validation": {
                        "status": "non_executable",
                        "reason": "unit test"
                    }
                },
                "pricing_metrics": {}
            }),
            expected_outputs: BTreeMap::new(),
            tolerances: BTreeMap::new(),
        };

        let err = run_rates_integration(&fixture)
            .expect_err("nested source_validation must not provide actuals");

        assert!(
            err.contains("requires executable inputs"),
            "unexpected error: {err}"
        );
    }
}
