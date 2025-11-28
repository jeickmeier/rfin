//! Tests for the cashflow builder state module.
//!
//! # Tolerance Conventions
//!
//! - `RATE_TOLERANCE` (1e-10): For rate/factor comparisons
//! - `FACTOR_TOLERANCE` (1e-12): For year fractions
//! - `financial_tolerance(notional)`: For money amounts

use finstack_core::cashflow::primitives::{CFKind, CashFlow};
use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, DayCount, Frequency, StubKind};
use finstack_core::dates::{Date, ScheduleBuilder};
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve as CoreDiscCurve;
use finstack_core::math::interp::InterpStyle;
use finstack_core::money::Money;
use finstack_valuations::cashflow::builder::specs::{CouponType, FixedCouponSpec};
use finstack_valuations::cashflow::builder::{AmortizationSpec, CashFlowSchedule};
use finstack_valuations::instruments::common::discountable::Discountable;
use crate::cashflow_tests::test_helpers::{
    financial_tolerance, FACTOR_TOLERANCE, RATE_TOLERANCE,
};
use time::Month;

fn kind_rank(kind: CFKind) -> u8 {
    match kind {
        CFKind::Fixed | CFKind::Stub | CFKind::FloatReset => 0,
        CFKind::Fee => 1,
        CFKind::Amortization => 2,
        CFKind::PIK => 3,
        CFKind::Notional => 4,
        _ => 5,
    }
}

#[test]
fn linear_vs_step_parity() {
    let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let maturity = Date::from_calendar_date(2026, Month::January, 15).unwrap();

    let fixed = FixedCouponSpec {
        coupon_type: CouponType::Cash,
        rate: 0.05,
        freq: Frequency::quarterly(),
        dc: DayCount::Act365F,
        bdc: BusinessDayConvention::Following,
        calendar_id: None,
        stub: StubKind::None,
    };

    let init = Money::new(1_000.0, Currency::USD);

    // Linear
    let mut b1 = CashFlowSchedule::builder();
    b1.principal(init, issue, maturity)
        .amortization(AmortizationSpec::LinearTo {
            final_notional: Money::new(0.0, Currency::USD),
        })
        .fixed_cf(fixed.clone());
    let s1 = b1.build().unwrap();

    // Step schedule equivalent
    let sched: Vec<Date> = ScheduleBuilder::new(issue, maturity)
        .frequency(Frequency::quarterly())
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
    b2.principal(init, issue, maturity)
        .amortization(AmortizationSpec::StepRemaining { schedule: pairs })
        .fixed_cf(fixed.clone());
    let s2 = b2.build().unwrap();

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
        rate: 0.10,
        freq: Frequency::semi_annual(),
        dc: DayCount::Act365F,
        bdc: BusinessDayConvention::Following,
        calendar_id: None,
        stub: StubKind::None,
    };

    let mut b = CashFlowSchedule::builder();
    b.principal(init, issue, maturity).fixed_cf(fixed.clone());
    let s = b.build().unwrap();
    let path = s.outstanding_path();
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
            cash_pct: 0.5,
            pik_pct: 0.5,
        },
        rate: 0.10,
        freq: Frequency::quarterly(),
        dc: DayCount::Act365F,
        bdc: BusinessDayConvention::Following,
        calendar_id: None,
        stub: StubKind::None,
    };

    // Percent-per-period amortization to force amort on coupon dates
    let mut b = CashFlowSchedule::builder();
    b.principal(init, issue, maturity)
        .amortization(AmortizationSpec::PercentPerPeriod { pct: 0.25 })
        .fixed_cf(fixed.clone());
    let s = b.build().unwrap();

    // On coupon dates where multiple flows exist, enforce order: Fixed/Stub -> Amortization -> PIK -> Notional
    let mut by_date: hashbrown::HashMap<Date, Vec<CFKind>> = hashbrown::HashMap::new();
    for cf in &s.flows {
        by_date.entry(cf.date).or_default().push(cf.kind);
    }

    for (_d, kinds) in by_date {
        let mut sorted = kinds.clone();
        sorted.sort_by_key(|k| kind_rank(*k));
        assert_eq!(kinds, sorted);
    }
}

#[test]
fn fixed_schedule_npv_equals_sum_cashflows() {
    let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let maturity = Date::from_calendar_date(2026, Month::January, 15).unwrap();

    let fixed = FixedCouponSpec {
        coupon_type: CouponType::Cash,
        rate: 0.05,
        freq: Frequency::semi_annual(),
        dc: DayCount::Act365F,
        bdc: BusinessDayConvention::Following,
        calendar_id: None,
        stub: StubKind::None,
    };

    let init = Money::new(1_000_000.0, Currency::USD);

    let mut b = CashFlowSchedule::builder();
    b.principal(init, issue, maturity).fixed_cf(fixed.clone());
    let schedule = b.build().unwrap();

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
        .npv(&curve, curve.base_date(), schedule.day_count)
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
        rate: 0.04,
        freq: Frequency::semi_annual(),
        dc: DayCount::Act365F,
        bdc: BusinessDayConvention::Following,
        calendar_id: None,
        stub: StubKind::None,
    };

    let init = Money::new(1_000_000.0, Currency::USD);

    let mut b = CashFlowSchedule::builder();
    b.principal(init, issue, maturity).fixed_cf(fixed.clone());
    let schedule = b.build().unwrap();

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
            cash_pct: 0.5,
            pik_pct: 0.5,
        },
        rate: 0.12,
        freq: Frequency::quarterly(),
        dc: DayCount::Act365F,
        bdc: BusinessDayConvention::Following,
        calendar_id: None,
        stub: StubKind::None,
    };

    let mut b = CashFlowSchedule::builder();
    b.principal(init, issue, maturity)
        .amortization(AmortizationSpec::PercentPerPeriod { pct: 0.25 })
        .fixed_cf(fixed.clone());
    let s = b.build().unwrap();

    let end_by_date = s.outstanding_by_date();

    // 1) One entry per unique date
    let unique_dates: std::collections::BTreeSet<Date> = s.flows.iter().map(|cf| cf.date).collect();
    assert_eq!(end_by_date.len(), unique_dates.len());
    // Dates are ordered
    for ((d1, _), d2) in end_by_date.iter().zip(unique_dates.iter()) {
        assert_eq!(d1, d2);
    }

    // 2) Values match the final outstanding on each date from outstanding_path()
    let path = s.outstanding_path();
    let mut last_by_date: hashbrown::HashMap<Date, f64> = hashbrown::HashMap::new();
    for (d, m) in path {
        last_by_date.insert(d, m.amount());
    }

    for (d, m) in end_by_date {
        let expected = *last_by_date.get(&d).unwrap();
        assert!(
            (m.amount() - expected).abs() < financial_tolerance(init.amount()),
            "Outstanding at {:?} should be {}, got {}",
            d,
            expected,
            m.amount()
        );
    }
}

#[test]
fn strict_schedule_mode_errors_on_unknown_calendar() {
    // Test that strict mode propagates calendar lookup errors
    let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let maturity = Date::from_calendar_date(2026, Month::January, 15).unwrap();

    let fixed = FixedCouponSpec {
        coupon_type: CouponType::Cash,
        rate: 0.05,
        freq: Frequency::semi_annual(),
        dc: DayCount::Act365F,
        bdc: BusinessDayConvention::Following,
        calendar_id: Some("UNKNOWN_CALENDAR_XYZ".to_string()),
        stub: StubKind::None,
    };

    // Strict mode should error
    let mut builder_strict = CashFlowSchedule::builder();
    builder_strict
        .principal(Money::new(1_000_000.0, Currency::USD), issue, maturity)
        .strict_schedules(true)
        .fixed_cf(fixed.clone());

    let result_strict = builder_strict.build();
    assert!(
        result_strict.is_err(),
        "Strict mode should error on unknown calendar"
    );

    // Graceful mode (default) should succeed with fallback
    let mut builder_graceful = CashFlowSchedule::builder();
    builder_graceful
        .principal(Money::new(1_000_000.0, Currency::USD), issue, maturity)
        .strict_schedules(false)
        .fixed_cf(fixed);

    let result_graceful = builder_graceful.build();
    assert!(
        result_graceful.is_ok(),
        "Graceful mode should succeed with fallback"
    );

    // Schedule should have flows despite calendar failure
    let schedule = result_graceful.unwrap();
    assert!(!schedule.flows.is_empty());
}

#[test]
fn try_builder_methods_error_before_principal() {
    // Test that try_* builder methods return errors instead of panicking
    let mut builder = CashFlowSchedule::builder();

    let fixed = FixedCouponSpec {
        coupon_type: CouponType::Cash,
        rate: 0.05,
        freq: Frequency::semi_annual(),
        dc: DayCount::Act365F,
        bdc: BusinessDayConvention::Following,
        calendar_id: None,
        stub: StubKind::None,
    };

    // Should error when principal not set
    let result = builder.try_fixed_cf(fixed.clone());
    assert!(
        result.is_err(),
        "try_fixed_cf should error when principal not set"
    );

    // After setting principal, should succeed
    let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let maturity = Date::from_calendar_date(2026, Month::January, 15).unwrap();
    builder.principal(Money::new(1_000_000.0, Currency::USD), issue, maturity);

    let result = builder.try_fixed_cf(fixed);
    assert!(
        result.is_ok(),
        "try_fixed_cf should succeed when principal set"
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
        rate: 0.06, // 6% annual rate
        freq: Frequency::semi_annual(),
        dc: DayCount::Thirty360, // Market standard for corporate bonds
        bdc: BusinessDayConvention::Following,
        calendar_id: None,
        stub: StubKind::None,
    };

    let init = Money::new(1_000_000.0, Currency::USD);

    let mut b = CashFlowSchedule::builder();
    b.principal(init, issue, maturity).fixed_cf(fixed.clone());
    let schedule = b.build().unwrap();

    // Find coupon flows only
    let coupon_flows: Vec<&CashFlow> = schedule
        .flows
        .iter()
        .filter(|cf| cf.kind == CFKind::Fixed || cf.kind == CFKind::Stub)
        .collect();

    // Should have at least one stub period
    let stubs: Vec<&&CashFlow> = coupon_flows.iter().filter(|cf| cf.kind == CFKind::Stub).collect();
    let regular: Vec<&&CashFlow> = coupon_flows.iter().filter(|cf| cf.kind == CFKind::Fixed).collect();

    assert!(!stubs.is_empty(), "Should have at least one stub period");
    assert!(!regular.is_empty(), "Should have at least one regular period");

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
    let model_150 = PrepaymentModelSpec::psa_150();
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

    let model = DefaultModelSpec::sda_100();

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
            assert!(
                smm < cpr,
                "SMM ({}) should be less than CPR ({})",
                smm,
                cpr
            );
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

    let model = DefaultModelSpec::sda_100();

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

    let sda_100 = DefaultModelSpec::sda_100();
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
