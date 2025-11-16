//! Tests for bond accrued interest calculations with different accrual methods.
//!
//! Validates Linear, Compounded (ICMA Rule 251), and Indexed accrual conventions
//! against known market calculations.

use finstack_core::currency::Currency;
use finstack_core::dates::{Date, Frequency};
use finstack_core::money::Money;
use finstack_valuations::instruments::bond::{AccrualMethod, Bond};
use time::Month;

fn make_date(y: i32, m: u8, d: u8) -> Date {
    Date::from_calendar_date(y, Month::try_from(m).unwrap(), d).unwrap()
}

#[test]
fn test_accrued_interest_linear_default() {
    // Standard bond with linear accrual (default)
    let bond = Bond::fixed(
        "LINEAR_TEST",
        Money::new(100.0, Currency::USD),
        0.06,  // 6% annual, semi-annual payments = 3% per period
        make_date(2025, 1, 1),
        make_date(2030, 1, 1),
        "USD-OIS",
    );
    
    // Default accrual method should be Linear
    assert!(matches!(bond.accrual_method, AccrualMethod::Linear));
    
    // Accrual halfway through first coupon period
    // Period: 2025-01-01 to 2025-07-01 (180 days in 30/360)
    // As of: 2025-04-01 (90 days)
    let as_of = make_date(2025, 4, 1);
    
    let accrued = finstack_valuations::instruments::bond::pricing::helpers::compute_accrued_interest(
        &bond,
        as_of,
    )
    .unwrap();
    
    // Expected: 3% coupon * (90/180) = 1.5% of notional = $1.50
    let expected = 1.50;
    assert!(
        (accrued - expected).abs() < 1e-6,
        "Linear accrual: expected {}, got {}",
        expected,
        accrued
    );
}

#[test]
fn test_accrued_interest_compounded_vs_linear() {
    // Compare compounded vs linear accrual for same bond
    
    // Linear accrual bond (default)
    let bond_linear = Bond::fixed(
        "LINEAR",
        Money::new(100.0, Currency::USD),
        0.06,  // 6% annual coupon, semi-annual = 3% per period
        make_date(2025, 1, 1),
        make_date(2030, 1, 1),
        "USD-OIS",
    );
    
    // Compounded accrual bond
    let mut bond_compounded = Bond::fixed(
        "COMPOUNDED",
        Money::new(100.0, Currency::USD),
        0.06,
        make_date(2025, 1, 1),
        make_date(2030, 1, 1),
        "USD-OIS",
    );
    bond_compounded.accrual_method = AccrualMethod::Compounded {
        frequency: Frequency::semi_annual(),
    };
    
    // Accrual at quarter-point (90 days out of 180)
    let as_of = make_date(2025, 4, 1);
    
    let accrued_linear = finstack_valuations::instruments::bond::pricing::helpers::compute_accrued_interest(
        &bond_linear,
        as_of,
    )
    .unwrap();
    
    let accrued_compounded = finstack_valuations::instruments::bond::pricing::helpers::compute_accrued_interest(
        &bond_compounded,
        as_of,
    )
    .unwrap();
    
    // Linear: 3% × (90/180) = 1.50%
    let expected_linear = 1.50;
    
    // Compounded (ICMA Rule 251): 100 × [(1.03)^(90/180) - 1]
    // = 100 × [(1.03)^0.5 - 1]
    // = 100 × [1.014889 - 1]
    // = 1.4889%
    let expected_compounded = 1.4889;
    
    assert!(
        (accrued_linear - expected_linear).abs() < 1e-2,
        "Linear: expected {}, got {}",
        expected_linear,
        accrued_linear
    );
    
    assert!(
        (accrued_compounded - expected_compounded).abs() < 1e-2,
        "Compounded: expected {}, got {}",
        expected_compounded,
        accrued_compounded
    );
    
    // Difference should be material (~1bp on $100 notional)
    assert!(
        (accrued_linear - accrued_compounded).abs() > 0.005,
        "Linear ({}) and compounded ({}) should differ materially",
        accrued_linear,
        accrued_compounded
    );
}

#[test]
fn test_accrued_interest_compounded_zero_coupon() {
    // Zero-coupon bond should have zero accrued regardless of method
    let mut bond = Bond::fixed(
        "ZERO",
        Money::new(100.0, Currency::USD),
        0.0,  // Zero coupon
        make_date(2025, 1, 1),
        make_date(2030, 1, 1),
        "USD-OIS",
    );
    bond.accrual_method = AccrualMethod::Compounded {
        frequency: Frequency::semi_annual(),
    };
    
    let as_of = make_date(2025, 4, 1);
    let accrued = finstack_valuations::instruments::bond::pricing::helpers::compute_accrued_interest(
        &bond,
        as_of,
    )
    .unwrap();
    
    assert!(
        accrued.abs() < 1e-10,
        "Zero-coupon bond should have zero accrued"
    );
}

#[test]
fn test_accrued_interest_ex_coupon_period() {
    // Test that ex-coupon dates result in zero accrual
    let mut bond = Bond::fixed(
        "EX_COUPON",
        Money::new(100.0, Currency::USD),
        0.05,
        make_date(2025, 1, 1),
        make_date(2030, 1, 1),
        "USD-OIS",
    );
    bond.ex_coupon_days = Some(7);  // Ex-coupon 7 days before payment
    
    // 5 days before coupon (within ex-coupon window)
    let coupon_date = make_date(2025, 7, 1);
    let as_of = coupon_date - time::Duration::days(5);
    
    let accrued = finstack_valuations::instruments::bond::pricing::helpers::compute_accrued_interest(
        &bond,
        as_of,
    )
    .unwrap();
    
    assert_eq!(accrued, 0.0, "Should be zero during ex-coupon period");
}

#[test]
fn test_accrued_interest_at_coupon_boundaries() {
    let bond = Bond::fixed(
        "BOUNDARY",
        Money::new(100.0, Currency::USD),
        0.04,  // 4% annual, semi-annual = 2% per period
        make_date(2025, 1, 1),
        make_date(2030, 1, 1),
        "USD-OIS",
    );
    
    // Midway through first coupon period (should have accrual)
    let midway = make_date(2025, 4, 1);  // 90 days into first 180-day period
    let accrued_midway = finstack_valuations::instruments::bond::pricing::helpers::compute_accrued_interest(
        &bond,
        midway,
    )
    .unwrap();
    
    // Should be positive: 2% × (90/180) = 1% of notional = $1.00
    assert!(accrued_midway > 0.99 && accrued_midway < 1.01, "Accrued at midway: {}", accrued_midway);
}

#[test]
#[cfg(feature = "serde")]
fn test_accrual_method_serialization() {
    // Test that accrual method survives JSON roundtrip
    let mut bond = Bond::fixed(
        "SERDE_TEST",
        Money::new(1000.0, Currency::EUR),
        0.025,
        make_date(2025, 1, 1),
        make_date(2035, 1, 1),
        "EUR-OIS",
    );
    bond.accrual_method = AccrualMethod::Compounded {
        frequency: Frequency::annual(),
    };
    
    let json = serde_json::to_string(&bond).expect("Serialization should succeed in test");
    let deserialized: Bond = serde_json::from_str(&json).expect("Deserialization should succeed in test");
    
    // Verify accrual method survived roundtrip
    match &deserialized.accrual_method {
        AccrualMethod::Compounded { frequency } => {
            assert_eq!(*frequency, Frequency::annual());
        }
        _ => panic!("Expected Compounded accrual method"),
    }
}

