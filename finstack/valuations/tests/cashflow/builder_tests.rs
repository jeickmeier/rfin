//! Tests for the cashflow builder state module.
//!
//! # Tolerance Conventions
//!
//! - `RATE_TOLERANCE` (1e-10): For rate/factor comparisons
//! - `FACTOR_TOLERANCE` (1e-12): For year fractions
//! - `financial_tolerance(notional)`: For money amounts

use crate::cashflow_tests::test_helpers::{financial_tolerance, FACTOR_TOLERANCE, RATE_TOLERANCE};
use finstack_core::cashflow::{CFKind, CashFlow};
use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, DayCount, StubKind, Tenor};
use finstack_core::dates::{Date, ScheduleBuilder};
use finstack_core::market_data::term_structures::DiscountCurve as CoreDiscCurve;
use finstack_core::math::interp::InterpStyle;
use finstack_core::money::Money;
use finstack_valuations::cashflow::builder::specs::{CouponType, FixedCouponSpec};
use finstack_valuations::cashflow::builder::{AmortizationSpec, CashFlowSchedule};
use finstack_valuations::instruments::common::discountable::Discountable;
use rust_decimal::Decimal;
use time::Month;

#[test]
fn linear_vs_step_parity() {
    let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let maturity = Date::from_calendar_date(2026, Month::January, 15).unwrap();

    let fixed = FixedCouponSpec {
        coupon_type: CouponType::Cash,
        rate: Decimal::try_from(0.05).expect("valid"),
        freq: Tenor::quarterly(),
        dc: DayCount::Act365F,
        bdc: BusinessDayConvention::Following,
        calendar_id: None,
        stub: StubKind::None,
    };

    let init = Money::new(1_000.0, Currency::USD);

    // Linear
    let mut b1 = CashFlowSchedule::builder();
    let _ = b1
        .principal(init, issue, maturity)
        .amortization(AmortizationSpec::LinearTo {
            final_notional: Money::new(0.0, Currency::USD),
        })
        .fixed_cf(fixed.clone());
    let s1 = b1.build_with_curves(None).unwrap();

    // Step schedule equivalent
    let sched: Vec<Date> = ScheduleBuilder::new(issue, maturity)
        .unwrap()
        .frequency(Tenor::quarterly())
        .build()
        .unwrap()
        .into_iter()
        .collect();
    let delta = init.amount() / (sched.len() - 1) as f64;
    let mut remaining = init.amount();
    let mut pairs: Vec<(Date, Money)> = Vec::new();
    for &d in sched.iter().skip(1) {
        remaining = (remaining - delta).max(0.0);
        pairs.push((d, Money::new(remaining, Currency::USD)));
    }

    let mut b2 = CashFlowSchedule::builder();
    let _ = b2
        .principal(init, issue, maturity)
        .amortization(AmortizationSpec::StepRemaining { schedule: pairs })
        .fixed_cf(fixed.clone());
    let s2 = b2.build_with_curves(None).unwrap();

    assert_eq!(s1.flows.len(), s2.flows.len());
    for (a, b) in s1.flows.iter().zip(s2.flows.iter()) {
        assert_eq!(a.date, b.date);
        assert_eq!(a.kind, b.kind);
        assert!(
            (a.amount.amount() - b.amount.amount()).abs() < financial_tolerance(init.amount()),
            "Flow amounts should match: {} vs {}",
            a.amount.amount(),
            b.amount.amount()
        );
    }
}

#[test]
fn pik_capitalization_increases_outstanding() {
    let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let maturity = Date::from_calendar_date(2026, Month::January, 15).unwrap();
    let init = Money::new(1_000.0, Currency::USD);

    let fixed = FixedCouponSpec {
        coupon_type: CouponType::PIK,
        rate: Decimal::try_from(0.10).expect("valid"),
        freq: Tenor::semi_annual(),
        dc: DayCount::Act365F,
        bdc: BusinessDayConvention::Following,
        calendar_id: None,
        stub: StubKind::None,
    };

    let mut b = CashFlowSchedule::builder();
    let _ = b.principal(init, issue, maturity).fixed_cf(fixed.clone());
    let s = b.build_with_curves(None).unwrap();
    let path = s.outstanding_path_per_flow().unwrap();
    // Find last outstanding before redemption
    let last_before = path
        .iter()
        .rev()
        .find(|(d, _)| *d < maturity)
        .unwrap()
        .1
        .amount();
    assert!(last_before > init.amount());
}

#[test]
fn ordering_invariants_within_date() {
    let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let maturity = Date::from_calendar_date(2025, Month::July, 15).unwrap();
    let init = Money::new(1_000.0, Currency::USD);
    let fixed = FixedCouponSpec {
        coupon_type: CouponType::Split {
            cash_pct: Decimal::try_from(0.5).expect("valid"),
            pik_pct: Decimal::try_from(0.5).expect("valid"),
        },
        rate: Decimal::try_from(0.10).expect("valid"),
        freq: Tenor::quarterly(),
        dc: DayCount::Act365F,
        bdc: BusinessDayConvention::Following,
        calendar_id: None,
        stub: StubKind::None,
    };

    // Percent-per-period amortization to force amort on coupon dates
    let mut b = CashFlowSchedule::builder();
    let _ = b
        .principal(init, issue, maturity)
        .amortization(AmortizationSpec::PercentPerPeriod { pct: 0.25 })
        .fixed_cf(fixed.clone());
    let s = b.build_with_curves(None).unwrap();

    // On coupon dates where multiple flows exist, enforce order: Fixed/Stub -> Amortization -> PIK -> Notional
    let mut by_date: finstack_core::HashMap<Date, Vec<CFKind>> = finstack_core::HashMap::default();
    for cf in &s.flows {
        by_date.entry(cf.date).or_default().push(cf.kind);
    }

    for (_d, kinds) in by_date {
        let mut sorted = kinds.clone();
        sorted.sort_by_key(|k| match k {
            CFKind::Fixed | CFKind::Stub | CFKind::FloatReset => 0,
            CFKind::Fee => 1,
            CFKind::Amortization => 2,
            CFKind::PIK => 3,
            CFKind::Notional => 4,
            _ => 5,
        });
        assert_eq!(kinds, sorted);
    }
}

#[test]
fn fixed_schedule_npv_equals_sum_cashflows() {
    let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let maturity = Date::from_calendar_date(2026, Month::January, 15).unwrap();

    let fixed = FixedCouponSpec {
        coupon_type: CouponType::Cash,
        rate: Decimal::try_from(0.05).expect("valid"),
        freq: Tenor::semi_annual(),
        dc: DayCount::Act365F,
        bdc: BusinessDayConvention::Following,
        calendar_id: None,
        stub: StubKind::None,
    };

    let init = Money::new(1_000_000.0, Currency::USD);

    let mut b = CashFlowSchedule::builder();
    let _ = b.principal(init, issue, maturity).fixed_cf(fixed.clone());
    let schedule = b.build_with_curves(None).unwrap();

    // Use a flat DF=1.0 curve for this test (testing NPV = sum when no discounting)
    // NOTE: Flat curves are not monotonically decreasing, so must allow_non_monotonic()
    let curve = CoreDiscCurve::builder("USD-OIS")
        .base_date(issue)
        .knots([(0.0, 1.0), (5.0, 1.0)])
        .set_interp(InterpStyle::Linear)
        .allow_non_monotonic() // Flat curve for testing NPV = sum of cashflows
        .build()
        .unwrap();

    let pv = schedule
        .npv(&curve, curve.base_date(), Some(schedule.day_count))
        .unwrap();

    // PV with flat curve DF=1.0 should equal sum of coupon amounts (no discounting)
    let expected = schedule
        .flows
        .iter()
        .fold(0.0, |sum, cf| sum + cf.amount.amount());
    assert!(
        (pv.amount() - expected).abs() < financial_tolerance(init.amount()),
        "PV should equal sum of cashflows: {} vs {}",
        pv.amount(),
        expected
    );
}

#[test]
fn detects_stub_periods() {
    let issue = Date::from_calendar_date(2025, Month::January, 10).unwrap(); // irregular
    let maturity = Date::from_calendar_date(2026, Month::January, 15).unwrap();

    let fixed = FixedCouponSpec {
        coupon_type: CouponType::Cash,
        rate: Decimal::try_from(0.04).expect("valid"),
        freq: Tenor::semi_annual(),
        dc: DayCount::Act365F,
        bdc: BusinessDayConvention::Following,
        calendar_id: None,
        stub: StubKind::None,
    };

    let init = Money::new(1_000_000.0, Currency::USD);

    let mut b = CashFlowSchedule::builder();
    let _ = b.principal(init, issue, maturity).fixed_cf(fixed.clone());
    let schedule = b.build_with_curves(None).unwrap();

    // Find coupon flows (not notional)
    let coupon_flows: Vec<&CashFlow> = schedule
        .flows
        .iter()
        .filter(|cf| cf.kind == CFKind::Fixed || cf.kind == CFKind::Stub)
        .collect();

    // At least one should be a stub due to irregular start date
    let has_stub = coupon_flows.iter().any(|cf| cf.kind == CFKind::Stub);
    assert!(
        has_stub,
        "Should detect stub period with irregular start date"
    );
}

#[test]
fn outstanding_by_date_dedup_and_values() {
    let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let maturity = Date::from_calendar_date(2025, Month::July, 15).unwrap();
    let init = Money::new(10_000.0, Currency::USD);

    // Force multiple flows per date: split coupon (cash + PIK) and amortization on coupon dates
    let fixed = FixedCouponSpec {
        coupon_type: CouponType::Split {
            cash_pct: Decimal::try_from(0.5).expect("valid"),
            pik_pct: Decimal::try_from(0.5).expect("valid"),
        },
        rate: Decimal::try_from(0.12).expect("valid"),
        freq: Tenor::quarterly(),
        dc: DayCount::Act365F,
        bdc: BusinessDayConvention::Following,
        calendar_id: None,
        stub: StubKind::None,
    };

    let mut b = CashFlowSchedule::builder();
    let _ = b
        .principal(init, issue, maturity)
        .amortization(AmortizationSpec::PercentPerPeriod { pct: 0.25 })
        .fixed_cf(fixed.clone());
    let s = b.build_with_curves(None).unwrap();

    let end_by_date = s.outstanding_by_date().unwrap();

    // 1) One entry per unique date
    let unique_dates: std::collections::BTreeSet<Date> = s.flows.iter().map(|cf| cf.date).collect();
    assert_eq!(end_by_date.len(), unique_dates.len());
    // Dates are ordered
    for ((d1, _), d2) in end_by_date.iter().zip(unique_dates.iter()) {
        assert_eq!(d1, d2);
    }

    // 2) Verify outstanding values are non-negative before maturity
    //    and zero at maturity (after redemption)
    for (i, (d, m)) in end_by_date.iter().enumerate() {
        assert!(
            m.amount() >= -0.01,
            "Outstanding should be non-negative, got {} at {:?}",
            m.amount(),
            d
        );

        // At maturity (last date), outstanding should be 0 after redemption
        if i == end_by_date.len() - 1 {
            assert!(
                m.amount().abs() < 0.01,
                "Outstanding at maturity should be 0 after redemption, got {}",
                m.amount()
            );
        }
    }

    // 3) At issue date, outstanding should be initial notional
    if let Some((d, m)) = end_by_date.first() {
        assert_eq!(*d, issue);
        assert!(
            (m.amount() - init.amount()).abs() < financial_tolerance(init.amount()),
            "Outstanding at issue should be initial notional {}, got {}",
            init.amount(),
            m.amount()
        );
    }

    // 4) Outstanding should decrease over time (due to amortization, despite PIK)
    //    with net decrease until maturity
    let first_outstanding = end_by_date.first().map(|(_, m)| m.amount()).unwrap_or(0.0);
    let last_outstanding = end_by_date.last().map(|(_, m)| m.amount()).unwrap_or(0.0);
    assert!(
        last_outstanding < first_outstanding,
        "Outstanding should decrease from {} to {} over the life",
        first_outstanding,
        last_outstanding
    );
}

#[test]
fn schedule_errors_on_unknown_calendar() {
    // Test that schedule generation propagates calendar lookup errors
    let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let maturity = Date::from_calendar_date(2026, Month::January, 15).unwrap();

    let fixed = FixedCouponSpec {
        coupon_type: CouponType::Cash,
        rate: Decimal::try_from(0.05).expect("valid"),
        freq: Tenor::semi_annual(),
        dc: DayCount::Act365F,
        bdc: BusinessDayConvention::Following,
        calendar_id: Some("UNKNOWN_CALENDAR_XYZ".to_string()),
        stub: StubKind::None,
    };

    let mut builder = CashFlowSchedule::builder();
    let _ = builder
        .principal(Money::new(1_000_000.0, Currency::USD), issue, maturity)
        .fixed_cf(fixed);

    let result = builder.build_with_curves(None);
    assert!(
        result.is_err(),
        "Schedule generation should error on unknown calendar"
    );
}

// =============================================================================
// Market Standards Review - Additional Day Count Convention Tests
// =============================================================================

#[test]
fn stub_period_thirty360_produces_proportional_accrual() {
    // Test that stub periods with 30/360 day count produce proportionally smaller accrued amounts
    // Market convention: 30/360 treats each month as 30 days and each year as 360 days
    let issue = Date::from_calendar_date(2025, Month::February, 10).unwrap(); // Irregular start (10th)
    let maturity = Date::from_calendar_date(2026, Month::February, 15).unwrap(); // Regular end (15th)

    let fixed = FixedCouponSpec {
        coupon_type: CouponType::Cash,
        rate: Decimal::try_from(0.06).expect("valid"), // 6% annual rate
        freq: Tenor::semi_annual(),
        dc: DayCount::Thirty360, // Market standard for corporate bonds
        bdc: BusinessDayConvention::Following,
        calendar_id: None,
        stub: StubKind::None,
    };

    let init = Money::new(1_000_000.0, Currency::USD);

    let mut b = CashFlowSchedule::builder();
    let _ = b.principal(init, issue, maturity).fixed_cf(fixed.clone());
    let schedule = b.build_with_curves(None).unwrap();

    // Find coupon flows only
    let coupon_flows: Vec<&CashFlow> = schedule
        .flows
        .iter()
        .filter(|cf| cf.kind == CFKind::Fixed || cf.kind == CFKind::Stub)
        .collect();

    // Should have at least one stub period
    let stubs: Vec<&&CashFlow> = coupon_flows
        .iter()
        .filter(|cf| cf.kind == CFKind::Stub)
        .collect();
    let regular: Vec<&&CashFlow> = coupon_flows
        .iter()
        .filter(|cf| cf.kind == CFKind::Fixed)
        .collect();

    assert!(!stubs.is_empty(), "Should have at least one stub period");
    assert!(
        !regular.is_empty(),
        "Should have at least one regular period"
    );

    // Regular semi-annual coupon should be approximately 3% of notional (6% / 2)
    // Stub period should be smaller due to shorter accrual period
    let regular_amount = regular[0].amount.amount();
    let stub_amount = stubs[0].amount.amount();

    // Regular should be close to 30,000 (1M * 6% * 0.5)
    // Using financial_tolerance for $1M notional (allows ~$10 variance)
    assert!(
        (regular_amount - 30_000.0).abs() < financial_tolerance(1_000_000.0),
        "Regular coupon should be ~30,000 ± ${:.2}, got {}",
        financial_tolerance(1_000_000.0),
        regular_amount
    );

    // Stub should be smaller than regular (shorter period)
    assert!(
        stub_amount < regular_amount,
        "Stub ({}) should be smaller than regular ({}) due to shorter period",
        stub_amount,
        regular_amount
    );
}

// =============================================================================
// Market Standards Review - PSA/SDA Golden Value Tests
// =============================================================================

#[test]
fn psa_smm_golden_values() {
    // PSA (Public Securities Association) Prepayment Model Golden Values
    // 100% PSA ramps to 6% CPR over 30 months, then stays flat
    // SMM = 1 - (1 - CPR)^(1/12)
    use finstack_valuations::cashflow::builder::credit_rates::{cpr_to_smm, smm_to_cpr};
    use finstack_valuations::cashflow::builder::PrepaymentModelSpec;

    let model = PrepaymentModelSpec::psa_100();

    // Month 0: 0% CPR → 0% SMM
    let smm_0 = model.smm(0);
    assert!(
        smm_0.abs() < RATE_TOLERANCE,
        "PSA at month 0 should be 0% SMM, got {}",
        smm_0
    );

    // Month 15: 3% CPR (halfway through ramp) → ~0.2536% SMM
    let smm_15 = model.smm(15);
    let cpr_15 = smm_to_cpr(smm_15);
    assert!(
        (cpr_15 - 0.03).abs() < RATE_TOLERANCE,
        "PSA at month 15 should be 3% CPR, got {}",
        cpr_15
    );

    // Month 30: 6% CPR (end of ramp) → ~0.5143% SMM
    let smm_30 = model.smm(30);
    let expected_smm_30 = cpr_to_smm(0.06);
    assert!(
        (smm_30 - expected_smm_30).abs() < RATE_TOLERANCE,
        "PSA at month 30 should be {} SMM, got {}",
        expected_smm_30,
        smm_30
    );

    // Month 60: Still 6% CPR (flat after ramp)
    let smm_60 = model.smm(60);
    assert!(
        (smm_60 - expected_smm_30).abs() < RATE_TOLERANCE,
        "PSA at month 60 should still be {} SMM, got {}",
        expected_smm_30,
        smm_60
    );

    // 150% PSA should be 1.5x the base values
    let model_150 = PrepaymentModelSpec::psa(1.5);
    let smm_30_150 = model_150.smm(30);
    let cpr_30_150 = smm_to_cpr(smm_30_150);
    assert!(
        (cpr_30_150 - 0.09).abs() < RATE_TOLERANCE,
        "150% PSA at month 30 should be 9% CPR, got {}",
        cpr_30_150
    );
}

#[test]
fn sda_mdr_golden_values() {
    // SDA (Standard Default Assumption) Model Golden Values
    // SDA peaks at month 30 with 6% CDR, then declines to 3% terminal over next 30 months
    use finstack_valuations::cashflow::builder::credit_rates::smm_to_cpr;
    use finstack_valuations::cashflow::builder::DefaultModelSpec;

    let model = DefaultModelSpec::sda(1.0);

    // Month 0: 0% CDR
    let mdr_0 = model.mdr(0);
    assert!(
        mdr_0.abs() < RATE_TOLERANCE,
        "SDA at month 0 should be 0% MDR, got {}",
        mdr_0
    );

    // Month 15: 3% CDR (halfway to peak)
    let mdr_15 = model.mdr(15);
    let cdr_15 = smm_to_cpr(mdr_15);
    assert!(
        (cdr_15 - 0.03).abs() < RATE_TOLERANCE,
        "SDA at month 15 should be 3% CDR, got {}",
        cdr_15
    );

    // Month 30: 6% CDR (peak)
    let mdr_30 = model.mdr(30);
    let cdr_30 = smm_to_cpr(mdr_30);
    assert!(
        (cdr_30 - 0.06).abs() < RATE_TOLERANCE,
        "SDA at month 30 should be 6% CDR (peak), got {}",
        cdr_30
    );

    // Month 60: 3% CDR (terminal, 30 months after peak)
    let mdr_60 = model.mdr(60);
    let cdr_60 = smm_to_cpr(mdr_60);
    assert!(
        (cdr_60 - 0.03).abs() < RATE_TOLERANCE,
        "SDA at month 60 should be 3% CDR (terminal), got {}",
        cdr_60
    );

    // Month 90: Still 3% CDR (flat after terminal)
    let mdr_90 = model.mdr(90);
    let cdr_90 = smm_to_cpr(mdr_90);
    assert!(
        (cdr_90 - 0.03).abs() < RATE_TOLERANCE,
        "SDA at month 90 should still be 3% CDR, got {}",
        cdr_90
    );
}

#[test]
fn cpr_smm_conversion_roundtrip_precision() {
    // Test that CPR ↔ SMM conversion maintains precision across range
    // Formula: SMM = 1 - (1 - CPR)^(1/12)
    //          CPR = 1 - (1 - SMM)^12
    use finstack_valuations::cashflow::builder::credit_rates::{cpr_to_smm, smm_to_cpr};

    let test_cprs = [0.0, 0.01, 0.03, 0.06, 0.10, 0.15, 0.20, 0.50];

    for &cpr in &test_cprs {
        let smm = cpr_to_smm(cpr);
        let cpr_back = smm_to_cpr(smm);

        assert!(
            (cpr - cpr_back).abs() < FACTOR_TOLERANCE,
            "CPR {} roundtrip failed: got {}",
            cpr,
            cpr_back
        );

        // SMM should always be less than CPR (except for 0)
        if cpr > 0.0 {
            assert!(smm < cpr, "SMM ({}) should be less than CPR ({})", smm, cpr);
        }
    }

    // Verify specific golden value: 6% CPR ≈ 0.5143% SMM
    // Using exact calculation: SMM = 1 - (1 - 0.06)^(1/12) ≈ 0.005143
    let smm_6pct = cpr_to_smm(0.06);
    let expected_smm = 1.0 - (1.0 - 0.06_f64).powf(1.0 / 12.0);
    assert!(
        (smm_6pct - expected_smm).abs() < FACTOR_TOLERANCE,
        "6% CPR should convert to {} SMM, got {}",
        expected_smm,
        smm_6pct
    );
}

// =============================================================================
// PSA/SDA Industry Standard Benchmark Tests
// =============================================================================

#[test]
fn psa_matches_industry_standard_ramp() {
    // Reference: Bond Market Association PSA Standard Prepayment Model
    // 100% PSA: Linear ramp from 0% CPR at month 0 to 6% CPR at month 30
    use finstack_valuations::cashflow::builder::credit_rates::smm_to_cpr;
    use finstack_valuations::cashflow::builder::PrepaymentModelSpec;

    let model = PrepaymentModelSpec::psa_100();

    // Month 1: 0.2% CPR (1/30 * 6%)
    let cpr_1 = smm_to_cpr(model.smm(1));
    assert!(
        (cpr_1 - 0.002).abs() < RATE_TOLERANCE,
        "PSA month 1 should be 0.2% CPR, got {}",
        cpr_1
    );

    // Month 10: 2.0% CPR (10/30 * 6%)
    let cpr_10 = smm_to_cpr(model.smm(10));
    assert!(
        (cpr_10 - 0.02).abs() < RATE_TOLERANCE,
        "PSA month 10 should be 2.0% CPR, got {}",
        cpr_10
    );

    // Month 20: 4.0% CPR (20/30 * 6%)
    let cpr_20 = smm_to_cpr(model.smm(20));
    assert!(
        (cpr_20 - 0.04).abs() < RATE_TOLERANCE,
        "PSA month 20 should be 4.0% CPR, got {}",
        cpr_20
    );

    // Verify ramp is linear for all months 1-30
    for month in 1..=30 {
        let expected_cpr = (month as f64 / 30.0) * 0.06;
        let actual_cpr = smm_to_cpr(model.smm(month));
        assert!(
            (actual_cpr - expected_cpr).abs() < RATE_TOLERANCE,
            "PSA month {} should be {:.4}% CPR, got {:.4}%",
            month,
            expected_cpr * 100.0,
            actual_cpr * 100.0
        );
    }
}

#[test]
fn psa_multiplier_scales_correctly() {
    // Test that PSA multipliers scale linearly
    use finstack_valuations::cashflow::builder::credit_rates::smm_to_cpr;
    use finstack_valuations::cashflow::builder::PrepaymentModelSpec;

    // 50% PSA, 100% PSA, 200% PSA at month 30
    let psa_50 = PrepaymentModelSpec::psa(0.5);
    let psa_100 = PrepaymentModelSpec::psa_100();
    let psa_200 = PrepaymentModelSpec::psa(2.0);

    let cpr_50 = smm_to_cpr(psa_50.smm(30));
    let cpr_100 = smm_to_cpr(psa_100.smm(30));
    let cpr_200 = smm_to_cpr(psa_200.smm(30));

    assert!(
        (cpr_50 - 0.03).abs() < RATE_TOLERANCE,
        "50% PSA at month 30 should be 3% CPR, got {}",
        cpr_50
    );
    assert!(
        (cpr_100 - 0.06).abs() < RATE_TOLERANCE,
        "100% PSA at month 30 should be 6% CPR, got {}",
        cpr_100
    );
    assert!(
        (cpr_200 - 0.12).abs() < RATE_TOLERANCE,
        "200% PSA at month 30 should be 12% CPR, got {}",
        cpr_200
    );

    // Verify linear scaling relationship
    assert!(
        (cpr_100 - 2.0 * cpr_50).abs() < RATE_TOLERANCE,
        "100% PSA should be 2x 50% PSA"
    );
    assert!(
        (cpr_200 - 2.0 * cpr_100).abs() < RATE_TOLERANCE,
        "200% PSA should be 2x 100% PSA"
    );
}

#[test]
fn psa_terminal_rate_is_flat() {
    // After month 30, PSA should stay flat at terminal rate
    use finstack_valuations::cashflow::builder::credit_rates::smm_to_cpr;
    use finstack_valuations::cashflow::builder::PrepaymentModelSpec;

    let model = PrepaymentModelSpec::psa_100();
    let terminal_cpr = 0.06;

    // Test various months after the ramp
    for month in [31, 50, 100, 200, 360] {
        let actual_cpr = smm_to_cpr(model.smm(month));
        assert!(
            (actual_cpr - terminal_cpr).abs() < RATE_TOLERANCE,
            "PSA month {} should be terminal 6% CPR, got {}",
            month,
            actual_cpr
        );
    }
}

#[test]
fn sda_matches_industry_standard_curve() {
    // Reference: Standard Default Assumption curve
    // Ramp to 6% CDR at month 30, decline to 3% CDR terminal by month 60
    use finstack_valuations::cashflow::builder::credit_rates::smm_to_cpr;
    use finstack_valuations::cashflow::builder::DefaultModelSpec;

    let model = DefaultModelSpec::sda(1.0);

    // Verify ramp phase (months 1-30)
    for month in 1..=30 {
        let expected_cdr = (month as f64 / 30.0) * 0.06;
        let actual_cdr = smm_to_cpr(model.mdr(month));
        assert!(
            (actual_cdr - expected_cdr).abs() < RATE_TOLERANCE,
            "SDA month {} (ramp) should be {:.4}% CDR, got {:.4}%",
            month,
            expected_cdr * 100.0,
            actual_cdr * 100.0
        );
    }

    // Verify decline phase (months 31-60)
    for month in 31..=60 {
        let months_past_peak = (month - 30) as f64;
        let expected_cdr = 0.06 - (months_past_peak / 30.0) * 0.03;
        let actual_cdr = smm_to_cpr(model.mdr(month));
        assert!(
            (actual_cdr - expected_cdr).abs() < RATE_TOLERANCE,
            "SDA month {} (decline) should be {:.4}% CDR, got {:.4}%",
            month,
            expected_cdr * 100.0,
            actual_cdr * 100.0
        );
    }

    // Verify terminal phase (month 61+)
    for month in [61, 100, 360] {
        let actual_cdr = smm_to_cpr(model.mdr(month));
        assert!(
            (actual_cdr - 0.03).abs() < RATE_TOLERANCE,
            "SDA month {} (terminal) should be 3% CDR, got {}",
            month,
            actual_cdr
        );
    }
}

#[test]
fn sda_multiplier_scales_correctly() {
    // Test that SDA multipliers scale linearly
    use finstack_valuations::cashflow::builder::credit_rates::smm_to_cpr;
    use finstack_valuations::cashflow::builder::DefaultModelSpec;

    let sda_100 = DefaultModelSpec::sda(1.0);
    let sda_200 = DefaultModelSpec::sda(2.0);

    // At peak (month 30)
    let cdr_100_peak = smm_to_cpr(sda_100.mdr(30));
    let cdr_200_peak = smm_to_cpr(sda_200.mdr(30));

    assert!(
        (cdr_100_peak - 0.06).abs() < RATE_TOLERANCE,
        "100% SDA peak should be 6% CDR"
    );
    assert!(
        (cdr_200_peak - 0.12).abs() < RATE_TOLERANCE,
        "200% SDA peak should be 12% CDR"
    );
    assert!(
        (cdr_200_peak - 2.0 * cdr_100_peak).abs() < RATE_TOLERANCE,
        "200% SDA should be 2x 100% SDA at peak"
    );

    // At terminal (month 90)
    let cdr_100_term = smm_to_cpr(sda_100.mdr(90));
    let cdr_200_term = smm_to_cpr(sda_200.mdr(90));

    assert!(
        (cdr_100_term - 0.03).abs() < RATE_TOLERANCE,
        "100% SDA terminal should be 3% CDR"
    );
    assert!(
        (cdr_200_term - 0.06).abs() < RATE_TOLERANCE,
        "200% SDA terminal should be 6% CDR"
    );
}

// =============================================================================
// Principal Events Validation Tests
// =============================================================================

#[test]
fn principal_events_after_maturity_rejected() {
    // Principal events after maturity should be rejected to prevent
    // post-maturity flows after outstanding has been zeroed out.
    use finstack_valuations::cashflow::builder::PrincipalEvent;

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
    use finstack_valuations::cashflow::builder::PrincipalEvent;

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
