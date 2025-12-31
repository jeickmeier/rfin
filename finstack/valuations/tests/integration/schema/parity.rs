//! Schema parity tests to ensure JSON schemas stay in sync with Rust types.
//!
//! These tests verify that the JSON schema files in `schemas/` accurately reflect
//! the serializable Rust types. When adding new enum variants or types, these tests
//! will fail if the corresponding schema updates are not made.
//!
//! To fix a failing test:
//! 1. Update the JSON schema file to match the Rust type
//! 2. Keep enum values alphabetically sorted in schemas for maintainability

use serde_json::Value;

// =============================================================================
// Attribution Schema Parity
// =============================================================================

/// Canonical list of attribution factors.
///
/// Must match `AttributionFactor` enum in `src/attribution/types.rs`
/// and the schema at `schemas/attribution/1/attribution.schema.json`.
const CANONICAL_ATTRIBUTION_FACTORS: &[&str] = &[
    "Carry",
    "Correlations",
    "CreditCurves",
    "Fx",
    "InflationCurves",
    "MarketScalars",
    "ModelParameters",
    "RatesCurves",
    "Volatility",
];

#[test]
fn test_attribution_factors_schema_parity() {
    let schema_json = include_str!("../../../schemas/attribution/1/attribution.schema.json");
    let schema: Value = serde_json::from_str(schema_json).expect("Schema JSON should be valid");

    // Extract the Waterfall factor enum from the schema
    // Path: properties.attribution.properties.method.oneOf[2].properties.Waterfall.items.enum
    let method_one_of = schema["properties"]["attribution"]["properties"]["method"]["oneOf"]
        .as_array()
        .expect("method should have oneOf array");

    // Find the Waterfall variant (object with Waterfall property)
    let waterfall_variant = method_one_of
        .iter()
        .find(|v| {
            v.get("properties")
                .and_then(|p| p.get("Waterfall"))
                .is_some()
        })
        .expect("Should have Waterfall variant");

    let mut schema_factors: Vec<&str> = waterfall_variant["properties"]["Waterfall"]["items"]
        ["enum"]
        .as_array()
        .expect("Waterfall.items.enum should be array")
        .iter()
        .map(|v| v.as_str().expect("Enum values should be strings"))
        .collect();

    // Sort both for comparison
    let mut expected: Vec<&str> = CANONICAL_ATTRIBUTION_FACTORS.to_vec();
    expected.sort();
    schema_factors.sort();

    // Find differences
    let missing_from_schema: Vec<&str> = expected
        .iter()
        .filter(|t| !schema_factors.contains(t))
        .copied()
        .collect();
    let extra_in_schema: Vec<&str> = schema_factors
        .iter()
        .filter(|t| !expected.contains(t))
        .copied()
        .collect();

    if !missing_from_schema.is_empty() || !extra_in_schema.is_empty() {
        let mut msg = String::from("attribution.schema.json is out of sync with Rust code!\n\n");
        if !missing_from_schema.is_empty() {
            msg.push_str(&format!(
                "Missing from schema (add these to Waterfall.items.enum):\n  {}\n\n",
                missing_from_schema.join(", ")
            ));
        }
        if !extra_in_schema.is_empty() {
            msg.push_str(&format!(
                "Extra in schema (remove or add to CANONICAL_ATTRIBUTION_FACTORS):\n  {}\n",
                extra_in_schema.join(", ")
            ));
        }
        panic!("{}", msg);
    }
}

// =============================================================================
// Calibration Schema Parity
// =============================================================================

/// Canonical list of calibration step kinds.
///
/// Must match `StepParams` enum variants in `src/calibration/api/schema.rs`
/// and the schema at `schemas/calibration/2/calibration.schema.json`.
const CANONICAL_CALIBRATION_STEP_KINDS: &[&str] = &[
    "base_correlation",
    "discount",
    "forward",
    "hazard",
    "inflation",
    "swaption_vol",
    "vol_surface",
];

#[test]
fn test_calibration_step_kinds_schema_parity() {
    let schema_json = include_str!("../../../schemas/calibration/2/calibration.schema.json");
    let schema: Value = serde_json::from_str(schema_json).expect("Schema JSON should be valid");

    // Extract CalibrationStep.kind enum from $defs
    let mut schema_kinds: Vec<&str> = schema["$defs"]["CalibrationStep"]["properties"]["kind"]["enum"]
        .as_array()
        .expect("CalibrationStep.kind.enum should be array")
        .iter()
        .map(|v| v.as_str().expect("Enum values should be strings"))
        .collect();

    // Sort both for comparison
    let mut expected: Vec<&str> = CANONICAL_CALIBRATION_STEP_KINDS.to_vec();
    expected.sort();
    schema_kinds.sort();

    // Find differences
    let missing_from_schema: Vec<&str> = expected
        .iter()
        .filter(|t| !schema_kinds.contains(t))
        .copied()
        .collect();
    let extra_in_schema: Vec<&str> = schema_kinds
        .iter()
        .filter(|t| !expected.contains(t))
        .copied()
        .collect();

    if !missing_from_schema.is_empty() || !extra_in_schema.is_empty() {
        let mut msg = String::from("calibration.schema.json is out of sync with Rust code!\n\n");
        if !missing_from_schema.is_empty() {
            msg.push_str(&format!(
                "Missing from schema (add these to CalibrationStep.kind.enum):\n  {}\n\n",
                missing_from_schema.join(", ")
            ));
        }
        if !extra_in_schema.is_empty() {
            msg.push_str(&format!(
                "Extra in schema (remove or add to CANONICAL_CALIBRATION_STEP_KINDS):\n  {}\n",
                extra_in_schema.join(", ")
            ));
        }
        panic!("{}", msg);
    }
}

// =============================================================================
// Cashflow Amortization Schema Parity
// =============================================================================

/// Canonical list of amortization spec variants.
///
/// Must match `AmortizationSpec` enum in `src/cashflow/builder/specs/amortization.rs`
/// and the schema at `schemas/cashflow/1/amortization_spec.schema.json`.
const CANONICAL_AMORTIZATION_VARIANTS: &[&str] = &[
    "CustomPrincipal",
    "LinearTo",
    "None",
    "PercentPerPeriod",
    "StepRemaining",
];

#[test]
fn test_amortization_spec_schema_parity() {
    let schema_json = include_str!("../../../schemas/cashflow/1/amortization_spec.schema.json");
    let schema: Value = serde_json::from_str(schema_json).expect("Schema JSON should be valid");

    // Extract AmortizationSpec variants from definitions.AmortizationSpec.oneOf
    let one_of = schema["definitions"]["AmortizationSpec"]["oneOf"]
        .as_array()
        .expect("AmortizationSpec.oneOf should be array");

    let mut schema_variants: Vec<&str> = Vec::new();
    for variant in one_of {
        // Check for string const (e.g., "None")
        if let Some(const_val) = variant.get("const").and_then(|v| v.as_str()) {
            schema_variants.push(const_val);
        }
        // Check for object with required key (e.g., {"LinearTo": {...}})
        else if let Some(required) = variant.get("required").and_then(|v| v.as_array()) {
            if let Some(first) = required.first().and_then(|v| v.as_str()) {
                schema_variants.push(first);
            }
        }
    }

    // Sort both for comparison
    let mut expected: Vec<&str> = CANONICAL_AMORTIZATION_VARIANTS.to_vec();
    expected.sort();
    schema_variants.sort();

    // Find differences
    let missing_from_schema: Vec<&str> = expected
        .iter()
        .filter(|t| !schema_variants.contains(t))
        .copied()
        .collect();
    let extra_in_schema: Vec<&str> = schema_variants
        .iter()
        .filter(|t| !expected.contains(t))
        .copied()
        .collect();

    if !missing_from_schema.is_empty() || !extra_in_schema.is_empty() {
        let mut msg =
            String::from("amortization_spec.schema.json is out of sync with Rust code!\n\n");
        if !missing_from_schema.is_empty() {
            msg.push_str(&format!(
                "Missing from schema (add these to AmortizationSpec.oneOf):\n  {}\n\n",
                missing_from_schema.join(", ")
            ));
        }
        if !extra_in_schema.is_empty() {
            msg.push_str(&format!(
                "Extra in schema (remove or add to CANONICAL_AMORTIZATION_VARIANTS):\n  {}\n",
                extra_in_schema.join(", ")
            ));
        }
        panic!("{}", msg);
    }
}

// =============================================================================
// Margin Schema Parity
// =============================================================================

/// Canonical list of IM methodologies.
///
/// Must match `ImMethodology` enum in `src/margin/types/enums.rs`
/// and the schema at `schemas/margin/1/margin.schema.json`.
const CANONICAL_IM_METHODOLOGIES: &[&str] = &[
    "ClearingHouse",
    "Haircut",
    "InternalModel",
    "Schedule",
    "Simm",
];

/// Canonical list of margin call types.
///
/// Must match `MarginCallType` enum in `src/margin/types/call.rs`
/// and the schema at `schemas/margin/1/margin.schema.json`.
const CANONICAL_MARGIN_CALL_TYPES: &[&str] = &[
    "InitialMargin",
    "Substitution",
    "TopUp",
    "VariationMarginDelivery",
    "VariationMarginReturn",
];

/// Canonical list of margin tenors.
///
/// Must match `MarginTenor` enum in `src/margin/types/enums.rs`
/// and the schema at `schemas/margin/1/margin.schema.json`.
const CANONICAL_MARGIN_TENORS: &[&str] = &["Daily", "Monthly", "OnDemand", "Weekly"];

#[test]
fn test_margin_im_methodology_schema_parity() {
    let schema_json = include_str!("../../../schemas/margin/1/margin.schema.json");
    let schema: Value = serde_json::from_str(schema_json).expect("Schema JSON should be valid");

    let mut schema_values: Vec<&str> = schema["definitions"]["ImMethodology"]["enum"]
        .as_array()
        .expect("ImMethodology.enum should be array")
        .iter()
        .map(|v| v.as_str().expect("Enum values should be strings"))
        .collect();

    let mut expected: Vec<&str> = CANONICAL_IM_METHODOLOGIES.to_vec();
    expected.sort();
    schema_values.sort();

    assert_eq!(
        expected, schema_values,
        "ImMethodology schema enum mismatch.\nExpected: {:?}\nActual: {:?}",
        expected, schema_values
    );
}

#[test]
fn test_margin_call_type_schema_parity() {
    let schema_json = include_str!("../../../schemas/margin/1/margin.schema.json");
    let schema: Value = serde_json::from_str(schema_json).expect("Schema JSON should be valid");

    let mut schema_values: Vec<&str> = schema["definitions"]["MarginCallType"]["enum"]
        .as_array()
        .expect("MarginCallType.enum should be array")
        .iter()
        .map(|v| v.as_str().expect("Enum values should be strings"))
        .collect();

    let mut expected: Vec<&str> = CANONICAL_MARGIN_CALL_TYPES.to_vec();
    expected.sort();
    schema_values.sort();

    assert_eq!(
        expected, schema_values,
        "MarginCallType schema enum mismatch.\nExpected: {:?}\nActual: {:?}",
        expected, schema_values
    );
}

#[test]
fn test_margin_tenor_schema_parity() {
    let schema_json = include_str!("../../../schemas/margin/1/margin.schema.json");
    let schema: Value = serde_json::from_str(schema_json).expect("Schema JSON should be valid");

    let mut schema_values: Vec<&str> = schema["definitions"]["MarginTenor"]["enum"]
        .as_array()
        .expect("MarginTenor.enum should be array")
        .iter()
        .map(|v| v.as_str().expect("Enum values should be strings"))
        .collect();

    let mut expected: Vec<&str> = CANONICAL_MARGIN_TENORS.to_vec();
    expected.sort();
    schema_values.sort();

    assert_eq!(
        expected, schema_values,
        "MarginTenor schema enum mismatch.\nExpected: {:?}\nActual: {:?}",
        expected, schema_values
    );
}
