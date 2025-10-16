//! DV01 (dollar value of a basis point) metric tests.

use crate::deposit::common::*;
use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_valuations::metrics::MetricId;

#[test]
fn test_dv01_positive_for_deposits() {
    // Setup - deposits should have positive DV01 (long interest rate risk)
    let base = date(2025, 1, 1);
    let ctx = ctx_with_standard_disc(base, "USD-OIS");
    let dep = DepositBuilder::new(base).end(date(2025, 7, 1)).build();

    // Execute
    let dv01 = compute_metric(&dep, &ctx, base, MetricId::Dv01);

    // Validate
    assert!(dv01 > 0.0, "DV01 should be positive: {}", dv01);
}

#[test]
fn test_dv01_scales_with_notional() {
    // Setup
    let base = date(2025, 1, 1);
    let ctx = ctx_with_standard_disc(base, "USD-OIS");

    let dep_1m = DepositBuilder::new(base)
        .notional(Money::new(1_000_000.0, Currency::USD))
        .end(date(2025, 7, 1))
        .build();

    let dep_2m = DepositBuilder::new(base)
        .notional(Money::new(2_000_000.0, Currency::USD))
        .end(date(2025, 7, 1))
        .build();

    // Execute
    let dv01_1m = compute_metric(&dep_1m, &ctx, base, MetricId::Dv01);
    let dv01_2m = compute_metric(&dep_2m, &ctx, base, MetricId::Dv01);

    // Validate - should scale linearly
    assert!((dv01_2m / dv01_1m - 2.0).abs() < 0.01);
}

#[test]
fn test_dv01_increases_with_maturity() {
    // Setup
    let base = date(2025, 1, 1);
    let ctx = ctx_with_standard_disc(base, "USD-OIS");

    let dep_3m = DepositBuilder::new(base).end(date(2025, 4, 1)).build();

    let dep_1y = DepositBuilder::new(base).end(date(2026, 1, 1)).build();

    // Execute
    let dv01_3m = compute_metric(&dep_3m, &ctx, base, MetricId::Dv01);
    let dv01_1y = compute_metric(&dep_1y, &ctx, base, MetricId::Dv01);

    // Validate - longer maturity has higher DV01
    assert!(dv01_1y > dv01_3m);
}

#[test]
fn test_dv01_zero_for_zero_period() {
    // Setup
    let base = date(2025, 1, 1);
    let ctx = ctx_with_standard_disc(base, "USD-OIS");

    let dep = DepositBuilder::new(base).start(base).end(base).build();

    // Execute
    let dv01 = compute_metric(&dep, &ctx, base, MetricId::Dv01);

    // Validate
    assert!(dv01.abs() < 1e-10);
}

#[test]
fn test_dv01_zero_after_maturity() {
    // Setup - price after maturity
    let base = date(2025, 1, 1);
    let ctx = ctx_with_standard_disc(base, "USD-OIS");

    let dep = DepositBuilder::new(base)
        .start(date(2024, 1, 1))
        .end(date(2024, 7, 1))
        .build();

    // Execute
    let dv01 = compute_metric(&dep, &ctx, base, MetricId::Dv01);

    // Validate
    assert!(dv01.abs() < 1e-10);
}

#[test]
fn test_dv01_reasonable_magnitude() {
    // Setup - for $1mm notional, 6m deposit, DV01 should be ~$500
    let base = date(2025, 1, 1);
    let ctx = ctx_with_standard_disc(base, "USD-OIS");

    let dep = DepositBuilder::new(base)
        .notional(Money::new(1_000_000.0, Currency::USD))
        .end(date(2025, 7, 1))
        .build();

    // Execute
    let dv01 = compute_metric(&dep, &ctx, base, MetricId::Dv01);

    // Validate - rough magnitude check (about 0.5 yrs * 1M notional * 1bp = ~$50)
    assert!(dv01 > 40.0 && dv01 < 60.0, "DV01: {}", dv01);
}

#[test]
fn test_dv01_with_different_day_counts() {
    // Setup
    let base = date(2025, 1, 1);
    let ctx = ctx_with_standard_disc(base, "USD-OIS");

    let dep_360 = DepositBuilder::new(base)
        .end(date(2025, 7, 1))
        .day_count(finstack_core::dates::DayCount::Act360)
        .build();

    let dep_365 = DepositBuilder::new(base)
        .end(date(2025, 7, 1))
        .day_count(finstack_core::dates::DayCount::Act365F)
        .build();

    // Execute
    let dv01_360 = compute_metric(&dep_360, &ctx, base, MetricId::Dv01);
    let dv01_365 = compute_metric(&dep_365, &ctx, base, MetricId::Dv01);

    // Validate - different day counts give slightly different DV01
    assert_ne!(dv01_360, dv01_365);
}
