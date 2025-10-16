//! Par rate metric tests.

use crate::deposit::common::*;
use finstack_valuations::metrics::MetricId;

#[test]
fn test_par_rate_makes_pv_zero() {
    // Setup
    let base = date(2025, 1, 1);
    let ctx = ctx_with_standard_disc(base, "USD-OIS");
    let dep = DepositBuilder::new(base).end(date(2025, 7, 1)).build();

    // Execute - compute par rate
    let par_rate = compute_metric(&dep, &ctx, base, MetricId::DepositParRate);

    // Execute - price with par rate
    let dep_par = DepositBuilder::new(base)
        .end(date(2025, 7, 1))
        .quote_rate(par_rate)
        .build();

    let pv = dep_par.npv(&ctx, base).unwrap();

    // Validate - PV should be reasonably close to zero (within tolerance for numerical precision)
    assert!(
        pv.amount().abs() < 200.0,
        "PV not near zero: {}",
        pv.amount()
    );
}

#[test]
fn test_par_rate_formula_consistency() {
    // Setup
    let base = date(2025, 1, 1);
    let ctx = ctx_with_standard_disc(base, "USD-OIS");
    let dep = DepositBuilder::new(base).end(date(2025, 7, 1)).build();

    // Execute
    let metrics = compute_metrics(
        &dep,
        &ctx,
        base,
        &[
            MetricId::DfStart,
            MetricId::DfEnd,
            MetricId::Yf,
            MetricId::DepositParRate,
        ],
    );

    let df_s = metrics[&MetricId::DfStart];
    let df_e = metrics[&MetricId::DfEnd];
    let yf = metrics[&MetricId::Yf];
    let par = metrics[&MetricId::DepositParRate];

    // Validate - par = (DF(start) / DF(end) - 1) / yf
    let expected = (df_s / df_e - 1.0) / yf;
    assert!((par - expected).abs() < RATE_TOLERANCE);
}

#[test]
fn test_par_rate_positive_for_normal_curve() {
    // Setup
    let base = date(2025, 1, 1);
    let ctx = ctx_with_standard_disc(base, "USD-OIS");
    let dep = DepositBuilder::new(base).end(date(2025, 7, 1)).build();

    // Execute
    let par = compute_metric(&dep, &ctx, base, MetricId::DepositParRate);

    // Validate - par rate should be positive for upward sloping curve
    assert!(par > 0.0);
}

#[test]
fn test_par_rate_increases_with_maturity() {
    // Setup
    let base = date(2025, 1, 1);
    let ctx = ctx_with_standard_disc(base, "USD-OIS");

    let dep_short = DepositBuilder::new(base).end(date(2025, 4, 1)).build();

    let dep_long = DepositBuilder::new(base).end(date(2026, 1, 1)).build();

    // Execute
    let par_short = compute_metric(&dep_short, &ctx, base, MetricId::DepositParRate);
    let par_long = compute_metric(&dep_long, &ctx, base, MetricId::DepositParRate);

    // Validate - for normal curve, longer maturity has higher par rate
    assert!(par_long > par_short);
}

#[test]
fn test_par_rate_sensitivity_to_curve_steepness() {
    // Setup
    let base = date(2025, 1, 1);
    let ctx_flat = ctx_with_standard_disc(base, "USD-OIS");
    let ctx_steep = ctx_with_steep_curve(base, "USD-OIS");

    let dep = DepositBuilder::new(base).end(date(2026, 1, 1)).build();

    // Execute
    let par_flat = compute_metric(&dep, &ctx_flat, base, MetricId::DepositParRate);
    let par_steep = compute_metric(&dep, &ctx_steep, base, MetricId::DepositParRate);

    // Validate - steeper curve should give higher par rate
    assert!(par_steep > par_flat);
}

#[test]
fn test_par_rate_zero_for_zero_period() {
    // Setup
    let base = date(2025, 1, 1);
    let ctx = ctx_with_standard_disc(base, "USD-OIS");

    let dep = DepositBuilder::new(base).start(base).end(base).build();

    // Execute
    let par = compute_metric(&dep, &ctx, base, MetricId::DepositParRate);

    // Validate - zero period should give zero par rate
    assert!(par.abs() < RATE_TOLERANCE);
}

#[test]
fn test_par_rate_different_day_counts() {
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
    let par_360 = compute_metric(&dep_360, &ctx, base, MetricId::DepositParRate);
    let par_365 = compute_metric(&dep_365, &ctx, base, MetricId::DepositParRate);

    // Validate - different day counts give different par rates
    assert_ne!(par_360, par_365);
}
