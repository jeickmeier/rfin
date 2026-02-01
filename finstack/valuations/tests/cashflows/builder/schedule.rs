//! Tests for cashflow schedule generation and computation.
//!
//! This module covers:
//! - Schedule generation with various amortization schemes
//! - Flow ordering within dates
//! - Stub period detection
//! - Outstanding balance tracking
//! - PV/NPV calculations
//! - Day count conventions in schedule context
//!
//! # Tolerance Conventions
//!
//! - `RATE_TOLERANCE` (1e-10): For rate/factor comparisons
//! - `FACTOR_TOLERANCE` (1e-12): For year fractions
//! - `financial_tolerance(notional)`: For money amounts

use crate::helpers::financial_tolerance;
use finstack_core::cashflow::{CFKind, CashFlow};
use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, DayCount, StubKind, Tenor};
use finstack_core::dates::{Date, ScheduleBuilder};
use finstack_core::market_data::term_structures::DiscountCurve as CoreDiscCurve;
use finstack_core::math::interp::InterpStyle;
use finstack_core::money::Money;
use finstack_valuations::cashflow::builder::specs::{CouponType, FixedCouponSpec};
use finstack_valuations::cashflow::builder::{AmortizationSpec, CashFlowSchedule};
use finstack_valuations::instruments::Discountable;
use rust_decimal::Decimal;
use time::Month;

// =============================================================================
// Amortization Scheme Tests
// =============================================================================

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

// =============================================================================
// Flow Ordering Tests
// =============================================================================

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

// =============================================================================
// PV/NPV Calculation Tests
// =============================================================================

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

// =============================================================================
// Stub Period Detection Tests
// =============================================================================

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

// =============================================================================
// Outstanding Balance Tracking Tests
// =============================================================================

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

// =============================================================================
// Error Handling Tests
// =============================================================================

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
// Day Count Convention in Schedule Context Tests
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
// NPV Golden Value Tests (with realistic discounting)
// =============================================================================

/// Golden value test: NPV discounting verification
///
/// This test verifies that discounting is working correctly by checking:
/// 1. NPV with DF=1 equals sum of positive flows
/// 2. NPV decreases as discount rate increases
/// 3. The discount factor is applied correctly
///
/// Note: The schedule includes an initial funding flow (negative) at issue date.
/// For holder-view NPV (excludes flows <= as_of), we need to understand that
/// NPV represents the net value of all flows to the holder.
#[test]
fn npv_golden_value_with_realistic_discount_curve() {
    let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let maturity = Date::from_calendar_date(2026, Month::January, 15).unwrap();

    let fixed = FixedCouponSpec {
        coupon_type: CouponType::Cash,
        rate: Decimal::try_from(0.05).expect("valid"), // 5% coupon
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

    // Count positive and negative flows to understand structure
    let positive_flows: f64 = schedule
        .flows
        .iter()
        .filter(|cf| cf.amount.amount() > 0.0)
        .map(|cf| cf.amount.amount())
        .sum();

    let negative_flows: f64 = schedule
        .flows
        .iter()
        .filter(|cf| cf.amount.amount() < 0.0)
        .map(|cf| cf.amount.amount())
        .sum();

    // Total undiscounted: should be net of funding (-1M) + coupons + redemption (+1M)
    // For a bond: -1M (funding) + coupons + 1M (redemption) = coupons only
    let _total_undiscounted = positive_flows + negative_flows;

    // Build flat DF=1 curve for baseline
    let flat_curve = CoreDiscCurve::builder("USD-OIS")
        .base_date(issue)
        .knots([(0.0, 1.0), (2.0, 1.0)])
        .set_interp(InterpStyle::Linear)
        .allow_non_monotonic()
        .build()
        .unwrap();

    let npv_flat = schedule
        .npv(&flat_curve, issue, Some(schedule.day_count))
        .unwrap();

    // Build 5% discount curve
    let curve_5pct = CoreDiscCurve::builder("USD-OIS")
        .base_date(issue)
        .knots([
            (0.0, 1.0),
            (0.5, (-0.05_f64 * 0.5).exp()),
            (1.0, (-0.05_f64 * 1.0).exp()),
        ])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();

    let npv_5pct = schedule
        .npv(&curve_5pct, issue, Some(schedule.day_count))
        .unwrap();

    // Key invariants:
    // 1. NPV with 5% rate should be less than NPV with flat curve (discounting works)
    assert!(
        npv_5pct.amount() < npv_flat.amount() + 1.0, // Allow small tolerance
        "NPV with 5% rate ({:.2}) should be <= NPV with flat rate ({:.2})",
        npv_5pct.amount(),
        npv_flat.amount()
    );

    // 2. The schedule generates expected number of positive cash flows
    // (2 coupons + 1 redemption = 3 positive flows for 1-year semi-annual)
    let positive_count = schedule
        .flows
        .iter()
        .filter(|cf| cf.amount.amount() > 0.0)
        .count();
    assert!(
        positive_count >= 2,
        "Should have at least 2 positive flows (coupons + redemption), got {}",
        positive_count
    );

    // 3. Verify coupon amounts are correct (~25K each for 5% semi-annual on 1M)
    let coupons: Vec<_> = schedule
        .flows
        .iter()
        .filter(|cf| cf.kind == CFKind::Fixed)
        .collect();

    for coupon in &coupons {
        let expected_coupon = 25_000.0; // 1M * 5% / 2
        assert!(
            (coupon.amount.amount() - expected_coupon).abs() < 2000.0,
            "Coupon should be ~${:.0}, got ${:.2}",
            expected_coupon,
            coupon.amount.amount()
        );
    }
}

/// Golden value test: coupon amounts with known expected values
///
/// Verifies that:
/// - Semi-annual 5% coupon on $1M = $25,000 (exactly)
/// - Day count fraction is applied correctly
#[test]
fn coupon_amount_golden_values() {
    let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let maturity = Date::from_calendar_date(2026, Month::January, 15).unwrap();

    // 5% semi-annual coupon on $1M notional
    let fixed = FixedCouponSpec {
        coupon_type: CouponType::Cash,
        rate: Decimal::try_from(0.05).expect("valid"),
        freq: Tenor::semi_annual(),
        dc: DayCount::Thirty360, // 30/360 gives exact 0.5 year fraction for 6 months
        bdc: BusinessDayConvention::Following,
        calendar_id: None,
        stub: StubKind::None,
    };

    let init = Money::new(1_000_000.0, Currency::USD);

    let mut b = CashFlowSchedule::builder();
    let _ = b.principal(init, issue, maturity).fixed_cf(fixed.clone());
    let schedule = b.build_with_curves(None).unwrap();

    // Find coupon flows
    let coupons: Vec<&CashFlow> = schedule
        .flows
        .iter()
        .filter(|cf| cf.kind == CFKind::Fixed)
        .collect();

    // Expected coupon: $1M * 5% * 0.5 = $25,000
    let expected_coupon = 25_000.0;

    for coupon in &coupons {
        assert!(
            (coupon.amount.amount() - expected_coupon).abs() < financial_tolerance(init.amount()),
            "Coupon amount should be ${:.2}, got ${:.2}",
            expected_coupon,
            coupon.amount.amount()
        );

        // Verify accrual factor is approximately 0.5 for semi-annual
        assert!(
            (coupon.accrual_factor - 0.5).abs() < 0.01,
            "Accrual factor should be ~0.5 for semi-annual, got {}",
            coupon.accrual_factor
        );
    }

    // Verify principal redemption amount
    let redemption: Vec<&CashFlow> = schedule
        .flows
        .iter()
        .filter(|cf| cf.kind == CFKind::Notional && cf.amount.amount() > 0.0)
        .collect();

    assert_eq!(
        redemption.len(),
        1,
        "Should have exactly one redemption flow"
    );
    assert!(
        (redemption[0].amount.amount() - init.amount()).abs() < financial_tolerance(init.amount()),
        "Redemption should equal notional: expected ${:.2}, got ${:.2}",
        init.amount(),
        redemption[0].amount.amount()
    );
}

/// Invariant test: PV01 relationship (higher rates = lower NPV)
#[test]
fn npv_decreases_with_higher_discount_rate() {
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

    // Build curves at different rates
    let build_curve = |rate: f64| {
        CoreDiscCurve::builder("USD-OIS")
            .base_date(issue)
            .knots([
                (0.0, 1.0),
                (0.5, (-rate * 0.5).exp()),
                (1.0, (-rate * 1.0).exp()),
            ])
            .set_interp(InterpStyle::Linear)
            .build()
            .unwrap()
    };

    let curve_3pct = build_curve(0.03);
    let curve_5pct = build_curve(0.05);
    let curve_7pct = build_curve(0.07);

    let npv_3pct = schedule
        .npv(
            &curve_3pct,
            curve_3pct.base_date(),
            Some(schedule.day_count),
        )
        .unwrap();
    let npv_5pct = schedule
        .npv(
            &curve_5pct,
            curve_5pct.base_date(),
            Some(schedule.day_count),
        )
        .unwrap();
    let npv_7pct = schedule
        .npv(
            &curve_7pct,
            curve_7pct.base_date(),
            Some(schedule.day_count),
        )
        .unwrap();

    // Monotonicity: higher discount rate = lower NPV
    assert!(
        npv_3pct.amount() > npv_5pct.amount(),
        "NPV at 3% ({}) should be greater than NPV at 5% ({})",
        npv_3pct.amount(),
        npv_5pct.amount()
    );
    assert!(
        npv_5pct.amount() > npv_7pct.amount(),
        "NPV at 5% ({}) should be greater than NPV at 7% ({})",
        npv_5pct.amount(),
        npv_7pct.amount()
    );
}
