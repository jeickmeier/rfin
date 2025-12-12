//! Edge case and boundary condition tests for Deposit instruments.
//!
//! Tests special scenarios and boundary conditions to ensure robustness.

use super::common::*;
use finstack_core::currency::Currency;
use finstack_core::dates::DayCount;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::Deposit;
use finstack_valuations::metrics::MetricId;

#[test]
fn test_zero_period_deposit() {
    // Setup - start == end (invalid - should fail validation)
    let base = date(2025, 1, 1);
    let ctx = ctx_with_standard_disc(base, "USD-OIS");

    let dep = DepositBuilder::new(base)
        .start(base)
        .end(base)
        .quote_rate(0.05)
        .build();

    // Execute - should fail validation (end must be after start)
    let result = dep.npv(&ctx, base);

    // Validate - zero period deposits are invalid
    assert!(
        result.is_err(),
        "Zero period deposit should fail validation"
    );
}

#[test]
fn test_zero_rate_deposit() {
    // Setup
    let base = date(2025, 1, 1);
    let ctx = ctx_with_standard_disc(base, "USD-OIS");

    let dep = DepositBuilder::new(base)
        .end(date(2025, 7, 1))
        .quote_rate(0.0)
        .build();

    // Execute
    let pv = dep.npv(&ctx, base).unwrap();

    // Validate - should be negative (time value of money)
    assert!(pv.amount() < 0.0);
}

#[test]
fn test_very_high_rate() {
    // Setup - test with unrealistically high rate (100%)
    let base = date(2025, 1, 1);
    let ctx = ctx_with_standard_disc(base, "USD-OIS");

    let dep = DepositBuilder::new(base)
        .end(date(2025, 7, 1))
        .quote_rate(1.0)
        .build();

    // Execute
    let pv = dep.npv(&ctx, base).unwrap();

    // Validate - should compute without error
    assert!(pv.currency() == Currency::USD);
    assert!(pv.amount().is_finite());
}

#[test]
fn test_very_small_notional() {
    // Setup
    let base = date(2025, 1, 1);
    let ctx = ctx_with_standard_disc(base, "USD-OIS");

    let dep = DepositBuilder::new(base)
        .notional(Money::new(0.01, Currency::USD))
        .end(date(2025, 7, 1))
        .quote_rate(0.03)
        .build();

    // Execute
    let pv = dep.npv(&ctx, base).unwrap();

    // Validate
    assert!(pv.amount().is_finite());
    assert!(pv.amount().abs() < 1.0); // Very small
}

#[test]
fn test_very_large_notional() {
    // Setup - test with trillion dollar notional
    let base = date(2025, 1, 1);
    let ctx = ctx_with_standard_disc(base, "USD-OIS");

    let dep = DepositBuilder::new(base)
        .notional(Money::new(1_000_000_000_000.0, Currency::USD))
        .end(date(2025, 7, 1))
        .quote_rate(0.03)
        .build();

    // Execute
    let pv = dep.npv(&ctx, base).unwrap();

    // Validate
    assert!(pv.amount().is_finite());
    assert!(pv.amount().abs() > 1_000_000_000.0); // Appropriately large
}

#[test]
fn test_very_short_maturity_one_day() {
    // Setup - overnight deposit
    let base = date(2025, 1, 1);
    let ctx = ctx_with_standard_disc(base, "USD-OIS");

    let dep = DepositBuilder::new(base)
        .start(base)
        .end(date(2025, 1, 2))
        .quote_rate(0.03)
        .build();

    // Execute
    let pv = dep.npv(&ctx, base).unwrap();
    let yf = compute_metric(&dep, &ctx, base, MetricId::Yf);

    // Validate
    assert!(pv.amount().is_finite());
    assert!(yf < 0.01); // Very small year fraction
}

#[test]
fn test_very_long_maturity() {
    // Setup - 10 year deposit
    let base = date(2025, 1, 1);
    let ctx = ctx_with_standard_disc(base, "USD-OIS");

    let dep = DepositBuilder::new(base)
        .end(date(2035, 1, 1))
        .quote_rate(0.03)
        .build();

    // Execute
    let pv = dep.npv(&ctx, base).unwrap();

    // Validate
    assert!(pv.amount().is_finite());
}

#[test]
fn test_negative_rate_environment() {
    // Setup - negative quoted rate
    let base = date(2025, 1, 1);
    let ctx = ctx_with_standard_disc(base, "USD-OIS");

    let dep = DepositBuilder::new(base)
        .end(date(2025, 7, 1))
        .quote_rate(-0.005)
        .build();

    // Execute
    let pv = dep.npv(&ctx, base).unwrap();

    // Validate - should compute correctly
    assert!(pv.currency() == Currency::USD);
    assert!(pv.amount().is_finite());
    // With negative rate, get back less than principal
    assert!(pv.amount() < 0.0);
}

#[test]
fn test_pricing_on_start_date() {
    // Setup - price exactly on start date
    let base = date(2025, 1, 1);
    let start = date(2025, 2, 1);
    let ctx = ctx_with_standard_disc(base, "USD-OIS");

    let dep = DepositBuilder::new(base)
        .start(start)
        .end(date(2025, 8, 1))
        .quote_rate(0.03)
        .build();

    // Execute
    let pv = dep.npv(&ctx, start).unwrap();

    // Validate
    assert!(pv.amount().is_finite());
}

#[test]
fn test_pricing_after_maturity() {
    // Setup - price after end date
    let base = date(2025, 1, 1);
    let ctx = ctx_with_standard_disc(base, "USD-OIS");

    let dep = DepositBuilder::new(date(2024, 1, 1))
        .start(date(2024, 1, 1))
        .end(date(2024, 7, 1))
        .quote_rate(0.03)
        .build();

    // Execute - price on date after maturity
    let pv = dep.npv(&ctx, base).unwrap();

    // Validate - should handle gracefully
    assert!(pv.amount().is_finite());
}

#[test]
fn test_thirty360_with_end_of_month() {
    // Setup - test 30/360 with month end dates
    let base = date(2025, 1, 31);
    let ctx = ctx_with_standard_disc(base, "USD-OIS");

    let dep = DepositBuilder::new(base)
        .start(date(2025, 1, 31))
        .end(date(2025, 7, 31))
        .day_count(DayCount::Thirty360)
        .quote_rate(0.03)
        .build();

    // Execute
    let pv = dep.npv(&ctx, base).unwrap();
    let yf = compute_metric(&dep, &ctx, base, MetricId::Yf);

    // Validate
    assert!(pv.amount().is_finite());
    assert!((yf - 0.5).abs() < 0.01); // 30/360 treats as 6 months
}

#[test]
fn test_leap_year_handling() {
    // Setup - test with leap year date
    let base = date(2024, 2, 29);
    let ctx = ctx_with_standard_disc(base, "USD-OIS");

    let dep = DepositBuilder::new(base)
        .start(base)
        .end(date(2024, 8, 29))
        .day_count(DayCount::Act365F)
        .quote_rate(0.03)
        .build();

    // Execute
    let pv = dep.npv(&ctx, base).unwrap();

    // Validate
    assert!(pv.amount().is_finite());
}

#[test]
fn test_missing_quote_rate_defaults_to_zero() {
    // Setup - no quote rate set
    let base = date(2025, 1, 1);
    let ctx = ctx_with_standard_disc(base, "USD-OIS");

    let dep = Deposit::builder()
        .id(InstrumentId::new("DEP-NO-QUOTE"))
        .notional(Money::new(1_000_000.0, Currency::USD))
        .start(base)
        .end(date(2025, 7, 1))
        .day_count(DayCount::Act360)
        .discount_curve_id(CurveId::new("USD-OIS"))
        .build()
        .unwrap();

    // Execute
    let err = dep.npv(&ctx, base).expect_err("npv() should require quote_rate");

    // Validate
    let msg = err.to_string();
    assert!(
        msg.contains("quote_rate"),
        "Error should mention quote_rate: {msg}"
    );
}

#[test]
fn test_back_to_back_deposits_same_period() {
    // Setup - two deposits with identical parameters
    let base = date(2025, 1, 1);
    let ctx = ctx_with_standard_disc(base, "USD-OIS");

    let dep1 = DepositBuilder::new(base)
        .id("DEP-1")
        .end(date(2025, 7, 1))
        .quote_rate(0.03)
        .build();

    let dep2 = DepositBuilder::new(base)
        .id("DEP-2")
        .end(date(2025, 7, 1))
        .quote_rate(0.03)
        .build();

    // Execute
    let pv1 = dep1.npv(&ctx, base).unwrap();
    let pv2 = dep2.npv(&ctx, base).unwrap();

    // Validate - should have identical PVs
    assert!((pv1.amount() - pv2.amount()).abs() < PRICE_TOLERANCE);
}

#[test]
fn test_rate_exactly_equal_to_par() {
    // Setup
    let base = date(2025, 1, 1);
    let ctx = ctx_with_standard_disc(base, "USD-OIS");
    let dep = DepositBuilder::new(base).end(date(2025, 7, 1)).build();

    // Execute - get par rate and use it
    let par = compute_metric(&dep, &ctx, base, MetricId::DepositParRate);

    let dep_at_par = DepositBuilder::new(base)
        .end(date(2025, 7, 1))
        .quote_rate(par)
        .build();

    let pv = dep_at_par.npv(&ctx, base).unwrap();

    // Validate - PV should be essentially zero for deposit at par rate
    // Market standard: < $0.01 on $1M notional (< 0.001bp)
    assert!(
        pv.amount().abs() < 0.01,
        "PV at par rate should be < $0.01, got: {}",
        pv.amount()
    );
}

#[test]
fn test_multiple_currencies_independent() {
    // Setup - test deposits in different currencies
    let base = date(2025, 1, 1);
    let ctx_usd = ctx_with_standard_disc(base, "USD-OIS");
    let ctx_eur = ctx_with_standard_disc(base, "EUR-OIS");

    let dep_usd = DepositBuilder::new(base)
        .notional(Money::new(1_000_000.0, Currency::USD))
        .end(date(2025, 7, 1))
        .quote_rate(0.03)
        .discount_curve_id("USD-OIS")
        .build();

    let dep_eur = DepositBuilder::new(base)
        .notional(Money::new(1_000_000.0, Currency::EUR))
        .end(date(2025, 7, 1))
        .quote_rate(0.03)
        .discount_curve_id("EUR-OIS")
        .build();

    // Execute
    let pv_usd = dep_usd.npv(&ctx_usd, base).unwrap();
    let pv_eur = dep_eur.npv(&ctx_eur, base).unwrap();

    // Validate - both should compute correctly
    assert_eq!(pv_usd.currency(), Currency::USD);
    assert_eq!(pv_eur.currency(), Currency::EUR);
}
