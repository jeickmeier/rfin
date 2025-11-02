//! Cashflow generation and validation tests.

use crate::deposit::common::*;
use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_valuations::cashflow::traits::CashflowProvider;

#[test]
fn test_cashflow_generation_two_flows() {
    // Setup
    let base = date(2025, 1, 1);
    let ctx = ctx_with_standard_disc(base, "USD-OIS");

    let dep = DepositBuilder::new(base)
        .start(base)
        .end(date(2025, 7, 1))
        .quote_rate(0.03)
        .build();

    // Execute
    let flows = dep.build_schedule(&ctx, base).unwrap();

    // Validate - should have exactly 2 flows
    assert_eq!(flows.len(), 2, "Expected 2 cashflows");

    // First flow (payment at start)
    assert_eq!(flows[0].0, base);
    assert_eq!(flows[0].1.currency(), Currency::USD);
    assert!(
        flows[0].1.amount() < 0.0,
        "First flow should be negative (payment)"
    );

    // Second flow (receipt at end)
    assert_eq!(flows[1].0, date(2025, 7, 1));
    assert_eq!(flows[1].1.currency(), Currency::USD);
    assert!(
        flows[1].1.amount() > 0.0,
        "Second flow should be positive (receipt)"
    );
}

#[test]
fn test_cashflow_redemption_amount() {
    // Setup
    let base = date(2025, 1, 1);
    let ctx = ctx_with_standard_disc(base, "USD-OIS");
    let notional = 1_000_000.0;
    let rate = 0.04;

    let dep = DepositBuilder::new(base)
        .notional(Money::new(notional, Currency::USD))
        .start(base)
        .end(date(2025, 7, 1))
        .quote_rate(rate)
        .build();

    // Execute
    let flows = dep.build_schedule(&ctx, base).unwrap();

    // Calculate expected redemption
    let yf = dep
        .day_count
        .year_fraction(
            dep.start,
            dep.end,
            finstack_core::dates::DayCountCtx::default(),
        )
        .unwrap();
    let expected_redemption = notional * (1.0 + rate * yf);

    // Validate
    assert!((flows[1].1.amount() - expected_redemption).abs() < 1.0);
}

#[test]
fn test_cashflow_conservation_of_value() {
    // Setup - sum of discounted cashflows should equal PV
    let base = date(2025, 1, 1);
    let ctx = ctx_with_standard_disc(base, "USD-OIS");

    let dep = DepositBuilder::new(base)
        .start(base)
        .end(date(2025, 7, 1))
        .quote_rate(0.03)
        .build();

    // Execute
    let flows = dep.build_schedule(&ctx, base).unwrap();
    let pv = dep.npv(&ctx, base).unwrap();

    // Manually discount flows
    let disc = ctx.get_discount_ref("USD-OIS").unwrap();
    let mut manual_pv: f64 = 0.0;
    for (date, amount) in flows {
        let df = disc.df_on_date_curve(date);
        manual_pv += amount.amount() * df;
    }

    // Validate - manual calculation should match npv() within tolerance
    assert!(
        (manual_pv - pv.amount()).abs() < 1000.0,
        "Difference: {}",
        (manual_pv - pv.amount()).abs()
    );
}

#[test]
fn test_cashflow_with_zero_rate() {
    // Setup
    let base = date(2025, 1, 1);
    let ctx = ctx_with_standard_disc(base, "USD-OIS");
    let notional = 1_000_000.0;

    let dep = DepositBuilder::new(base)
        .notional(Money::new(notional, Currency::USD))
        .start(base)
        .end(date(2025, 7, 1))
        .quote_rate(0.0)
        .build();

    // Execute
    let flows = dep.build_schedule(&ctx, base).unwrap();

    // Validate - with zero rate, redemption = notional
    assert!((flows[1].1.amount() - notional).abs() < 1e-9);
}

#[test]
fn test_cashflow_dates_ordered() {
    // Setup
    let base = date(2025, 1, 1);
    let ctx = ctx_with_standard_disc(base, "USD-OIS");

    let dep = DepositBuilder::new(base)
        .start(base)
        .end(date(2025, 7, 1))
        .build();

    // Execute
    let flows = dep.build_schedule(&ctx, base).unwrap();

    // Validate - dates should be in order
    assert!(flows[0].0 < flows[1].0);
}

#[test]
fn test_cashflow_notional_scales() {
    // Setup
    let base = date(2025, 1, 1);
    let ctx = ctx_with_standard_disc(base, "USD-OIS");

    let dep_1m = DepositBuilder::new(base)
        .notional(Money::new(1_000_000.0, Currency::USD))
        .quote_rate(0.03)
        .build();

    let dep_2m = DepositBuilder::new(base)
        .notional(Money::new(2_000_000.0, Currency::USD))
        .quote_rate(0.03)
        .build();

    // Execute
    let flows_1m = dep_1m.build_schedule(&ctx, base).unwrap();
    let flows_2m = dep_2m.build_schedule(&ctx, base).unwrap();

    // Validate - cashflows should scale linearly
    assert!((flows_2m[0].1.amount() / flows_1m[0].1.amount() - 2.0).abs() < 1e-10);
    assert!((flows_2m[1].1.amount() / flows_1m[1].1.amount() - 2.0).abs() < 1e-6);
}

#[test]
fn test_cashflow_currency_consistency() {
    // Setup
    let base = date(2025, 1, 1);
    let ctx = ctx_with_standard_disc(base, "USD-OIS");

    let dep = DepositBuilder::new(base)
        .notional(Money::new(1_000_000.0, Currency::EUR))
        .discount_curve_id("EUR-OIS")
        .build();

    // Execute
    let flows = dep.build_schedule(&ctx, base).unwrap();

    // Validate - all flows should be in EUR
    for (_, amount) in flows {
        assert_eq!(amount.currency(), Currency::EUR);
    }
}
