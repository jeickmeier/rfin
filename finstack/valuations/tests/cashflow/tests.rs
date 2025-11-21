//! Tests for the cashflow builder state module.

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
        assert!((a.amount.amount() - b.amount.amount()).abs() < 1e-9);
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
    assert!((pv.amount() - expected).abs() < 1e-9);
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
        assert!((m.amount() - expected).abs() < 1e-9);
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
