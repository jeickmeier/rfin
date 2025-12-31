//! Tests for principal event validation in cashflow schedules.
//!
//! These tests verify boundary conditions and validation rules for principal events,
//! including date constraints relative to issue and maturity dates.

use finstack_core::cashflow::CFKind;
use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_valuations::cashflow::builder::{CashFlowSchedule, PrincipalEvent};
use time::Month;

use finstack_core::dates::Date;

// =============================================================================
// Principal Event Date Validation
// =============================================================================

#[test]
fn principal_events_after_maturity_rejected() {
    // Principal events after maturity should be rejected to prevent
    // post-maturity flows after outstanding has been zeroed out.

    let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let maturity = Date::from_calendar_date(2026, Month::January, 15).unwrap();
    let post_maturity = Date::from_calendar_date(2026, Month::February, 15).unwrap();

    let init = Money::new(1_000_000.0, Currency::USD);

    // Event after maturity should cause build to fail
    let event = PrincipalEvent {
        date: post_maturity,
        delta: Money::new(-100_000.0, Currency::USD), // Draw
        cash: Money::new(-100_000.0, Currency::USD),
        kind: CFKind::Notional,
    };

    let mut builder = CashFlowSchedule::builder();
    let _ = builder
        .principal(init, issue, maturity)
        .principal_events(&[event]);

    let result = builder.build_with_curves(None);
    assert!(
        result.is_err(),
        "Build should fail when principal event is after maturity"
    );

    // Error should indicate date is out of range
    let err_msg = format!("{}", result.unwrap_err());
    assert!(
        err_msg.contains("outside") || err_msg.contains("range"),
        "Error message should mention date is outside allowed range: {}",
        err_msg
    );
}

#[test]
fn principal_events_at_maturity_accepted() {
    // Principal events exactly at maturity should be allowed
    // (e.g., final draw for a bullet redemption structure)

    let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let maturity = Date::from_calendar_date(2026, Month::January, 15).unwrap();

    let init = Money::new(1_000_000.0, Currency::USD);

    // Event exactly at maturity should be allowed
    let event = PrincipalEvent {
        date: maturity,
        delta: Money::new(500_000.0, Currency::USD), // Partial repay at maturity
        cash: Money::new(500_000.0, Currency::USD),
        kind: CFKind::Notional,
    };

    let mut builder = CashFlowSchedule::builder();
    let _ = builder
        .principal(init, issue, maturity)
        .principal_events(&[event]);

    let result = builder.build_with_curves(None);
    assert!(
        result.is_ok(),
        "Build should succeed when principal event is exactly at maturity"
    );
}
