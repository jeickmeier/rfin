//! Unit tests for behavioral model specifications.
//!
//! Tests cover:
//! - Prepayment model spec calculations
//! - Default model spec calculations
//! - Recovery model spec calculations
//! - JSON serialization/deserialization
//! - Factory methods and convenience constructors

use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_valuations::instruments::structured_credit::{
    CreditFactors, DefaultModelSpec, MarketConditions, MarketFactors, PrepaymentModelSpec,
    RecoveryModelSpec,
};
use time::Month;

// ============================================================================
// Prepayment Model Spec Tests
// ============================================================================

#[test]
fn test_prepayment_spec_psa_100pct() {
    // Arrange
    let spec = PrepaymentModelSpec::Psa { multiplier: 1.0 };
    let market = MarketConditions::default();

    // Act: Month 30 (terminal)
    let smm = spec.prepayment_rate(
        Date::from_calendar_date(2025, Month::July, 1).unwrap(),
        Date::from_calendar_date(2023, Month::January, 1).unwrap(),
        30,
        &market,
    );

    // Assert: 100% PSA at month 30 = 6% CPR ≈ 0.514% SMM
    let expected_smm = 1.0 - (1.0 - 0.06_f64).powf(1.0 / 12.0);
    assert!((smm - expected_smm).abs() < 0.0001);
}

#[test]
fn test_prepayment_spec_psa_150pct() {
    // Arrange
    let spec = PrepaymentModelSpec::Psa { multiplier: 1.5 };
    let market = MarketConditions::default();

    // Act: Month 30
    let smm = spec.prepayment_rate(
        Date::from_calendar_date(2025, Month::July, 1).unwrap(),
        Date::from_calendar_date(2023, Month::January, 1).unwrap(),
        30,
        &market,
    );

    // Assert: 150% PSA = 9% CPR
    let expected_cpr = 0.09;
    let expected_smm = 1.0 - (1.0_f64 - expected_cpr).powf(1.0 / 12.0);
    assert!((smm - expected_smm).abs() < 0.001);
}

#[test]
fn test_prepayment_spec_constant_cpr() {
    // Arrange
    let spec = PrepaymentModelSpec::ConstantCpr { cpr: 0.12 };
    let market = MarketConditions::default();

    // Act
    let smm = spec.prepayment_rate(
        Date::from_calendar_date(2025, Month::January, 1).unwrap(),
        Date::from_calendar_date(2024, Month::January, 1).unwrap(),
        12,
        &market,
    );

    // Assert: Should convert 12% CPR to SMM
    let expected_smm = 1.0 - (1.0 - 0.12_f64).powf(1.0 / 12.0);
    assert!((smm - expected_smm).abs() < 0.0001);
}

#[test]
fn test_prepayment_spec_constant_smm() {
    // Arrange
    let spec = PrepaymentModelSpec::ConstantSmm { smm: 0.015 };
    let market = MarketConditions::default();

    // Act
    let smm = spec.prepayment_rate(
        Date::from_calendar_date(2025, Month::January, 1).unwrap(),
        Date::from_calendar_date(2024, Month::January, 1).unwrap(),
        12,
        &market,
    );

    // Assert: Should return SMM directly
    assert_eq!(smm, 0.015);
}

#[test]
fn test_prepayment_spec_factory_methods() {
    // Act
    let psa_100 = PrepaymentModelSpec::psa_100();
    let psa_150 = PrepaymentModelSpec::psa_150();
    let cpr_6pct = PrepaymentModelSpec::cpr_6pct();

    // Assert
    match psa_100 {
        PrepaymentModelSpec::Psa { multiplier } => assert_eq!(multiplier, 1.0),
        _ => panic!("Expected PSA 100"),
    }

    match psa_150 {
        PrepaymentModelSpec::Psa { multiplier } => assert_eq!(multiplier, 1.5),
        _ => panic!("Expected PSA 150"),
    }

    match cpr_6pct {
        PrepaymentModelSpec::ConstantCpr { cpr } => assert_eq!(cpr, 0.06),
        _ => panic!("Expected CPR 6%"),
    }
}

// ============================================================================
// Default Model Spec Tests
// ============================================================================

#[test]
fn test_default_spec_sda_100pct() {
    // Arrange
    let spec = DefaultModelSpec::Sda { multiplier: 1.0 };
    let factors = CreditFactors::default();

    // Act: At peak month (30)
    let mdr = spec.default_rate(
        Date::from_calendar_date(2025, Month::July, 1).unwrap(),
        Date::from_calendar_date(2023, Month::January, 1).unwrap(),
        30,
        &factors,
    );

    // Assert: Peak CDR = 0.6% → MDR
    let expected_cdr = 0.006;
    let expected_mdr = 1.0 - (1.0_f64 - expected_cdr).powf(1.0 / 12.0);
    assert!((mdr - expected_mdr).abs() < 0.0001);
}

#[test]
fn test_default_spec_constant_cdr() {
    // Arrange
    let spec = DefaultModelSpec::ConstantCdr { cdr: 0.02 };
    let factors = CreditFactors::default();

    // Act
    let mdr = spec.default_rate(
        Date::from_calendar_date(2025, Month::January, 1).unwrap(),
        Date::from_calendar_date(2024, Month::January, 1).unwrap(),
        12,
        &factors,
    );

    // Assert: 2% CDR → MDR
    let expected_mdr = 1.0 - (1.0 - 0.02_f64).powf(1.0 / 12.0);
    assert!((mdr - expected_mdr).abs() < 0.0001);
}

#[test]
fn test_default_spec_constant_mdr() {
    // Arrange
    let spec = DefaultModelSpec::ConstantMdr { mdr: 0.002 };
    let factors = CreditFactors::default();

    // Act
    let mdr = spec.default_rate(
        Date::from_calendar_date(2025, Month::January, 1).unwrap(),
        Date::from_calendar_date(2024, Month::January, 1).unwrap(),
        12,
        &factors,
    );

    // Assert: Should return MDR directly
    assert_eq!(mdr, 0.002);
}

#[test]
fn test_default_spec_factory_methods() {
    // Act
    let sda_100 = DefaultModelSpec::sda_100();
    let cdr_2pct = DefaultModelSpec::cdr_2pct();

    // Assert
    match sda_100 {
        DefaultModelSpec::Sda { multiplier } => assert_eq!(multiplier, 1.0),
        _ => panic!("Expected SDA 100"),
    }

    match cdr_2pct {
        DefaultModelSpec::ConstantCdr { cdr } => assert_eq!(cdr, 0.02),
        _ => panic!("Expected CDR 2%"),
    }
}

// ============================================================================
// Recovery Model Spec Tests
// ============================================================================

#[test]
fn test_recovery_spec_constant_rate() {
    // Arrange
    let spec = RecoveryModelSpec::Constant { rate: 0.40 };
    let market = MarketFactors::default();

    // Act
    let recovery = spec.recovery_rate(
        Date::from_calendar_date(2025, Month::January, 1).unwrap(),
        6,
        None,
        Money::new(100_000.0, Currency::USD),
        &market,
    );

    // Assert
    assert_eq!(recovery, 0.40);
}

#[test]
fn test_recovery_spec_factory_methods() {
    // Act
    let recovery_40 = RecoveryModelSpec::recovery_40pct();
    let recovery_70 = RecoveryModelSpec::recovery_70pct();

    // Assert
    match recovery_40 {
        RecoveryModelSpec::Constant { rate } => assert_eq!(rate, 0.4),
        _ => panic!("Expected 40% recovery"),
    }

    match recovery_70 {
        RecoveryModelSpec::Constant { rate } => assert_eq!(rate, 0.7),
        _ => panic!("Expected 70% recovery"),
    }
}

// ============================================================================
// Serialization Tests
// ============================================================================

#[cfg(feature = "serde")]
#[test]
fn test_prepayment_spec_serialization() {
    // Arrange
    let specs = vec![
        PrepaymentModelSpec::Psa { multiplier: 150.0 },
        PrepaymentModelSpec::ConstantCpr { cpr: 0.15 },
        PrepaymentModelSpec::ConstantSmm { smm: 0.012 },
        PrepaymentModelSpec::AssetDefault {
            asset_type: "auto".to_string(),
        },
    ];

    for spec in specs {
        // Act
        let json = serde_json::to_string(&spec).expect("Failed to serialize");
        let deserialized: PrepaymentModelSpec =
            serde_json::from_str(&json).expect("Failed to deserialize");

        // Assert
        assert_eq!(spec, deserialized);
    }
}

#[cfg(feature = "serde")]
#[test]
fn test_default_spec_serialization() {
    // Arrange
    let specs = vec![
        DefaultModelSpec::ConstantCdr { cdr: 0.02 },
        DefaultModelSpec::Sda { multiplier: 100.0 },
        DefaultModelSpec::AssetDefault {
            asset_type: "corporate".to_string(),
        },
    ];

    for spec in specs {
        // Act
        let json = serde_json::to_string(&spec).expect("Failed to serialize");
        let deserialized: DefaultModelSpec =
            serde_json::from_str(&json).expect("Failed to deserialize");

        // Assert
        assert_eq!(spec, deserialized);
    }
}

#[cfg(feature = "serde")]
#[test]
fn test_recovery_spec_serialization() {
    // Arrange
    let specs = vec![
        RecoveryModelSpec::Constant { rate: 0.70 },
        RecoveryModelSpec::AssetDefault {
            asset_type: "collateral".to_string(),
        },
    ];

    for spec in specs {
        // Act
        let json = serde_json::to_string(&spec).expect("Failed to serialize");
        let deserialized: RecoveryModelSpec =
            serde_json::from_str(&json).expect("Failed to deserialize");

        // Assert
        assert_eq!(spec, deserialized);
    }
}
