//! Unit tests for behavioral model specifications.
//!
//! Tests cover:
//! - Prepayment model spec calculations
//! - Default model spec calculations
//! - Recovery model spec calculations
//! - JSON serialization/deserialization
//!
//! Note: Behavioral model specs are now unified in the cashflow builder
//! and re-exported from structured_credit. Core functionality is tested
//! in the builder module tests.

use finstack_valuations::instruments::structured_credit::components::{
    DefaultModelSpec, PrepaymentModelSpec, RecoveryModelSpec,
};

// ============================================================================
// Prepayment Model Spec Tests
// ============================================================================

#[test]
fn test_prepayment_spec_psa() {
    let spec = PrepaymentModelSpec::psa(1.5);

    // Month 30: 150% PSA = 9% CPR
    let smm = spec.smm(30);
    let expected_smm = 1.0 - (1.0 - 0.09_f64).powf(1.0 / 12.0);
    assert!((smm - expected_smm).abs() < 0.001);
}

#[test]
fn test_prepayment_spec_constant_cpr() {
    let spec = PrepaymentModelSpec::constant_cpr(0.12);

    let smm = spec.smm(12);
    let expected_smm = 1.0 - (1.0 - 0.12_f64).powf(1.0 / 12.0);
    assert!((smm - expected_smm).abs() < 0.0001);
}

// ============================================================================
// Default Model Spec Tests
// ============================================================================

#[test]
fn test_default_spec_sda() {
    let spec = DefaultModelSpec::sda(2.0); // 200% SDA

    // Should have ramp up and decline pattern
    let month_10 = spec.mdr(10);
    let month_30 = spec.mdr(30); // Peak
    let month_70 = spec.mdr(70); // Terminal

    assert!(month_30 > month_10);
    assert!(month_30 > month_70);
}

#[test]
fn test_default_spec_constant_cdr() {
    let spec = DefaultModelSpec::constant_cdr(0.02);

    let mdr = spec.mdr(12);
    let expected_mdr = 1.0 - (1.0 - 0.02_f64).powf(1.0 / 12.0);
    assert!((mdr - expected_mdr).abs() < 0.0001);
}

// ============================================================================
// Recovery Model Spec Tests
// ============================================================================

#[test]
fn test_recovery_spec() {
    let spec = RecoveryModelSpec::with_lag(0.40, 12);

    assert_eq!(spec.rate, 0.40);
    assert_eq!(spec.recovery_lag, 12);
}

// ============================================================================
// JSON Serialization Tests
// ============================================================================

#[cfg(feature = "serde")]
#[test]
fn test_prepayment_spec_json_roundtrip() {
    let specs = vec![
        PrepaymentModelSpec::psa(150.0),
        PrepaymentModelSpec::constant_cpr(0.15),
    ];

    for spec in specs {
        let json = serde_json::to_string(&spec).unwrap();
        let deserialized: PrepaymentModelSpec = serde_json::from_str(&json).unwrap();
        assert_eq!(spec, deserialized);
    }
}

#[cfg(feature = "serde")]
#[test]
fn test_default_spec_json_roundtrip() {
    let specs = vec![
        DefaultModelSpec::constant_cdr(0.02),
        DefaultModelSpec::sda(100.0),
    ];

    for spec in specs {
        let json = serde_json::to_string(&spec).unwrap();
        let deserialized: DefaultModelSpec = serde_json::from_str(&json).unwrap();
        assert_eq!(spec, deserialized);
    }
}

#[cfg(feature = "serde")]
#[test]
fn test_recovery_spec_json_roundtrip() {
    let specs = vec![
        RecoveryModelSpec::with_lag(0.70, 12),
        RecoveryModelSpec::with_lag(0.40, 18),
    ];

    for spec in specs {
        let json = serde_json::to_string(&spec).unwrap();
        let deserialized: RecoveryModelSpec = serde_json::from_str(&json).unwrap();
        assert_eq!(spec, deserialized);
    }
}
