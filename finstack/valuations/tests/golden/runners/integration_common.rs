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
    let expected_outputs = source_validation_expected_outputs(&inputs)?;
    let nested = nested_fixture(fixture, inputs, expected_outputs);
    crate::golden::runners::calibration_common::run_curve_fixture(&nested)
}

fn run_nested_hazard(
    fixture: &GoldenFixture,
    inputs: serde_json::Value,
) -> Result<BTreeMap<String, f64>, String> {
    let expected_outputs = source_validation_expected_outputs(&inputs)?;
    let nested = nested_fixture(fixture, inputs, expected_outputs);
    crate::golden::runners::calibration_common::run_hazard_fixture(&nested)
}

fn run_nested_sabr(
    fixture: &GoldenFixture,
    inputs: serde_json::Value,
) -> Result<BTreeMap<String, f64>, String> {
    let expected_outputs = source_validation_expected_outputs(&inputs)?;
    let nested = nested_fixture(fixture, inputs, expected_outputs);
    crate::golden::runners::calibration_common::run_sabr_cube_fixture(&nested)
}

fn source_validation_expected_outputs(
    inputs: &serde_json::Value,
) -> Result<BTreeMap<String, f64>, String> {
    let Some(references) = inputs
        .get("source_validation")
        .and_then(|source_validation| source_validation.get("reference_outputs"))
    else {
        return Ok(BTreeMap::new());
    };
    let references = references
        .as_object()
        .ok_or("nested source_validation.reference_outputs must be an object")?;
    references
        .iter()
        .map(|(metric, value)| {
            value
                .as_f64()
                .map(|value| (metric.clone(), value))
                .ok_or_else(|| {
                    format!(
                        "nested source_validation.reference_outputs['{metric}'] must be numeric"
                    )
                })
        })
        .collect()
}

fn run_nested_pricing(
    fixture: &GoldenFixture,
    pricing_fixture: Option<String>,
    inputs: serde_json::Value,
    metric_map: BTreeMap<String, String>,
) -> Result<BTreeMap<String, f64>, String> {
    let inputs = if let Some(relative_path) = pricing_fixture {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let path = manifest_dir.join("tests/golden/data").join(relative_path);
        let raw = std::fs::read_to_string(&path)
            .map_err(|err| format!("read nested pricing fixture {path:?}: {err}"))?;
        let nested: GoldenFixture = serde_json::from_str(&raw)
            .map_err(|err| format!("parse nested pricing fixture {path:?}: {err}"))?;
        nested.inputs
    } else {
        inputs
    };
    let expected_outputs = metric_map
        .keys()
        .map(|metric| (metric.clone(), 0.0))
        .collect();
    let nested = nested_fixture(fixture, inputs, expected_outputs);
    let actuals = crate::golden::runners::pricing_common::run_pricing_fixture(&nested)?;
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
