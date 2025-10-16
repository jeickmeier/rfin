//! Integration tests for JSON serialization and wire format stability.
//!
//! Tests that all structured credit types serialize/deserialize correctly
//! and maintain wire format compatibility.

use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_valuations::instruments::structured_credit::{
    AssetPool, DealType, DefaultModelSpec, PrepaymentModelSpec, RecoveryModelSpec,
    StructuredCredit, Tranche, TrancheCoupon, TrancheSeniority, TrancheStructure,
    WaterfallEngine,
};
use time::Month;

fn maturity_date() -> Date {
    Date::from_calendar_date(2030, Month::January, 1).unwrap()
}

// ============================================================================
// Model Spec Serialization Tests
// ============================================================================

#[test]
fn test_prepayment_spec_all_variants_serialize() {
    // Arrange
    let specs = vec![
        PrepaymentModelSpec::Psa { multiplier: 100.0 },
        PrepaymentModelSpec::ConstantCpr { cpr: 0.15 },
        PrepaymentModelSpec::ConstantSmm { smm: 0.012 },
        PrepaymentModelSpec::AssetDefault {
            asset_type: "auto".to_string(),
        },
    ];

    for spec in specs {
        // Act
        let json = serde_json::to_string(&spec).expect("Serialization failed");
        let deserialized: PrepaymentModelSpec =
            serde_json::from_str(&json).expect("Deserialization failed");

        // Assert
        assert_eq!(spec, deserialized, "Roundtrip failed for {:?}", spec);
    }
}

#[test]
fn test_default_spec_all_variants_serialize() {
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
        let json = serde_json::to_string(&spec).expect("Serialization failed");
        let deserialized: DefaultModelSpec =
            serde_json::from_str(&json).expect("Deserialization failed");

        // Assert
        assert_eq!(spec, deserialized, "Roundtrip failed for {:?}", spec);
    }
}

#[test]
fn test_recovery_spec_all_variants_serialize() {
    // Arrange
    let specs = vec![
        RecoveryModelSpec::Constant { rate: 0.70 },
        RecoveryModelSpec::AssetDefault {
            asset_type: "collateral".to_string(),
        },
    ];

    for spec in specs {
        // Act
        let json = serde_json::to_string(&spec).expect("Serialization failed");
        let deserialized: RecoveryModelSpec =
            serde_json::from_str(&json).expect("Deserialization failed");

        // Assert
        assert_eq!(spec, deserialized, "Roundtrip failed for {:?}", spec);
    }
}

// ============================================================================
// Full Instrument Serialization Tests
// ============================================================================

#[cfg(feature = "serde")]
#[test]
fn test_clo_json_roundtrip() {
    // Arrange
    let pool = AssetPool::new("TEST_POOL", DealType::CLO, Currency::USD);

    let tranche = Tranche::new(
        "AAA",
        0.0,
        100.0,
        TrancheSeniority::Senior,
        Money::new(10_000_000.0, Currency::USD),
        TrancheCoupon::Fixed { rate: 0.05 },
        maturity_date(),
    )
    .unwrap();

    let tranches = TrancheStructure::new(vec![tranche]).unwrap();
    let waterfall = WaterfallEngine::new(Currency::USD);

    let original = StructuredCredit::new_clo(
        "TEST_CLO",
        pool,
        tranches,
        waterfall,
        Date::from_calendar_date(2024, Month::January, 1).unwrap(),
        maturity_date(),
        "USD-OIS",
    );

    // Act
    let json = serde_json::to_string(&original).expect("Serialization failed");
    let deserialized: StructuredCredit =
        serde_json::from_str(&json).expect("Deserialization failed");

    // Assert
    assert_eq!(original.id.as_str(), deserialized.id.as_str());
    assert_eq!(original.deal_type, deserialized.deal_type);
    assert_eq!(original.prepayment_spec, deserialized.prepayment_spec);
    assert_eq!(original.default_spec, deserialized.default_spec);
}

#[cfg(feature = "serde")]
#[test]
fn test_rmbs_with_overrides_serialization() {
    // Arrange
    let pool = AssetPool::new("TEST_POOL", DealType::RMBS, Currency::USD);

    let tranche = Tranche::new(
        "AAA",
        0.0,
        100.0,
        TrancheSeniority::Senior,
        Money::new(10_000_000.0, Currency::USD),
        TrancheCoupon::Fixed { rate: 0.05 },
        maturity_date(),
    )
    .unwrap();

    let tranches = TrancheStructure::new(vec![tranche]).unwrap();
    let waterfall = WaterfallEngine::new(Currency::USD);

    let mut rmbs = StructuredCredit::new_rmbs(
        "TEST_RMBS",
        pool,
        tranches,
        waterfall,
        Date::from_calendar_date(2024, Month::January, 1).unwrap(),
        maturity_date(),
        "USD-OIS",
    );

    // Set behavior overrides
    rmbs.behavior_overrides.psa_speed_multiplier = Some(1.5);
    rmbs.behavior_overrides.cdr_annual = Some(0.01);

    // Act
    let json = serde_json::to_string(&rmbs).expect("Serialization failed");
    let deserialized: StructuredCredit =
        serde_json::from_str(&json).expect("Deserialization failed");

    // Assert
    assert_eq!(
        deserialized.behavior_overrides.psa_speed_multiplier,
        Some(1.5)
    );
    assert_eq!(deserialized.behavior_overrides.cdr_annual, Some(0.01));
}

// ============================================================================
// JSON Format Stability Tests
// ============================================================================

#[test]
fn test_prepayment_spec_json_format() {
    // Arrange
    let spec = PrepaymentModelSpec::Psa { multiplier: 150.0 };

    // Act
    let json = serde_json::to_string(&spec).unwrap();

    // Assert: Check JSON structure (wire format stability)
    assert!(json.contains("\"type\""));
    assert!(json.contains("\"psa\""));
    assert!(json.contains("\"multiplier\""));
    assert!(json.contains("150"));
}

#[test]
fn test_default_spec_json_format() {
    // Arrange
    let spec = DefaultModelSpec::ConstantCdr { cdr: 0.02 };

    // Act
    let json = serde_json::to_string(&spec).unwrap();

    // Assert: Check JSON structure
    assert!(json.contains("\"type\""));
    assert!(json.contains("\"constant_cdr\""));
    assert!(json.contains("\"cdr\""));
    assert!(json.contains("0.02"));
}

#[test]
fn test_recovery_spec_json_format() {
    // Arrange
    let spec = RecoveryModelSpec::Constant { rate: 0.70 };

    // Act
    let json = serde_json::to_string(&spec).unwrap();

    // Assert: Check JSON structure
    assert!(json.contains("\"type\""));
    assert!(json.contains("\"constant\""));
    assert!(json.contains("\"rate\""));
    assert!(json.contains("0.7"));
}

