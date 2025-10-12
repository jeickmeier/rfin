//! Comprehensive tests for structured credit instrument JSON serialization.
//!
//! Verifies that ABS, RMBS, CMBS, and CLO instruments can be fully serialized
//! and deserialized from JSON, including custom behavioral models.

use finstack_valuations::instruments::structured_credit::{
    DefaultModelSpec, PrepaymentModelSpec, RecoveryModelSpec,
};

#[test]
fn test_all_prepayment_model_specs_serialize() {
    // Test all prepayment model variants
    let specs = vec![
        PrepaymentModelSpec::Psa { multiplier: 100.0 },
        PrepaymentModelSpec::ConstantCpr { cpr: 0.15 },
        PrepaymentModelSpec::ConstantSmm { smm: 0.012 },
        PrepaymentModelSpec::AssetDefault {
            asset_type: "auto".to_string(),
        },
    ];

    for spec in specs {
        let json = serde_json::to_string(&spec).expect("Failed to serialize spec");
        println!("Prepayment spec JSON: {}", json);
        let deserialized: PrepaymentModelSpec =
            serde_json::from_str(&json).expect("Failed to deserialize spec");
        assert_eq!(spec, deserialized);
    }
}

#[test]
fn test_all_default_model_specs_serialize() {
    let specs = vec![
        DefaultModelSpec::ConstantCdr { cdr: 0.02 },
        DefaultModelSpec::Sda { multiplier: 100.0 },
        DefaultModelSpec::AssetDefault {
            asset_type: "auto".to_string(),
        },
    ];

    for spec in specs {
        let json = serde_json::to_string(&spec).expect("Failed to serialize spec");
        println!("Default spec JSON: {}", json);
        let deserialized: DefaultModelSpec =
            serde_json::from_str(&json).expect("Failed to deserialize spec");
        assert_eq!(spec, deserialized);
    }
}

#[test]
fn test_all_recovery_model_specs_serialize() {
    let specs = vec![
        RecoveryModelSpec::Constant { rate: 0.70 },
        RecoveryModelSpec::AssetDefault {
            asset_type: "collateral".to_string(),
        },
    ];

    for spec in specs {
        let json = serde_json::to_string(&spec).expect("Failed to serialize spec");
        println!("Recovery spec JSON: {}", json);
        let deserialized: RecoveryModelSpec =
            serde_json::from_str(&json).expect("Failed to deserialize spec");
        assert_eq!(spec, deserialized);
    }
}

// NOTE: Roundtrip tests removed
// The from_arc() method has been removed as part of the simplification effort.
// Specs are now the source of truth and evaluated directly without trait object conversion.

#[test]
fn test_json_spec_examples() {
    // Test that we can parse realistic JSON specs

    // PSA prepayment model
    let json = r#"{"type":"psa","multiplier":150.0}"#;
    let spec: PrepaymentModelSpec = serde_json::from_str(json).unwrap();
    match spec {
        PrepaymentModelSpec::Psa { multiplier } => assert_eq!(multiplier, 150.0),
        _ => panic!("Expected PSA model"),
    }

    // Constant CDR default model
    let json = r#"{"type":"constant_cdr","cdr":0.02}"#;
    let spec: DefaultModelSpec = serde_json::from_str(json).unwrap();
    match spec {
        DefaultModelSpec::ConstantCdr { cdr } => assert_eq!(cdr, 0.02),
        _ => panic!("Expected ConstantCdr model"),
    }

    // Constant recovery model
    let json = r#"{"type":"constant","rate":0.70}"#;
    let spec: RecoveryModelSpec = serde_json::from_str(json).unwrap();
    match spec {
        RecoveryModelSpec::Constant { rate } => assert_eq!(rate, 0.70),
        _ => panic!("Expected Constant recovery model"),
    }

    println!("✅ All JSON spec examples parsed successfully!");
}
