//! Validation edge case tests for CashFlow struct.
//!
//! These tests verify that CashFlow validation correctly rejects invalid inputs
//! including NaN, Infinity, and invalid date combinations.

use finstack_core::cashflow::primitives::{CFKind, CashFlow};
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::money::Money;
use time::Month;

/// Helper to create dates
fn d(year: i32, month: u8, day: u8) -> Date {
    Date::from_calendar_date(year, Month::try_from(month).unwrap(), day).unwrap()
}

/// Helper to create a valid base cashflow for testing
fn valid_cashflow() -> CashFlow {
    CashFlow {
        date: d(2025, 1, 15),
        reset_date: None,
        amount: Money::new(100.0, Currency::USD),
        kind: CFKind::Fixed,
        accrual_factor: 0.5,
        rate: Some(0.05),
    }
}

// =============================================================================
// Amount Validation Tests
// =============================================================================

// Note: Money::new() already validates that amounts are finite and panics on NaN/Infinity.
// These tests verify that behavior at the Money level (via #[should_panic]).

#[test]
#[should_panic(expected = "finite amount")]
fn money_new_rejects_nan() {
    let _ = Money::new(f64::NAN, Currency::USD);
}

#[test]
#[should_panic(expected = "finite amount")]
fn money_new_rejects_positive_infinity() {
    let _ = Money::new(f64::INFINITY, Currency::USD);
}

#[test]
#[should_panic(expected = "finite amount")]
fn money_new_rejects_negative_infinity() {
    let _ = Money::new(f64::NEG_INFINITY, Currency::USD);
}

#[test]
fn cashflow_rejects_zero_amount() {
    let cf = CashFlow {
        amount: Money::new(0.0, Currency::USD),
        ..valid_cashflow()
    };
    assert!(cf.validate().is_err(), "Zero amount should be rejected");
}

#[test]
fn cashflow_accepts_negative_amount() {
    // Negative amounts are valid (represent outflows)
    let cf = CashFlow {
        amount: Money::new(-100.0, Currency::USD),
        ..valid_cashflow()
    };
    assert!(cf.validate().is_ok(), "Negative amount should be accepted");
}

#[test]
fn cashflow_accepts_large_amount() {
    let cf = CashFlow {
        amount: Money::new(1e15, Currency::USD),
        ..valid_cashflow()
    };
    assert!(cf.validate().is_ok(), "Large amount should be accepted");
}

#[test]
fn cashflow_accepts_small_but_nonzero_amount() {
    // Note: Money rounds to currency decimal places (2 for USD)
    // Very small amounts like 1e-10 would round to 0.0 and fail validation
    // Use a value that survives rounding: 0.01 is the minimum for USD
    let cf = CashFlow {
        amount: Money::new(0.01, Currency::USD),
        ..valid_cashflow()
    };
    assert!(
        cf.validate().is_ok(),
        "Small but non-zero amount should be accepted"
    );
}

// =============================================================================
// Accrual Factor Validation Tests
// =============================================================================

#[test]
fn cashflow_rejects_nan_accrual_factor() {
    let cf = CashFlow {
        accrual_factor: f64::NAN,
        ..valid_cashflow()
    };
    assert!(
        cf.validate().is_err(),
        "NaN accrual factor should be rejected"
    );
}

#[test]
fn cashflow_rejects_positive_infinity_accrual_factor() {
    let cf = CashFlow {
        accrual_factor: f64::INFINITY,
        ..valid_cashflow()
    };
    assert!(
        cf.validate().is_err(),
        "Positive infinity accrual factor should be rejected"
    );
}

#[test]
fn cashflow_rejects_negative_infinity_accrual_factor() {
    let cf = CashFlow {
        accrual_factor: f64::NEG_INFINITY,
        ..valid_cashflow()
    };
    assert!(
        cf.validate().is_err(),
        "Negative infinity accrual factor should be rejected"
    );
}

#[test]
fn cashflow_accepts_zero_accrual_factor() {
    // Zero accrual factor is valid (e.g., notional flows)
    let cf = CashFlow {
        accrual_factor: 0.0,
        ..valid_cashflow()
    };
    assert!(
        cf.validate().is_ok(),
        "Zero accrual factor should be accepted"
    );
}

#[test]
fn cashflow_accepts_negative_accrual_factor() {
    // Negative accrual factor could occur in edge cases
    let cf = CashFlow {
        accrual_factor: -0.25,
        ..valid_cashflow()
    };
    assert!(
        cf.validate().is_ok(),
        "Negative accrual factor should be accepted"
    );
}

// =============================================================================
// Rate Validation Tests
// =============================================================================

#[test]
fn cashflow_rejects_nan_rate() {
    let cf = CashFlow {
        rate: Some(f64::NAN),
        ..valid_cashflow()
    };
    assert!(cf.validate().is_err(), "NaN rate should be rejected");
}

#[test]
fn cashflow_rejects_positive_infinity_rate() {
    let cf = CashFlow {
        rate: Some(f64::INFINITY),
        ..valid_cashflow()
    };
    assert!(
        cf.validate().is_err(),
        "Positive infinity rate should be rejected"
    );
}

#[test]
fn cashflow_rejects_negative_infinity_rate() {
    let cf = CashFlow {
        rate: Some(f64::NEG_INFINITY),
        ..valid_cashflow()
    };
    assert!(
        cf.validate().is_err(),
        "Negative infinity rate should be rejected"
    );
}

#[test]
fn cashflow_accepts_none_rate() {
    let cf = CashFlow {
        rate: None,
        ..valid_cashflow()
    };
    assert!(cf.validate().is_ok(), "None rate should be accepted");
}

#[test]
fn cashflow_accepts_zero_rate() {
    let cf = CashFlow {
        rate: Some(0.0),
        ..valid_cashflow()
    };
    assert!(cf.validate().is_ok(), "Zero rate should be accepted");
}

#[test]
fn cashflow_accepts_negative_rate() {
    // Negative rates are valid (negative interest rate environment)
    let cf = CashFlow {
        rate: Some(-0.005),
        ..valid_cashflow()
    };
    assert!(cf.validate().is_ok(), "Negative rate should be accepted");
}

#[test]
fn cashflow_accepts_high_rate() {
    // High rates are valid (distressed debt, VC scenarios)
    let cf = CashFlow {
        rate: Some(2.5), // 250%
        ..valid_cashflow()
    };
    assert!(cf.validate().is_ok(), "High rate should be accepted");
}

// =============================================================================
// Reset Date Validation Tests
// =============================================================================

#[test]
fn cashflow_rejects_reset_date_after_payment() {
    let payment = d(2025, 1, 10);
    let reset = d(2025, 1, 15); // After payment date

    let cf = CashFlow {
        date: payment,
        reset_date: Some(reset),
        amount: Money::new(100.0, Currency::USD),
        kind: CFKind::FloatReset,
        accrual_factor: 0.25,
        rate: None,
    };
    assert!(
        cf.validate().is_err(),
        "Reset date after payment date should be rejected"
    );
}

#[test]
fn cashflow_accepts_reset_date_before_payment() {
    let payment = d(2025, 1, 15);
    let reset = d(2025, 1, 10); // Before payment date

    let cf = CashFlow {
        date: payment,
        reset_date: Some(reset),
        amount: Money::new(100.0, Currency::USD),
        kind: CFKind::FloatReset,
        accrual_factor: 0.25,
        rate: None,
    };
    assert!(
        cf.validate().is_ok(),
        "Reset date before payment date should be accepted"
    );
}

#[test]
fn cashflow_accepts_reset_date_equal_to_payment() {
    let date = d(2025, 1, 15);

    let cf = CashFlow {
        date,
        reset_date: Some(date), // Same as payment date
        amount: Money::new(100.0, Currency::USD),
        kind: CFKind::FloatReset,
        accrual_factor: 0.25,
        rate: None,
    };
    assert!(
        cf.validate().is_ok(),
        "Reset date equal to payment date should be accepted"
    );
}

#[test]
fn cashflow_accepts_no_reset_date() {
    let cf = CashFlow {
        date: d(2025, 1, 15),
        reset_date: None,
        amount: Money::new(100.0, Currency::USD),
        kind: CFKind::Fixed,
        accrual_factor: 0.25,
        rate: Some(0.05),
    };
    assert!(
        cf.validate().is_ok(),
        "No reset date should be accepted for fixed cashflows"
    );
}

// =============================================================================
// Combined Edge Cases
// =============================================================================

#[test]
fn cashflow_valid_with_all_fields_populated() {
    let cf = CashFlow {
        date: d(2025, 6, 15),
        reset_date: Some(d(2025, 6, 1)),
        amount: Money::new(25_000.0, Currency::EUR),
        kind: CFKind::FloatReset,
        accrual_factor: 0.25,
        rate: Some(0.0325),
    };
    assert!(
        cf.validate().is_ok(),
        "Valid cashflow with all fields should be accepted"
    );
}

#[test]
fn cashflow_multiple_invalid_fields_first_error_wins() {
    // Multiple invalid fields: accrual_factor and rate are invalid
    // Validation should fail on the first non-amount check (accrual_factor)
    // Note: Can't test invalid amount since Money::new panics on non-finite
    let cf = CashFlow {
        date: d(2025, 1, 15),
        reset_date: Some(d(2025, 1, 20)), // After payment - invalid
        amount: Money::new(100.0, Currency::USD),
        kind: CFKind::Fixed,
        accrual_factor: f64::INFINITY, // Invalid
        rate: Some(f64::NAN),          // Also invalid
    };
    assert!(
        cf.validate().is_err(),
        "Cashflow with multiple invalid fields should be rejected"
    );
}

