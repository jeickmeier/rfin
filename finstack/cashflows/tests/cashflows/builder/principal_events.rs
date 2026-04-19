//! Tests for principal event validation in cashflow schedules.
//!
//! These tests verify boundary conditions and validation rules for principal events,
//! including date constraints relative to issue and maturity dates.
//!
//! # Coverage
//!
//! - Date boundary validation (before issue, at issue, at maturity, after maturity)
//! - Currency mismatch detection
//! - Multiple events on same date
//! - Draw vs repay semantics
//! - Outstanding balance constraints

use finstack_cashflows::builder::{CashFlowSchedule, PrincipalEvent};
use finstack_core::cashflow::CFKind;
use finstack_core::currency::Currency;
use finstack_core::money::Money;
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
        .add_principal_event(event.date, event.delta, Some(event.cash), event.kind);

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
        .add_principal_event(event.date, event.delta, Some(event.cash), event.kind);

    let result = builder.build_with_curves(None);
    assert!(
        result.is_ok(),
        "Build should succeed when principal event is exactly at maturity"
    );
}

// =============================================================================
// Principal Event Before Issue Date
// =============================================================================

#[test]
fn principal_events_before_issue_included_and_adjusts_outstanding() {
    // Principal events before issue date are included and should adjust
    // the initial outstanding balance (e.g., delayed funding structures).

    let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let maturity = Date::from_calendar_date(2026, Month::January, 15).unwrap();
    let pre_issue = Date::from_calendar_date(2024, Month::December, 15).unwrap();

    let init = Money::new(1_000_000.0, Currency::USD);

    // Draw 100k before issue (positive delta increases outstanding)
    let event = PrincipalEvent {
        date: pre_issue,
        delta: Money::new(100_000.0, Currency::USD),
        cash: Money::new(100_000.0, Currency::USD),
        kind: CFKind::Notional,
    };

    let mut builder = CashFlowSchedule::builder();
    let _ = builder
        .principal(init, issue, maturity)
        .add_principal_event(event.date, event.delta, Some(event.cash), event.kind);

    let schedule = builder.build_with_curves(None).unwrap();

    // Pre-issue event should appear in flows
    assert!(
        schedule.flows.iter().any(|cf| cf.date == pre_issue),
        "Pre-issue event should appear in flows"
    );

    // Outstanding at issue should include the pre-issue draw
    let outstanding = schedule.outstanding_by_date().unwrap();
    let issue_outstanding = outstanding
        .iter()
        .find(|(d, _)| *d == issue)
        .map(|(_, m)| m.amount())
        .unwrap();
    assert!(
        (issue_outstanding - (init.amount() + 100_000.0)).abs() < 0.01,
        "Outstanding at issue should include pre-issue draw: expected {}, got {}",
        init.amount() + 100_000.0,
        issue_outstanding
    );
}

#[test]
fn principal_events_at_issue_accepted() {
    // Principal events exactly at issue date should be allowed
    // (e.g., partial funding at closing)

    let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let maturity = Date::from_calendar_date(2026, Month::January, 15).unwrap();

    let init = Money::new(1_000_000.0, Currency::USD);

    // Additional draw at issue (delayed draw term loan pattern)
    let event = PrincipalEvent {
        date: issue,
        delta: Money::new(-500_000.0, Currency::USD), // Additional draw
        cash: Money::new(-500_000.0, Currency::USD),
        kind: CFKind::Notional,
    };

    let mut builder = CashFlowSchedule::builder();
    let _ = builder
        .principal(init, issue, maturity)
        .add_principal_event(event.date, event.delta, Some(event.cash), event.kind);

    let result = builder.build_with_curves(None);
    assert!(
        result.is_ok(),
        "Build should succeed when principal event is at issue date"
    );
}

// =============================================================================
// Currency Mismatch Validation
// =============================================================================

#[test]
fn principal_events_currency_mismatch_rejected() {
    // Principal events with different currency than notional should be rejected
    // to avoid cross-currency outstanding tracking.

    let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let maturity = Date::from_calendar_date(2026, Month::January, 15).unwrap();
    let mid_date = Date::from_calendar_date(2025, Month::July, 15).unwrap();

    let init = Money::new(1_000_000.0, Currency::USD);

    // Event in EUR when notional is USD
    let event = PrincipalEvent {
        date: mid_date,
        delta: Money::new(100_000.0, Currency::EUR), // Wrong currency
        cash: Money::new(100_000.0, Currency::EUR),
        kind: CFKind::Notional,
    };

    let mut builder = CashFlowSchedule::builder();
    let _ = builder
        .principal(init, issue, maturity)
        .add_principal_event(event.date, event.delta, Some(event.cash), event.kind);

    let result = builder.build_with_curves(None);
    assert!(
        result.is_err(),
        "Build should fail when principal event currency differs from notional"
    );
}

#[test]
fn principal_event_delta_cash_currency_mismatch_rejected() {
    // Delta/cash currency mismatch should be rejected at the builder layer.
    let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let maturity = Date::from_calendar_date(2026, Month::January, 15).unwrap();
    let mid_date = Date::from_calendar_date(2025, Month::July, 15).unwrap();

    let init = Money::new(1_000_000.0, Currency::USD);

    let event = PrincipalEvent {
        date: mid_date,
        delta: Money::new(100_000.0, Currency::USD),
        cash: Money::new(100_000.0, Currency::EUR), // Mismatch
        kind: CFKind::Notional,
    };

    let mut builder = CashFlowSchedule::builder();
    let _ = builder
        .principal(init, issue, maturity)
        .add_principal_event(event.date, event.delta, Some(event.cash), event.kind);

    let result = builder.build_with_curves(None);
    assert!(
        result.is_err(),
        "Build should fail when principal event delta/cash currencies differ"
    );
}

// =============================================================================
// Multiple Events on Same Date
// =============================================================================

#[test]
fn multiple_principal_events_same_date_accepted() {
    // Multiple principal events on the same date should be processed
    // (e.g., draw and partial repay on same day for restructuring)

    let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let maturity = Date::from_calendar_date(2026, Month::January, 15).unwrap();
    let mid_date = Date::from_calendar_date(2025, Month::July, 15).unwrap();

    let init = Money::new(1_000_000.0, Currency::USD);

    // Two events on same date
    let events = [
        PrincipalEvent {
            date: mid_date,
            delta: Money::new(200_000.0, Currency::USD), // Draw
            cash: Money::new(200_000.0, Currency::USD),
            kind: CFKind::Notional,
        },
        PrincipalEvent {
            date: mid_date,
            delta: Money::new(-100_000.0, Currency::USD), // Repay
            cash: Money::new(-100_000.0, Currency::USD),
            kind: CFKind::Notional,
        },
    ];

    let mut builder = CashFlowSchedule::builder();
    let _ = builder
        .principal(init, issue, maturity)
        .add_principal_event(
            events[0].date,
            events[0].delta,
            Some(events[0].cash),
            events[0].kind,
        )
        .add_principal_event(
            events[1].date,
            events[1].delta,
            Some(events[1].cash),
            events[1].kind,
        );

    let result = builder.build_with_curves(None);
    assert!(
        result.is_ok(),
        "Build should succeed with multiple events on same date"
    );

    // Verify net effect: -200k + 100k = -100k net draw
    let schedule = result.unwrap();
    let notional_flows_on_mid: Vec<_> = schedule
        .flows
        .iter()
        .filter(|cf| cf.date == mid_date && cf.kind == CFKind::Notional)
        .collect();

    // Should have both events as separate flows
    assert!(
        notional_flows_on_mid.len() >= 2,
        "Should have at least 2 notional flows on mid_date"
    );

    // Net outstanding change should be +100k (draw 200k, repay 100k)
    let outstanding = schedule.outstanding_by_date().unwrap();
    let mid_outstanding = outstanding
        .iter()
        .find(|(d, _)| *d == mid_date)
        .map(|(_, m)| m.amount())
        .unwrap();
    assert!(
        (mid_outstanding - (init.amount() + 100_000.0)).abs() < 0.01,
        "Outstanding after same-day events should be {}, got {}",
        init.amount() + 100_000.0,
        mid_outstanding
    );
}

// =============================================================================
// Draw and Repay Semantics
// =============================================================================

#[test]
fn principal_event_draw_increases_outstanding() {
    // A draw should increase outstanding balance
    //
    // Note: The sign convention for draws may vary by implementation.
    // Negative delta typically means cash outflow (draw), positive means inflow (repay).
    // Check actual behavior and document.

    let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let maturity = Date::from_calendar_date(2026, Month::January, 15).unwrap();
    let mid_date = Date::from_calendar_date(2025, Month::July, 15).unwrap();

    let init = Money::new(1_000_000.0, Currency::USD);

    // Draw 500k more (positive delta increases outstanding)
    let event = PrincipalEvent {
        date: mid_date,
        delta: Money::new(500_000.0, Currency::USD),
        cash: Money::new(500_000.0, Currency::USD),
        kind: CFKind::Notional,
    };

    let mut builder = CashFlowSchedule::builder();
    let _ = builder
        .principal(init, issue, maturity)
        .add_principal_event(event.date, event.delta, Some(event.cash), event.kind);

    let schedule = builder.build_with_curves(None).unwrap();
    let outstanding = schedule.outstanding_by_date().unwrap();

    // Find outstanding at mid_date
    let mid_outstanding = outstanding
        .iter()
        .find(|(d, _)| *d == mid_date)
        .map(|(_, m)| m.amount())
        .unwrap();

    assert!(
        (mid_outstanding - (init.amount() + 500_000.0)).abs() < 0.01,
        "Outstanding should increase after draw: expected {}, got {}",
        init.amount() + 500_000.0,
        mid_outstanding
    );
}

#[test]
fn principal_event_repay_effect_on_outstanding() {
    // Test that principal events are included in the outstanding path
    //
    // Note: The sign convention and exact semantics depend on implementation.
    // This test verifies events are processed, not specific values.

    let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let maturity = Date::from_calendar_date(2026, Month::January, 15).unwrap();
    let mid_date = Date::from_calendar_date(2025, Month::July, 15).unwrap();

    let init = Money::new(1_000_000.0, Currency::USD);

    // Add a principal event
    let event = PrincipalEvent {
        date: mid_date,
        delta: Money::new(-300_000.0, Currency::USD),
        cash: Money::new(-300_000.0, Currency::USD),
        kind: CFKind::Notional,
    };

    let mut builder = CashFlowSchedule::builder();
    let _ = builder
        .principal(init, issue, maturity)
        .add_principal_event(event.date, event.delta, Some(event.cash), event.kind);

    let schedule = builder.build_with_curves(None).unwrap();

    // Verify the event was added to flows
    let notional_flows: Vec<_> = schedule
        .flows
        .iter()
        .filter(|cf| cf.kind == CFKind::Notional)
        .collect();

    // Should have at least 2 notional flows: initial funding + our event (+ maturity redemption)
    assert!(
        notional_flows.len() >= 2,
        "Should have at least 2 notional flows, got {}",
        notional_flows.len()
    );

    // Verify a flow exists at mid_date
    let mid_flows: Vec<_> = schedule
        .flows
        .iter()
        .filter(|cf| cf.date == mid_date && cf.kind == CFKind::Notional)
        .collect();

    assert!(
        !mid_flows.is_empty(),
        "Should have a notional flow at mid_date"
    );

    // Outstanding should decrease by 300k
    let outstanding = schedule.outstanding_by_date().unwrap();
    let mid_outstanding = outstanding
        .iter()
        .find(|(d, _)| *d == mid_date)
        .map(|(_, m)| m.amount())
        .unwrap();
    assert!(
        (mid_outstanding - (init.amount() - 300_000.0)).abs() < 0.01,
        "Outstanding should decrease after repayment: expected {}, got {}",
        init.amount() - 300_000.0,
        mid_outstanding
    );
}

// =============================================================================
// Empty Events List
// =============================================================================

#[test]
fn empty_principal_events_accepted() {
    // Empty events list should be valid (no ad-hoc principal changes)

    let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let maturity = Date::from_calendar_date(2026, Month::January, 15).unwrap();

    let init = Money::new(1_000_000.0, Currency::USD);

    let mut builder = CashFlowSchedule::builder();
    let _ = builder.principal(init, issue, maturity);

    let result = builder.build_with_curves(None);
    assert!(
        result.is_ok(),
        "Build should succeed with empty events list"
    );
}
