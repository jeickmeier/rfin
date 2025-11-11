//! Market standards and benchmark validation tests.
//!
//! Tests against known market conventions and expected behaviors.

use crate::deposit::common::*;
use finstack_core::currency::Currency;
use finstack_core::dates::DayCount;
use finstack_core::money::Money;
use finstack_valuations::metrics::MetricId;

#[test]
fn test_standard_usd_3m_deposit() {
    // Setup - typical USD 3-month deposit
    let base = date(2025, 1, 1);
    let ctx = ctx_with_flat_rate(base, "USD-OIS", 0.02);

    let dep = DepositBuilder::new(base)
        .notional(Money::new(10_000_000.0, Currency::USD))
        .start(base)
        .end(date(2025, 4, 1))
        .day_count(DayCount::Act360)
        .quote_rate(0.02)
        .discount_curve_id("USD-OIS")
        .build();

    // Execute
    let pv = dep.npv(&ctx, base).unwrap();
    let par = compute_metric(&dep, &ctx, base, MetricId::DepositParRate);

    // Validate - at market rate, PV should be near zero
    assert!(pv.amount().abs() < 100_000.0, "PV: {}", pv.amount());
    assert!((par - 0.02).abs() < 0.005, "Par rate: {}", par);
}

#[test]
fn test_standard_eur_6m_deposit() {
    // Setup - typical EUR 6-month deposit
    let base = date(2025, 1, 1);
    let ctx = ctx_with_flat_rate(base, "EUR-OIS", 0.015);

    let dep = DepositBuilder::new(base)
        .notional(Money::new(10_000_000.0, Currency::EUR))
        .start(base)
        .end(date(2025, 7, 1))
        .day_count(DayCount::Act360)
        .quote_rate(0.015)
        .discount_curve_id("EUR-OIS")
        .build();

    // Execute
    let pv = dep.npv(&ctx, base).unwrap();

    // Validate
    assert!(pv.currency() == Currency::EUR);
    assert!(pv.amount().abs() < 100_000.0);
}

#[test]
fn test_overnight_deposit_libor_style() {
    // Setup - overnight deposit (O/N)
    let base = date(2025, 1, 1);
    let ctx = ctx_with_flat_rate(base, "USD-OIS", 0.05);

    let dep = DepositBuilder::new(base)
        .notional(Money::new(50_000_000.0, Currency::USD))
        .start(base)
        .end(date(2025, 1, 2))
        .day_count(DayCount::Act360)
        .quote_rate(0.05)
        .build();

    // Execute
    let pv = dep.npv(&ctx, base).unwrap();
    let yf = compute_metric(&dep, &ctx, base, MetricId::Yf);

    // Validate
    assert!(yf < 0.003, "Year fraction too large: {}", yf); // ~1/360
    assert!(pv.amount().is_finite());
}

#[test]
fn test_deposit_rate_convention_act360() {
    // Setup - Act/360 is standard for USD money market
    let base = date(2025, 1, 1);
    let ctx = ctx_with_standard_disc(base, "USD-OIS");

    let dep = DepositBuilder::new(base)
        .start(base)
        .end(date(2025, 7, 1))
        .day_count(DayCount::Act360)
        .quote_rate(0.03)
        .build();

    // Execute
    let yf = compute_metric(&dep, &ctx, base, MetricId::Yf);

    // Validate - 6 months with Act/360 should be ~0.5
    assert!(yf > 0.48 && yf < 0.52, "YF: {}", yf);
}

#[test]
fn test_par_rate_relationship_to_discount_factors() {
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

    // Validate - par = (DF(s)/DF(e) - 1) / yf
    let expected_par = (df_s / df_e - 1.0) / yf;
    assert!((par - expected_par).abs() < RATE_TOLERANCE);
}

#[test]
fn test_dv01_magnitude_check() {
    // Setup - $10mm 6M deposit should have DV01 magnitude ~$500
    let base = date(2025, 1, 1);
    let ctx = ctx_with_standard_disc(base, "USD-OIS");

    let dep = DepositBuilder::new(base)
        .notional(Money::new(10_000_000.0, Currency::USD))
        .end(date(2025, 7, 1))
        .build();

    // Execute
    let dv01 = compute_metric(&dep, &ctx, base, MetricId::Dv01);

    // Validate - rough magnitude check based on market standards (about 0.5 yrs * 10M notional * 1bp = ~$500)
    // DV01 is negative for long positions (standard convention)
    assert!(dv01.abs() > 400.0 && dv01.abs() < 600.0, "DV01: {}", dv01);
}

#[test]
fn test_simple_interest_calculation() {
    // Setup - verify simple interest formula
    let base = date(2025, 1, 1);
    let ctx = ctx_with_standard_disc(base, "USD-OIS");
    let notional = 1_000_000.0;
    let rate = 0.05;

    let dep = DepositBuilder::new(base)
        .notional(Money::new(notional, Currency::USD))
        .start(base)
        .end(date(2025, 7, 1))
        .quote_rate(rate)
        .build();

    // Execute
    let yf = compute_metric(&dep, &ctx, base, MetricId::Yf);

    // Validate - interest = notional × rate × year_fraction
    let expected_interest = notional * rate * yf;
    assert!(expected_interest > 20_000.0 && expected_interest < 30_000.0);
}

#[test]
fn test_multi_currency_portfolio() {
    // Setup - test deposits in multiple currencies
    let base = date(2025, 1, 1);
    let ctx_usd = ctx_with_standard_disc(base, "USD-OIS");
    let ctx_eur = ctx_with_standard_disc(base, "EUR-OIS");
    let ctx_gbp = ctx_with_standard_disc(base, "GBP-OIS");

    let dep_usd = DepositBuilder::new(base)
        .notional(Money::new(1_000_000.0, Currency::USD))
        .end(date(2025, 7, 1))
        .quote_rate(0.03)
        .discount_curve_id("USD-OIS")
        .build();

    let dep_eur = DepositBuilder::new(base)
        .notional(Money::new(1_000_000.0, Currency::EUR))
        .end(date(2025, 7, 1))
        .quote_rate(0.02)
        .discount_curve_id("EUR-OIS")
        .build();

    let dep_gbp = DepositBuilder::new(base)
        .notional(Money::new(1_000_000.0, Currency::GBP))
        .end(date(2025, 7, 1))
        .quote_rate(0.025)
        .discount_curve_id("GBP-OIS")
        .build();

    // Execute
    let pv_usd = dep_usd.npv(&ctx_usd, base).unwrap();
    let pv_eur = dep_eur.npv(&ctx_eur, base).unwrap();
    let pv_gbp = dep_gbp.npv(&ctx_gbp, base).unwrap();

    // Validate - each should price correctly in its own currency
    assert_eq!(pv_usd.currency(), Currency::USD);
    assert_eq!(pv_eur.currency(), Currency::EUR);
    assert_eq!(pv_gbp.currency(), Currency::GBP);
    assert!(pv_usd.amount().is_finite());
    assert!(pv_eur.amount().is_finite());
    assert!(pv_gbp.amount().is_finite());
}

#[test]
fn test_rate_quote_vs_price_quote() {
    // Setup - test that rate quoting works correctly
    let base = date(2025, 1, 1);
    let ctx = ctx_with_standard_disc(base, "USD-OIS");

    // Deposit quoted at rate
    let dep_rate = DepositBuilder::new(base)
        .end(date(2025, 7, 1))
        .quote_rate(0.035)
        .build();

    // Deposit at par rate
    let par = compute_metric(&dep_rate, &ctx, base, MetricId::DepositParRate);
    let dep_par = DepositBuilder::new(base)
        .end(date(2025, 7, 1))
        .quote_rate(par)
        .build();

    // Execute
    let pv_rate = dep_rate.npv(&ctx, base).unwrap();
    let pv_par = dep_par.npv(&ctx, base).unwrap();

    // Validate - par deposit should have reasonably small PV (within numerical precision)
    assert!(pv_par.amount().abs() < 200.0);
    // Off-market rate should have non-zero PV
    assert!(pv_rate.amount().abs() > 10.0);
}
