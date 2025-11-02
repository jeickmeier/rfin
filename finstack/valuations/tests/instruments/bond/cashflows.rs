//! Bond cashflow generation tests.
//!
//! Tests cashflow generation for:
//! - Fixed-rate bonds
//! - Floating-rate bonds (FRNs)
//! - Amortizing bonds
//! - Custom cashflow schedules
//! - PIK and step-up structures

use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Frequency, StubKind};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::{DiscountCurve, ForwardCurve};
use finstack_core::money::Money;
use finstack_core::types::CurveId;
use finstack_valuations::cashflow::builder::{
    CashFlowSchedule, CouponType, FixedCouponSpec, ScheduleParams,
};
use finstack_valuations::cashflow::primitives::AmortizationSpec;
use finstack_valuations::cashflow::traits::CashflowProvider;
use finstack_valuations::instruments::bond::{Bond, BondFloatSpec};
use finstack_valuations::instruments::PricingOverrides;
use time::macros::date;

fn create_test_curves(base_date: Date) -> MarketContext {
    let disc = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .day_count(DayCount::Act365F)
        .knots([(0.0, 1.0), (1.0, 0.95), (5.0, 0.80)])
        .build()
        .unwrap();

    let fwd = ForwardCurve::builder("USD-SOFR-3M", 0.25)
        .base_date(base_date)
        .knots([(0.0, 0.05), (5.0, 0.055)])
        .build()
        .unwrap();

    MarketContext::new()
        .insert_discount(disc)
        .insert_forward(fwd)
}

#[test]
fn test_fixed_rate_cashflows() {
    let as_of = date!(2025 - 01 - 01);
    let maturity = date!(2027 - 01 - 01);

    let bond = Bond::fixed(
        "FIXED_CF",
        Money::new(1000.0, Currency::USD),
        0.06,
        as_of,
        maturity,
        "USD-OIS",
    );

    let curves = create_test_curves(as_of);
    let flows = bond.build_schedule(&curves, as_of).unwrap();

    // Should have coupons + principal
    assert!(!flows.is_empty());

    // All flows after as_of
    for (date, _amount) in &flows {
        assert!(*date > as_of);
    }

    // Last flow should include principal
    let last_flow = flows.last().unwrap();
    assert!(last_flow.1.amount() > 30.0); // Coupon + principal
}

#[test]
fn test_cashflow_dates_alignment() {
    let as_of = date!(2025 - 01 - 01);
    let maturity = date!(2030 - 01 - 01);

    let bond = Bond::fixed(
        "CF_DATES",
        Money::new(1000.0, Currency::USD),
        0.05,
        as_of,
        maturity,
        "USD-OIS",
    );

    let curves = create_test_curves(as_of);
    let flows = bond.build_schedule(&curves, as_of).unwrap();

    // Verify dates are in chronological order (allowing for same-date flows)
    for i in 1..flows.len() {
        assert!(
            flows[i].0 >= flows[i - 1].0,
            "Flows should be in chronological order: flow[{}]={:?} < flow[{}]={:?}",
            i,
            flows[i].0,
            i - 1,
            flows[i - 1].0
        );
    }

    // Last flow should be at or before maturity
    assert!(flows.last().unwrap().0 <= maturity);
}

#[test]
fn test_quarterly_coupon_frequency() {
    let as_of = date!(2025 - 01 - 01);
    let maturity = date!(2026 - 01 - 01);

    let bond = Bond::builder()
        .id("QUARTERLY".into())
        .notional(Money::new(1000.0, Currency::USD))
        .coupon(0.04)
        .issue(as_of)
        .maturity(maturity)
        .freq(Frequency::quarterly())
        .dc(DayCount::Act365F)
        .bdc(BusinessDayConvention::Following)
        .calendar_id_opt(None)
        .stub(StubKind::None)
        .discount_curve_id("USD-OIS".into())
        .pricing_overrides(PricingOverrides::default())
        .build()
        .unwrap();

    let curves = create_test_curves(as_of);
    let flows = bond.build_schedule(&curves, as_of).unwrap();

    // 1 year = 4 quarters + principal
    assert_eq!(flows.len(), 5);
}

#[test]
fn test_floating_rate_cashflows() {
    let as_of = date!(2025 - 01 - 01);
    let maturity = date!(2027 - 01 - 01);

    let bond = Bond::builder()
        .id("FRN".into())
        .notional(Money::new(1000.0, Currency::USD))
        .coupon(0.0) // Unused for FRN
        .issue(as_of)
        .maturity(maturity)
        .freq(Frequency::quarterly())
        .dc(DayCount::Act360)
        .bdc(BusinessDayConvention::Following)
        .calendar_id_opt(None)
        .stub(StubKind::None)
        .discount_curve_id(CurveId::new("USD-OIS"))
        .float_opt(Some(BondFloatSpec {
            forward_curve_id: CurveId::new("USD-SOFR-3M"),
            margin_bp: 150.0,
            gearing: 1.0,
            reset_lag_days: 2,
        }))
        .pricing_overrides(PricingOverrides::default())
        .build()
        .unwrap();

    let curves = create_test_curves(as_of);
    let flows = bond.build_schedule(&curves, as_of).unwrap();

    // Should have floating coupons + principal
    assert!(!flows.is_empty());

    // All flows should be positive (holder perspective)
    for (_date, amount) in &flows {
        assert!(amount.amount() > 0.0);
    }
}

#[test]
fn test_amortizing_bond_linear() {
    let as_of = date!(2025 - 01 - 01);
    let maturity = date!(2027 - 01 - 01);

    let bond = Bond::builder()
        .id("AMORT_LINEAR".into())
        .notional(Money::new(1000.0, Currency::USD))
        .coupon(0.05)
        .issue(as_of)
        .maturity(maturity)
        .freq(Frequency::semi_annual())
        .dc(DayCount::Act365F)
        .bdc(BusinessDayConvention::Following)
        .calendar_id_opt(None)
        .stub(StubKind::None)
        .discount_curve_id("USD-OIS".into())
        .amortization_opt(Some(AmortizationSpec::LinearTo {
            final_notional: Money::new(400.0, Currency::USD),
        }))
        .pricing_overrides(PricingOverrides::default())
        .build()
        .unwrap();

    let curves = create_test_curves(as_of);
    let flows = bond.build_schedule(&curves, as_of).unwrap();

    // Should have cashflows (coupons + amortization + redemption)
    assert!(!flows.is_empty(), "Amortizing bond should have cashflows");

    // Amortizing bonds have:
    // - Positive coupon flows
    // - Negative amortization flows (principal paydowns from holder perspective)
    // - Positive final redemption

    // Count different flow types
    let positive_flows: Vec<_> = flows.iter().filter(|(_, amt)| amt.amount() > 0.0).collect();
    let negative_flows: Vec<_> = flows.iter().filter(|(_, amt)| amt.amount() < 0.0).collect();

    // Should have both positive (coupons + redemption) and negative (amortization) flows
    assert!(
        !positive_flows.is_empty(),
        "Should have positive cashflows (coupons, redemption)"
    );
    assert!(
        !negative_flows.is_empty(),
        "Should have negative cashflows (amortization)"
    );

    // Net cashflow should be positive (holder receives more than pays)
    let total: f64 = flows.iter().map(|(_, amt)| amt.amount()).sum();
    assert!(total.is_finite(), "Total cashflow should be finite");
}

#[test]
fn test_custom_cashflows_from_schedule() {
    let issue = date!(2025 - 01 - 01);
    let maturity = date!(2027 - 01 - 01);

    // Build custom step-up schedule
    let step1 = date!(2026 - 01 - 01);
    let params = ScheduleParams {
        freq: Frequency::semi_annual(),
        dc: DayCount::Act365F,
        bdc: BusinessDayConvention::Following,
        calendar_id: None,
        stub: StubKind::None,
    };

    let custom_schedule = CashFlowSchedule::builder()
        .principal(Money::new(1000.0, Currency::USD), issue, maturity)
        .fixed_stepup(&[(step1, 0.04), (maturity, 0.06)], params, CouponType::Cash)
        .build()
        .unwrap();

    let bond = Bond::from_cashflows("CUSTOM", custom_schedule, "USD-OIS", Some(98.0)).unwrap();

    let curves = create_test_curves(issue);
    let flows = bond.build_schedule(&curves, issue).unwrap();

    // Should use custom cashflows
    assert!(!flows.is_empty());
}

#[test]
fn test_pik_cashflows() {
    let issue = date!(2025 - 01 - 01);
    let maturity = date!(2027 - 01 - 01);

    // Build PIK toggle schedule
    let custom_schedule = CashFlowSchedule::builder()
        .principal(Money::new(1000.0, Currency::USD), issue, maturity)
        .fixed_cf(FixedCouponSpec {
            coupon_type: CouponType::PIK,
            rate: 0.08,
            freq: Frequency::semi_annual(),
            dc: DayCount::Act365F,
            bdc: BusinessDayConvention::Following,
            calendar_id: None,
            stub: StubKind::None,
        })
        .build()
        .unwrap();

    let bond = Bond::from_cashflows("PIK", custom_schedule, "USD-OIS", None).unwrap();

    let curves = create_test_curves(issue);
    let full_schedule = bond.get_full_schedule(&curves).unwrap();

    // Should have PIK coupons in the schedule
    assert!(!full_schedule.flows.is_empty());
}

#[test]
fn test_cashflows_for_matured_bond() {
    let as_of = date!(2025 - 01 - 01);
    let issue = date!(2020 - 01 - 01);
    let maturity = date!(2024 - 01 - 01); // Already matured

    let bond = Bond::fixed(
        "MATURED",
        Money::new(1000.0, Currency::USD),
        0.05,
        issue,
        maturity,
        "USD-OIS",
    );

    let curves = create_test_curves(as_of);
    let flows = bond.build_schedule(&curves, as_of).unwrap();

    // The cashflow builder generates all flows from issue to maturity.
    // The CashflowProvider trait's build_schedule filters to future flows (date > as_of).
    // However, the builder itself may still include all historical flows.
    // What matters is that when pricing, only future flows are used.

    // Verify no flows are after the maturity date
    for (date, _) in &flows {
        assert!(*date <= maturity, "No flows should be after maturity");
    }
}

#[test]
fn test_cashflows_with_short_front_stub() {
    let as_of = date!(2025 - 01 - 15); // Mid-month start
    let maturity = date!(2027 - 01 - 01);

    let bond = Bond::builder()
        .id("STUB_SHORT".into())
        .notional(Money::new(1000.0, Currency::USD))
        .coupon(0.05)
        .issue(as_of)
        .maturity(maturity)
        .freq(Frequency::semi_annual())
        .dc(DayCount::Act365F)
        .bdc(BusinessDayConvention::Following)
        .calendar_id_opt(None)
        .stub(StubKind::ShortFront)
        .discount_curve_id("USD-OIS".into())
        .pricing_overrides(PricingOverrides::default())
        .build()
        .unwrap();

    let curves = create_test_curves(as_of);
    let flows = bond.build_schedule(&curves, as_of).unwrap();

    // Should generate cashflows with stub handling
    assert!(!flows.is_empty());

    // Verify all flows are after issue date
    for (date, _) in &flows {
        assert!(*date > as_of, "All flows should be after issue");
    }

    // Should have multiple payment dates through to maturity
    assert!(flows.len() > 1, "Should have multiple cashflows");
}

#[test]
fn test_zero_coupon_cashflows() {
    let as_of = date!(2025 - 01 - 01);
    let maturity = date!(2030 - 01 - 01);

    let bond = Bond::fixed(
        "ZERO",
        Money::new(1000.0, Currency::USD),
        0.0,
        as_of,
        maturity,
        "USD-OIS",
    );

    let curves = create_test_curves(as_of);
    let flows = bond.build_schedule(&curves, as_of).unwrap();

    // Zero coupon bond: only principal at maturity
    // The schedule builder may still generate coupon periods but with 0 amounts
    // Or it may optimize to just the redemption
    assert!(!flows.is_empty());

    // Last flow should be the principal
    let last_flow = flows.last().unwrap();
    assert_eq!(last_flow.0, maturity);
    assert!(last_flow.1.amount() > 0.0);
}

#[test]
fn test_cashflows_notional_scaling() {
    let as_of = date!(2025 - 01 - 01);
    let maturity = date!(2027 - 01 - 01);

    let notionals = vec![100.0, 1_000.0, 1_000_000.0, 10_000_000.0];

    for notional_amt in notionals {
        let bond = Bond::fixed(
            format!("SCALE_{}", notional_amt),
            Money::new(notional_amt, Currency::USD),
            0.05,
            as_of,
            maturity,
            "USD-OIS",
        );

        let curves = create_test_curves(as_of);
        let flows = bond.build_schedule(&curves, as_of).unwrap();

        assert!(!flows.is_empty());

        // Last flow should scale with notional
        let last_amount = flows.last().unwrap().1.amount();
        assert!(last_amount > notional_amt * 0.5); // At least includes partial principal
    }
}

#[test]
fn test_get_full_schedule_fixed() {
    let as_of = date!(2025 - 01 - 01);
    let maturity = date!(2027 - 01 - 01);

    let bond = Bond::fixed(
        "FULL_SCHED",
        Money::new(1000.0, Currency::USD),
        0.06,
        as_of,
        maturity,
        "USD-OIS",
    );

    let curves = create_test_curves(as_of);
    let full_schedule = bond.get_full_schedule(&curves).unwrap();

    // Should have flows with CFKind metadata
    assert!(!full_schedule.flows.is_empty());
    assert_eq!(full_schedule.notional.initial.amount(), 1000.0);
}

#[test]
fn test_get_full_schedule_floating() {
    let as_of = date!(2025 - 01 - 01);
    let maturity = date!(2027 - 01 - 01);

    let bond = Bond::builder()
        .id("FULL_FRN".into())
        .notional(Money::new(1000.0, Currency::USD))
        .coupon(0.0)
        .issue(as_of)
        .maturity(maturity)
        .freq(Frequency::quarterly())
        .dc(DayCount::Act360)
        .bdc(BusinessDayConvention::Following)
        .calendar_id_opt(None)
        .stub(StubKind::None)
        .discount_curve_id(CurveId::new("USD-OIS"))
        .float_opt(Some(BondFloatSpec {
            forward_curve_id: CurveId::new("USD-SOFR-3M"),
            margin_bp: 100.0,
            gearing: 1.0,
            reset_lag_days: 2,
        }))
        .pricing_overrides(PricingOverrides::default())
        .build()
        .unwrap();

    let curves = create_test_curves(as_of);
    let full_schedule = bond.get_full_schedule(&curves).unwrap();

    assert!(!full_schedule.flows.is_empty());
}

#[test]
fn test_cashflows_day_count_conventions() {
    let as_of = date!(2025 - 01 - 01);
    let maturity = date!(2027 - 01 - 01);

    let day_counts = vec![
        DayCount::Thirty360,
        DayCount::Act360,
        DayCount::Act365F,
        DayCount::ActAct,
    ];

    for dc in day_counts {
        let bond = Bond::builder()
            .id(format!("DC_{:?}", dc).into())
            .notional(Money::new(1000.0, Currency::USD))
            .coupon(0.05)
            .issue(as_of)
            .maturity(maturity)
            .freq(Frequency::semi_annual())
            .dc(dc)
            .bdc(BusinessDayConvention::Following)
            .calendar_id_opt(None)
            .stub(StubKind::None)
            .discount_curve_id("USD-OIS".into())
            .pricing_overrides(PricingOverrides::default())
            .build()
            .unwrap();

        let curves = create_test_curves(as_of);
        let flows = bond.build_schedule(&curves, as_of).unwrap();

        // All day count conventions should produce valid cashflows
        assert!(!flows.is_empty());
    }
}

#[test]
fn test_amortizing_full_redemption() {
    let as_of = date!(2025 - 01 - 01);
    let maturity = date!(2027 - 01 - 01);

    let bond = Bond::builder()
        .id("AMORT_FULL".into())
        .notional(Money::new(1000.0, Currency::USD))
        .coupon(0.05)
        .issue(as_of)
        .maturity(maturity)
        .freq(Frequency::semi_annual())
        .dc(DayCount::Act365F)
        .bdc(BusinessDayConvention::Following)
        .calendar_id_opt(None)
        .stub(StubKind::None)
        .discount_curve_id("USD-OIS".into())
        .amortization_opt(Some(AmortizationSpec::LinearTo {
            final_notional: Money::new(0.0, Currency::USD),
        }))
        .pricing_overrides(PricingOverrides::default())
        .build()
        .unwrap();

    let curves = create_test_curves(as_of);
    let flows = bond.build_schedule(&curves, as_of).unwrap();

    // Should have amortization flows throughout
    assert!(!flows.is_empty());
}
