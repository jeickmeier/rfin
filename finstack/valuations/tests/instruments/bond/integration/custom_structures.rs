//! Custom bond structure integration tests (PIK, step-up, etc.).

use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, DayCount, Frequency, StubKind};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::money::Money;
use finstack_valuations::cashflow::builder::{
    CashFlowSchedule, CouponType, FixedCouponSpec, ScheduleParams,
};
use finstack_valuations::instruments::bond::Bond;
use finstack_valuations::instruments::common::traits::Instrument;
use time::macros::date;

fn create_curve() -> MarketContext {
    let curve = DiscountCurve::builder("USD-OIS")
        .base_date(date!(2025 - 01 - 01))
        .knots([(0.0, 1.0), (5.0, 0.80)])
        .build()
        .unwrap();
    MarketContext::new().insert_discount(curve)
}

#[test]
fn test_pik_bond() {
    let issue = date!(2025 - 01 - 01);
    let maturity = date!(2027 - 01 - 01);

    let schedule = CashFlowSchedule::builder()
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

    let bond = Bond::from_cashflows("PIK", schedule, "USD-OIS", None).unwrap();
    let market = create_curve();
    let pv = bond.value(&market, issue).unwrap();

    assert!(pv.amount() > 0.0);
}

#[test]
fn test_step_up_bond() {
    let issue = date!(2025 - 01 - 01);
    let maturity = date!(2028 - 01 - 01);
    let step1 = date!(2026 - 01 - 01);
    let step2 = date!(2027 - 01 - 01);

    let params = ScheduleParams {
        freq: Frequency::semi_annual(),
        dc: DayCount::Act365F,
        bdc: BusinessDayConvention::Following,
        calendar_id: None,
        stub: StubKind::None,
    };

    let schedule = CashFlowSchedule::builder()
        .principal(Money::new(1000.0, Currency::USD), issue, maturity)
        .fixed_stepup(
            &[(step1, 0.04), (step2, 0.05), (maturity, 0.06)],
            params,
            CouponType::Cash,
        )
        .build()
        .unwrap();

    let bond = Bond::from_cashflows("STEPUP", schedule, "USD-OIS", None).unwrap();
    let market = create_curve();
    let pv = bond.value(&market, issue).unwrap();

    assert!(pv.amount() > 0.0);
}
