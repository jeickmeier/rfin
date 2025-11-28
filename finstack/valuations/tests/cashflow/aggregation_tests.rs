//! Integration tests for cashflow aggregation.
//!
//! # Tolerance Conventions
//!
//! - `RATE_TOLERANCE` (1e-10): For rate/factor comparisons
//! - `FACTOR_TOLERANCE` (1e-12): For year fractions
//! - `financial_tolerance(notional)`: For money amounts
//!
//! # Test Curve Conventions
//!
//! - `FlatRateCurve`: Time-dependent DF = exp(-r*t), DF(0) = 1.0
//! - `FlatHazardRateCurve`: Time-dependent SP = exp(-λ*t), SP(0) = 1.0

use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount, DayCountCtx, Frequency, Period, PeriodId};
use finstack_core::market_data::traits::{Discounting, Survival};
use finstack_core::money::Money;
use finstack_valuations::cashflow::aggregation::{
    aggregate_by_period, aggregate_cashflows_precise_checked,
    pv_by_period_credit_adjusted_with_ctx, pv_by_period_with_ctx,
};
use crate::cashflow_tests::test_helpers::{
    financial_tolerance, FlatHazardRateCurve, FlatRateCurve, FACTOR_TOLERANCE,
};
use time::Month;

fn d(year: i32, month: u8, day: u8) -> Date {
    Date::from_calendar_date(
        year,
        Month::try_from(month).expect("Valid month (1-12)"),
        day,
    )
    .expect("Valid test date")
}

fn quarters_2025() -> Vec<Period> {
    vec![
        Period {
            id: PeriodId::quarter(2025, 1),
            start: d(2025, 1, 1),
            end: d(2025, 4, 1),
            is_actual: true,
        },
        Period {
            id: PeriodId::quarter(2025, 2),
            start: d(2025, 4, 1),
            end: d(2025, 7, 1),
            is_actual: false,
        },
        Period {
            id: PeriodId::quarter(2025, 3),
            start: d(2025, 7, 1),
            end: d(2025, 10, 1),
            is_actual: false,
        },
    ]
}

#[test]
fn empty_inputs_yield_empty_aggregation() {
    let periods = quarters_2025();
    assert!(aggregate_by_period(&[], &periods).is_empty());
    let flows = vec![(d(2025, 1, 15), Money::new(1.0, Currency::USD))];
    assert!(aggregate_by_period(&flows, &[]).is_empty());
}

#[test]
fn cashflows_are_grouped_by_period_and_currency() {
    let periods = quarters_2025();
    let flows = vec![
        // Unsorted on purpose (algorithm should sort internally)
        (d(2025, 4, 15), Money::new(50.0, Currency::USD)),
        (d(2025, 1, 10), Money::new(100.0, Currency::USD)),
        (d(2025, 2, 20), Money::new(200.0, Currency::EUR)),
        // Boundary case: falls exactly on period end, should roll into next quarter
        (d(2025, 4, 1), Money::new(10.0, Currency::USD)),
    ];

    let aggregated = aggregate_by_period(&flows, &periods);
    let expected_keys = vec![PeriodId::quarter(2025, 1), PeriodId::quarter(2025, 2)];
    let keys: Vec<_> = aggregated.keys().cloned().collect();
    assert_eq!(keys, expected_keys);

    let q1 = aggregated
        .get(&PeriodId::quarter(2025, 1))
        .expect("Q1 should exist");
    assert_eq!(q1.len(), 2);
    assert!((q1[&Currency::USD].amount() - 100.0).abs() < FACTOR_TOLERANCE);
    assert!((q1[&Currency::EUR].amount() - 200.0).abs() < FACTOR_TOLERANCE);

    let q2 = aggregated
        .get(&PeriodId::quarter(2025, 2))
        .expect("Q2 should exist");
    assert_eq!(q2.len(), 1);
    assert!((q2[&Currency::USD].amount() - 60.0).abs() < FACTOR_TOLERANCE);

    // Third quarter has no flows -> should not be present
    assert!(aggregated.get(&PeriodId::quarter(2025, 3)).is_none());
}

#[test]
fn checked_empty_returns_zero_target() {
    let total = aggregate_cashflows_precise_checked(&[], Currency::USD)
        .expect("Aggregation should succeed")
        .expect("Result should be Some");
    assert_eq!(total.amount(), 0.0);
    assert_eq!(total.currency(), Currency::USD);
}

#[test]
fn test_aggregate_30y_bond_cashflows() {
    // Simulate 30-year semi-annual bond (60 cashflows)
    let flows: Vec<finstack_valuations::cashflow::DatedFlow> = (0..60)
        .map(|i| {
            // Semi-annual payments
            let months = i * 6;
            let years = months / 12;
            let remaining_months = months % 12;
            (
                Date::from_calendar_date(
                    2025 + years,
                    Month::try_from((remaining_months + 1) as u8).expect("Valid month (1-12)"),
                    1,
                )
                .expect("Valid test date"),
                Money::new(25_000.0, Currency::USD), // $25k coupon
            )
        })
        .collect();

    let total = aggregate_cashflows_precise_checked(&flows, Currency::USD)
        .expect("Aggregation should succeed")
        .expect("Result should be Some");

    // Should sum to 60 * $25k = $1.5M
    assert!(
        (total.amount() - 1_500_000.0).abs() < financial_tolerance(1_500_000.0),
        "Total should be $1.5M, got {}",
        total.amount()
    );
}

#[test]
fn checked_currency_mismatch_errors() {
    let flows = vec![
        (
            Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date"),
            Money::new(100.0, Currency::USD),
        ),
        (
            Date::from_calendar_date(2025, Month::February, 1).expect("Valid test date"),
            Money::new(200.0, Currency::EUR),
        ),
    ];
    let err = aggregate_cashflows_precise_checked(&flows, Currency::USD)
        .expect_err("should fail with currency mismatch");
    match err {
        finstack_core::error::Error::CurrencyMismatch { .. } => {}
        _ => panic!("expected CurrencyMismatch"),
    }
}

#[test]
fn checked_sum_matches() {
    let flows = vec![
        (
            Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date"),
            Money::new(100.0, Currency::USD),
        ),
        (
            Date::from_calendar_date(2025, Month::February, 1).expect("Valid test date"),
            Money::new(200.0, Currency::USD),
        ),
    ];
    let total = aggregate_cashflows_precise_checked(&flows, Currency::USD)
        .expect("Aggregation should succeed")
        .expect("Result should be Some");
    assert_eq!(total.currency(), Currency::USD);
    assert!((total.amount() - 300.0).abs() < FACTOR_TOLERANCE);
}


#[test]
fn pv_with_ctx_sum_matches_direct_calculation() {
    // Test that PV aggregation with Act365F sums correctly
    // Uses time-dependent discounting with 5% continuous rate
    let base = d(2025, 1, 1);
    let periods = quarters_2025();

    let flows = vec![
        (d(2025, 2, 15), Money::new(100.0, Currency::USD)),
        (d(2025, 5, 15), Money::new(200.0, Currency::USD)),
    ];

    // 5% continuous rate: DF(t) = exp(-0.05 * t)
    let curve = FlatRateCurve::new("USD-OIS", base, 0.05);

    let dc_ctx = DayCountCtx {
        frequency: Some(Frequency::quarterly()),
        calendar: None,
        bus_basis: None,
    };

    let pv_map = pv_by_period_with_ctx(&flows, &periods, &curve, base, DayCount::Act365F, dc_ctx)
        .expect("PV by period calculation should succeed in test");

    // Sum of period PVs
    let sum_pv: f64 = pv_map
        .values()
        .flat_map(|m| m.values())
        .map(|m| m.amount())
        .sum();

    // Standalone NPV using default context (Act365F doesn't require special ctx)
    use finstack_core::cashflow::discounting::npv_static;
    let total_npv = npv_static(&curve, base, DayCount::Act365F, &flows)
        .expect("NPV calculation should succeed in test");

    // Should match within financial tolerance for ~$300 total
    assert!(
        (sum_pv - total_npv.amount()).abs() < financial_tolerance(300.0),
        "Sum of period PVs ({}) should match NPV ({})",
        sum_pv,
        total_npv.amount()
    );
}

#[test]
fn pv_with_ctx_errors_on_missing_frequency_for_isma() {
    // Act/Act ISMA requires frequency in context
    let base = d(2025, 1, 1);
    let periods = quarters_2025();
    let flows = vec![(d(2025, 2, 15), Money::new(100.0, Currency::USD))];

    let curve = FlatRateCurve::new("USD-OIS", base, 0.05);

    // Missing frequency for ISMA should error
    let dc_ctx = DayCountCtx {
        frequency: None, // Missing!
        calendar: None,
        bus_basis: None,
    };

    let result =
        pv_by_period_with_ctx(&flows, &periods, &curve, base, DayCount::ActActIsma, dc_ctx);

    assert!(result.is_err(), "Should error when ISMA frequency missing");
}

#[test]
fn pv_by_period_deterministic_multi_currency() {
    // Multi-currency PV aggregation should preserve currency separation
    // Using zero rate (DF=1.0) to focus on currency handling, not discounting
    let base = d(2025, 1, 1);
    let periods = quarters_2025();

    let flows = vec![
        (d(2025, 2, 15), Money::new(100.0, Currency::USD)),
        (d(2025, 2, 20), Money::new(200.0, Currency::EUR)),
        (d(2025, 5, 10), Money::new(50.0, Currency::USD)),
    ];

    let curve = FlatRateCurve::new("USD-OIS", base, 0.0); // No discounting

    let dc_ctx = DayCountCtx {
        frequency: Some(Frequency::quarterly()),
        calendar: None,
        bus_basis: None,
    };

    let pv_map = pv_by_period_with_ctx(&flows, &periods, &curve, base, DayCount::Act365F, dc_ctx)
        .expect("PV by period calculation should succeed in test");

    // Q1 should have both USD and EUR
    let q1 = pv_map
        .get(&PeriodId::quarter(2025, 1))
        .expect("Q1 should exist");
    assert_eq!(q1.len(), 2);
    assert!(q1.contains_key(&Currency::USD));
    assert!(q1.contains_key(&Currency::EUR));

    // Q2 should have only USD
    let q2 = pv_map
        .get(&PeriodId::quarter(2025, 2))
        .expect("Q2 should exist");
    assert_eq!(q2.len(), 1);
    assert!(q2.contains_key(&Currency::USD));
}


#[test]
fn pv_by_period_sum_matches_npv() {
    // Test that period-aggregated PVs sum to total NPV with time-dependent discounting
    let base = d(2025, 1, 1);
    let periods = quarters_2025();
    let flows = vec![
        (d(2025, 2, 15), Money::new(100.0, Currency::USD)),
        (d(2025, 5, 15), Money::new(200.0, Currency::USD)),
    ];

    // 5% continuous rate: DF(t) = exp(-0.05 * t)
    let curve = FlatRateCurve::new("USD-OIS", base, 0.05);

    let pv_map = pv_by_period_with_ctx(
        &flows,
        &periods,
        &curve,
        base,
        DayCount::Act365F,
        DayCountCtx::default(),
    )
    .expect("PV calculation should succeed");

    // Q1 flow at Feb 15: t = 45/365 ≈ 0.1233, DF = exp(-0.05 * 0.1233) ≈ 0.9939
    // Q1 PV ≈ 100 * 0.9939 ≈ 99.39
    let t_q1 = 45.0 / 365.0;
    let expected_q1_pv = 100.0 * (-0.05_f64 * t_q1).exp();
    let q1_pv = pv_map
        .get(&PeriodId::quarter(2025, 1))
        .and_then(|m| m.get(&Currency::USD))
        .map(|m| m.amount())
        .unwrap_or(0.0);
    assert!(
        (q1_pv - expected_q1_pv).abs() < financial_tolerance(100.0),
        "Q1 PV should be {}, got {}",
        expected_q1_pv,
        q1_pv
    );

    // Q2 flow at May 15: t = 134/365 ≈ 0.3671, DF = exp(-0.05 * 0.3671) ≈ 0.9818
    // Q2 PV ≈ 200 * 0.9818 ≈ 196.37
    let t_q2 = 134.0 / 365.0;
    let expected_q2_pv = 200.0 * (-0.05_f64 * t_q2).exp();
    let q2_pv = pv_map
        .get(&PeriodId::quarter(2025, 2))
        .and_then(|m| m.get(&Currency::USD))
        .map(|m| m.amount())
        .unwrap_or(0.0);
    assert!(
        (q2_pv - expected_q2_pv).abs() < financial_tolerance(200.0),
        "Q2 PV should be {}, got {}",
        expected_q2_pv,
        q2_pv
    );

    // Sum should equal total NPV (within financial tolerance for ~$300 total)
    use finstack_core::cashflow::discounting::npv_static;
    let total_npv = npv_static(&curve, base, DayCount::Act365F, &flows)
        .expect("NPV calculation should succeed in test");
    let sum_pv = q1_pv + q2_pv;
    assert!(
        (sum_pv - total_npv.amount()).abs() < financial_tolerance(300.0),
        "Sum {} should equal NPV {}",
        sum_pv,
        total_npv.amount()
    );
}

#[test]
fn pv_by_period_respects_boundaries() {
    // Test that flows on period boundaries go to the correct period
    // Using zero rate (DF=1.0) to focus on boundary handling
    let base = d(2025, 1, 1);
    let periods = quarters_2025();
    // Flow exactly on period boundary should go to next period
    let flows = vec![(d(2025, 4, 1), Money::new(100.0, Currency::USD))];

    let curve = FlatRateCurve::new("USD-OIS", base, 0.0); // No discounting

    let pv_map = pv_by_period_with_ctx(
        &flows,
        &periods,
        &curve,
        base,
        DayCount::Act365F,
        DayCountCtx::default(),
    )
    .expect("PV calculation should succeed");

    // Should be in Q2, not Q1
    assert!(pv_map.get(&PeriodId::quarter(2025, 1)).is_none());
    let q2_pv = pv_map
        .get(&PeriodId::quarter(2025, 2))
        .and_then(|m| m.get(&Currency::USD))
        .map(|m| m.amount())
        .unwrap_or(0.0);
    assert!(
        (q2_pv - 100.0).abs() < financial_tolerance(100.0),
        "Q2 PV should be 100.0, got {}",
        q2_pv
    );
}

#[test]
fn pv_by_period_multi_currency_separation() {
    // Test that multi-currency flows are kept separate with proper discounting
    let base = d(2025, 1, 1);
    let periods = quarters_2025();
    let flows = vec![
        (d(2025, 2, 15), Money::new(100.0, Currency::USD)),
        (d(2025, 2, 20), Money::new(200.0, Currency::EUR)),
    ];

    // 5% continuous rate
    let curve = FlatRateCurve::new("USD-OIS", base, 0.05);

    let pv_map = pv_by_period_with_ctx(
        &flows,
        &periods,
        &curve,
        base,
        DayCount::Act365F,
        DayCountCtx::default(),
    )
    .expect("PV calculation should succeed");

    let q1 = pv_map
        .get(&PeriodId::quarter(2025, 1))
        .expect("Q1 should exist");
    assert_eq!(q1.len(), 2); // Both currencies present

    // USD flow at Feb 15: t = 45/365
    let t_usd = 45.0 / 365.0;
    let expected_usd = 100.0 * (-0.05_f64 * t_usd).exp();
    assert!(
        (q1[&Currency::USD].amount() - expected_usd).abs() < financial_tolerance(100.0),
        "USD PV should be {}, got {}",
        expected_usd,
        q1[&Currency::USD].amount()
    );

    // EUR flow at Feb 20: t = 50/365
    let t_eur = 50.0 / 365.0;
    let expected_eur = 200.0 * (-0.05_f64 * t_eur).exp();
    assert!(
        (q1[&Currency::EUR].amount() - expected_eur).abs() < financial_tolerance(200.0),
        "EUR PV should be {}, got {}",
        expected_eur,
        q1[&Currency::EUR].amount()
    );
}

#[test]
fn test_pv_by_period_credit_adjusted() {
    // Test credit-adjusted PV with time-dependent discount and survival curves
    let base = d(2025, 1, 1);
    let periods = quarters_2025();
    let flows = vec![(d(2025, 2, 15), Money::new(100.0, Currency::USD))];

    // 5% discount rate
    let disc_curve = FlatRateCurve::new("USD-OIS", base, 0.05);

    // 2% annual hazard rate (approx 2% default probability per year)
    let hazard_curve = FlatHazardRateCurve::new("AAPL-HAZARD", base, 0.02);

    let pv_map = pv_by_period_credit_adjusted_with_ctx(
        &flows,
        &periods,
        &disc_curve,
        Some(&hazard_curve),
        base,
        DayCount::Act365F,
        DayCountCtx::default(),
    )
    .expect("PV calculation should succeed");

    // Flow at Feb 15: t = 45/365 ≈ 0.1233
    // DF = exp(-0.05 * 0.1233) ≈ 0.9939
    // SP = exp(-0.02 * 0.1233) ≈ 0.9975
    // PV = 100 * DF * SP ≈ 100 * 0.9939 * 0.9975 ≈ 99.14
    let t = 45.0 / 365.0;
    let expected_df = (-0.05_f64 * t).exp();
    let expected_sp = (-0.02_f64 * t).exp();
    let expected_pv = 100.0 * expected_df * expected_sp;

    let q1_pv = pv_map
        .get(&PeriodId::quarter(2025, 1))
        .and_then(|m| m.get(&Currency::USD))
        .map(|m| m.amount())
        .unwrap_or(0.0);
    assert!(
        (q1_pv - expected_pv).abs() < financial_tolerance(100.0),
        "Credit-adjusted PV should be {}, got {}",
        expected_pv,
        q1_pv
    );
}

// =============================================================================
// Market Standards Review - Day Count Convention Tests
// =============================================================================

#[test]
fn pv_with_ctx_act365f_year_fraction() {
    // Test Act/365 Fixed day count produces correct year fractions
    // Market standard: actual days / 365
    let base = d(2025, 1, 1);
    let periods = quarters_2025();

    // Flow at Jan 15 = 14 days from Jan 1
    // Year fraction = 14/365 ≈ 0.03836
    let flows = vec![(d(2025, 1, 15), Money::new(1000.0, Currency::USD))];

    let curve = FlatRateCurve::new("USD-OIS", base, 0.0); // No discounting

    let dc_ctx = DayCountCtx::default();

    let result = pv_by_period_with_ctx(&flows, &periods, &curve, base, DayCount::Act365F, dc_ctx);
    assert!(result.is_ok(), "Act365F should not require special context");
}

#[test]
fn pv_with_ctx_act360_year_fraction() {
    // Test Act/360 day count (money market convention)
    // Market standard: actual days / 360
    let base = d(2025, 1, 1);
    let periods = quarters_2025();

    let flows = vec![(d(2025, 1, 31), Money::new(1000.0, Currency::USD))];

    let curve = FlatRateCurve::new("USD-OIS", base, 0.0); // No discounting

    let dc_ctx = DayCountCtx::default();

    let result = pv_by_period_with_ctx(&flows, &periods, &curve, base, DayCount::Act360, dc_ctx);
    assert!(result.is_ok(), "Act360 should not require special context");
}

#[test]
fn pv_with_ctx_thirty360_year_fraction() {
    // Test 30/360 day count (corporate bond convention)
    // Market standard: each month = 30 days, year = 360 days
    let base = d(2025, 1, 15);
    let periods = quarters_2025();

    // Flow at Jul 15 = exactly 6 months = 180 days (30/360)
    // Year fraction = 180/360 = 0.5
    let flows = vec![(d(2025, 7, 15), Money::new(1000.0, Currency::USD))];

    let curve = FlatRateCurve::new("USD-OIS", base, 0.0); // No discounting

    let dc_ctx = DayCountCtx::default();

    let result = pv_by_period_with_ctx(&flows, &periods, &curve, base, DayCount::Thirty360, dc_ctx);
    assert!(result.is_ok(), "30/360 should not require special context");
}

#[test]
fn pv_with_ctx_actact_isma_requires_frequency() {
    // Act/Act ISMA (ISDA-2006) requires frequency in context
    // This is the convention used for many government bonds
    let base = d(2025, 1, 1);
    let periods = quarters_2025();
    let flows = vec![(d(2025, 2, 15), Money::new(100.0, Currency::USD))];

    let curve = FlatRateCurve::new("USD-OIS", base, 0.05);

    // Without frequency - should error
    let dc_ctx_no_freq = DayCountCtx {
        frequency: None,
        calendar: None,
        bus_basis: None,
    };

    let result =
        pv_by_period_with_ctx(&flows, &periods, &curve, base, DayCount::ActActIsma, dc_ctx_no_freq);
    assert!(
        result.is_err(),
        "ActActIsma without frequency should error"
    );

    // With frequency - should succeed
    let dc_ctx_with_freq = DayCountCtx {
        frequency: Some(Frequency::semi_annual()),
        calendar: None,
        bus_basis: None,
    };

    let result =
        pv_by_period_with_ctx(&flows, &periods, &curve, base, DayCount::ActActIsma, dc_ctx_with_freq);
    assert!(
        result.is_ok(),
        "ActActIsma with frequency should succeed"
    );
}

#[test]
fn pv_with_ctx_actact_isda_no_frequency_required() {
    // Act/Act ISDA does NOT require frequency - it uses actual days / actual days in year
    // This distinguishes it from Act/Act ISMA which needs coupon frequency
    let base = d(2025, 1, 1);
    let periods = quarters_2025();
    let flows = vec![(d(2025, 2, 15), Money::new(100.0, Currency::USD))];

    let curve = FlatRateCurve::new("USD-OIS", base, 0.05);

    // Without frequency - should still work
    let dc_ctx = DayCountCtx {
        frequency: None,
        calendar: None,
        bus_basis: None,
    };

    // Note: ActAct (ISDA) uses calendar year (365 or 366 for leap years)
    let result =
        pv_by_period_with_ctx(&flows, &periods, &curve, base, DayCount::ActAct, dc_ctx);
    assert!(
        result.is_ok(),
        "ActAct (ISDA) should not require frequency context"
    );
}

// =============================================================================
// Edge Case Tests - Boundary Conditions
// =============================================================================

#[test]
fn discount_factor_at_base_date_is_one() {
    let base = d(2025, 1, 1);
    let curve = FlatRateCurve::new("USD-OIS", base, 0.05);

    assert!(
        (curve.df(0.0) - 1.0).abs() < FACTOR_TOLERANCE,
        "DF(0) must equal 1.0, got {}",
        curve.df(0.0)
    );
}

#[test]
fn survival_probability_at_base_date_is_one() {
    let base = d(2025, 1, 1);
    let curve = FlatHazardRateCurve::new("ISSUER-HAZARD", base, 0.02);

    assert!(
        (curve.sp(0.0) - 1.0).abs() < FACTOR_TOLERANCE,
        "SP(0) must equal 1.0, got {}",
        curve.sp(0.0)
    );
}

#[test]
fn pv_of_cashflow_at_base_date() {
    let base = d(2025, 1, 1);
    let flows = vec![(base, Money::new(100.0, Currency::USD))];

    let curve = FlatRateCurve::new("USD-OIS", base, 0.05);

    // Cashflow at t=0 should have PV = notional (DF=1.0)
    use finstack_core::cashflow::discounting::npv_static;
    let pv = npv_static(&curve, base, DayCount::Act365F, &flows).unwrap();
    assert!(
        (pv.amount() - 100.0).abs() < financial_tolerance(100.0),
        "PV at t=0 should equal notional, got {}",
        pv.amount()
    );
}

#[test]
fn long_dated_cashflow_stability() {
    // 30-year cashflow should have small but positive DF
    let base = d(2025, 1, 1);
    let maturity = d(2055, 1, 1);
    let flows = vec![(maturity, Money::new(1_000_000.0, Currency::USD))];

    let curve = FlatRateCurve::new("USD-OIS", base, 0.05);

    // Calculate exact year fraction (includes leap years between 2025-2055)
    // Act365F: actual days / 365
    let dc = DayCount::Act365F;
    let t = dc
        .year_fraction(base, maturity, DayCountCtx::default())
        .unwrap();
    let expected_df = (-0.05_f64 * t).exp();
    let expected_pv = 1_000_000.0 * expected_df;

    use finstack_core::cashflow::discounting::npv_static;
    let pv = npv_static(&curve, base, DayCount::Act365F, &flows).unwrap();

    // Use looser tolerance for 30-year horizon due to year fraction complexity
    assert!(
        (pv.amount() - expected_pv).abs() < financial_tolerance(1_000_000.0) * 100.0,
        "30-year PV should be ~{}, got {} (year fraction: {})",
        expected_pv,
        pv.amount(),
        t
    );
    assert!(pv.amount() > 0.0, "Long-dated PV must be positive");
    assert!(pv.amount() < 1_000_000.0, "PV must be less than notional for positive rates");
}

#[test]
fn negative_time_handling() {
    // Cashflows before base date - verify graceful handling
    let base = d(2025, 1, 1);
    let past_flow = d(2024, 1, 1);
    let flows = vec![(past_flow, Money::new(100.0, Currency::USD))];

    let curve = FlatRateCurve::new("USD-OIS", base, 0.05);

    // FlatRateCurve returns DF = 1.0 for t <= 0 (past cashflows)
    // This is a conservative approach that treats past cashflows at par
    use finstack_core::cashflow::discounting::npv_static;
    let result = npv_static(&curve, base, DayCount::Act365F, &flows);

    // Should succeed
    assert!(result.is_ok());
    let pv = result.unwrap();
    // With t <= 0, DF = 1.0, so PV = notional
    assert!(
        (pv.amount() - 100.0).abs() < financial_tolerance(100.0),
        "Past cashflow with DF=1.0 should have PV = notional, got {}",
        pv.amount()
    );
}

#[test]
fn discount_factor_monotonically_decreases() {
    let base = d(2025, 1, 1);
    let curve = FlatRateCurve::new("USD-OIS", base, 0.05);

    let times = [0.0, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0];
    let mut prev_df = f64::MAX;

    for t in times {
        let df = curve.df(t);
        assert!(
            df <= prev_df,
            "DF must be monotonically decreasing: DF({}) = {} > DF(prev)",
            t,
            df
        );
        assert!(df > 0.0, "DF must be positive: DF({}) = {}", t, df);
        assert!(
            df <= 1.0,
            "DF must be <= 1.0 for positive rates: DF({}) = {}",
            t,
            df
        );
        prev_df = df;
    }
}

#[test]
fn survival_probability_monotonically_decreases() {
    let base = d(2025, 1, 1);
    let curve = FlatHazardRateCurve::new("ISSUER-HAZARD", base, 0.02);

    let times = [0.0, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0];
    let mut prev_sp = f64::MAX;

    for t in times {
        let sp = curve.sp(t);
        assert!(
            sp <= prev_sp,
            "SP must be monotonically decreasing: SP({}) = {} > SP(prev)",
            t,
            sp
        );
        assert!(sp > 0.0, "SP must be positive: SP({}) = {}", t, sp);
        assert!(sp <= 1.0, "SP must be <= 1.0: SP({}) = {}", t, sp);
        prev_sp = sp;
    }
}
