//! Test cases for `Valuation` code.

use finstack_core::cashflow::primitives::CFKind;
use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Frequency, StubKind};
use finstack_core::math::interp::InterpStyle;
// use finstack_core::market_data::traits::Discount as _;
use finstack_core::money::Money;
use finstack_valuations::cashflow::builder::{
    cf, CouponType, FixedWindow, FloatCouponParams, FloatWindow, ScheduleParams,
};
use finstack_valuations::instruments::common::discountable::Discountable;
use time::Month;

#[test]
fn fixed_stepup_aligned_and_misaligned_boundaries() {
    let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let maturity = Date::from_calendar_date(2026, Month::January, 15).unwrap();
    let init = Money::new(1_000_000.0, Currency::USD);

    let sched = ScheduleParams {
        freq: Frequency::quarterly(),
        dc: DayCount::Act365F,
        bdc: BusinessDayConvention::Following,
        calendar_id: None,
        stub: StubKind::None,
    };

    let boundary = Date::from_calendar_date(2025, Month::May, 1).unwrap(); // may not align -> stub

    let mut b = cf();
    b.principal(init, issue, maturity).fixed_stepup(
        &[(boundary, 0.05), (maturity, 0.06)],
        sched,
        CouponType::Cash,
    );
    let s = b.build().unwrap();

    // Ensure stub detection around boundary
    let has_stub = s
        .flows
        .iter()
        .any(|cf| cf.kind == CFKind::Stub);
    assert!(
        has_stub,
        "Expected a stub period due to misaligned step-up boundary"
    );

    // PV with flat curve DF=1.0 equals sum of flows
    let curve = CoreDiscCurve::builder("USD-OIS")
        .base_date(issue)
        .knots([(0.0, 1.0), (5.0, 1.0)])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();
    let pv = s.npv(&curve, curve.base_date(), s.day_count).unwrap();
    let expected = s.flows.iter().fold(0.0, |sum, cf| sum + cf.amount.amount());
    assert!((pv.amount() - expected).abs() < 1e-9);
}

#[test]
fn float_margin_stepup_creates_resets_and_uses_split() {
    let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let maturity = Date::from_calendar_date(2026, Month::January, 15).unwrap();
    let init = Money::new(500_000.0, Currency::USD);

    let sched = ScheduleParams {
        freq: Frequency::quarterly(),
        dc: DayCount::Act365F,
        bdc: BusinessDayConvention::Following,
        calendar_id: None,
        stub: StubKind::None,
    };

    let params = FloatCouponParams {
        index_id: "SOFR_3M",
        margin_bp: 0.0,
        gearing: 1.0,
        reset_lag_days: 2,
    };
    let boundary = Date::from_calendar_date(2025, Month::July, 5).unwrap();

    let mut b = cf();
    b.principal(init, issue, maturity).float_margin_stepup(
        &[(boundary, 450.0), (maturity, 600.0)],
        params,
        sched,
        CouponType::Split {
            cash_pct: 0.6,
            pik_pct: 0.4,
        },
    );
    let s = b.build().unwrap();

    // Ensure float resets are present and PIK flows exist
    let has_resets = s
        .flows
        .iter()
        .any(|cf| cf.kind == CFKind::FloatReset);
    let has_pik = s
        .flows
        .iter()
        .any(|cf| cf.kind == CFKind::PIK);
    assert!(has_resets && has_pik);
}

#[test]
fn fixed_to_float_switch_no_double_count_at_switch() {
    let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let switch = Date::from_calendar_date(2025, Month::June, 15).unwrap();
    let maturity = Date::from_calendar_date(2026, Month::January, 15).unwrap();
    let init = Money::new(750_000.0, Currency::USD);

    let fixed_sched = ScheduleParams {
        freq: Frequency::quarterly(),
        dc: DayCount::Act365F,
        bdc: BusinessDayConvention::Following,
        calendar_id: None,
        stub: StubKind::None,
    };
    let float_sched = ScheduleParams {
        freq: Frequency::quarterly(),
        dc: DayCount::Act365F,
        bdc: BusinessDayConvention::Following,
        calendar_id: None,
        stub: StubKind::None,
    };
    let fixed_win = FixedWindow {
        rate: 0.05,
        schedule: fixed_sched,
    };
    let float_params = FloatCouponParams {
        index_id: "SOFR_3M",
        margin_bp: 600.0,
        gearing: 1.0,
        reset_lag_days: 2,
    };
    let float_win = FloatWindow {
        params: float_params,
        schedule: float_sched,
    };

    let mut b = cf();
    b.principal(init, issue, maturity).fixed_to_float(
        switch,
        fixed_win,
        float_win,
        CouponType::Cash,
    );
    let s = b.build().unwrap();

    // Ensure no duplicate coupon at the exact switch date
    let coupons_on_switch: Vec<_> = s.flows.iter().filter(|cf| cf.date == switch).collect();
    assert!(
        coupons_on_switch.len() <= 2,
        "At most one coupon kind (and possibly amort/PIK) on switch date"
    );
}

#[test]
fn pik_toggle_window_increases_outstanding_only_in_window() {
    let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let start_pik = Date::from_calendar_date(2025, Month::April, 15).unwrap();
    let end_pik = Date::from_calendar_date(2025, Month::October, 15).unwrap();
    let maturity = Date::from_calendar_date(2026, Month::January, 15).unwrap();
    let init = Money::new(100_000.0, Currency::USD);

    let sched = ScheduleParams {
        freq: Frequency::quarterly(),
        dc: DayCount::Act365F,
        bdc: BusinessDayConvention::Following,
        calendar_id: None,
        stub: StubKind::None,
    };

    let mut b = cf();
    b.principal(init, issue, maturity)
        .fixed_stepup(&[(maturity, 0.12)], sched, CouponType::Cash)
        .add_payment_window(start_pik, end_pik, CouponType::PIK);
    let s = b.build().unwrap();

    let path = s.outstanding_by_date();
    let mut increasing_inside = false;
    let mut non_increasing_before_after = true;
    for (d, _m) in path {
        if d >= start_pik && d <= end_pik {
            // Expect to find at least one date in window where PIK increased outstanding
            if s.flows.iter().any(|cf| {
                cf.date == d && cf.kind == CFKind::PIK
            }) {
                increasing_inside = true;
            }
        } else if d < start_pik || d > end_pik {
            // Outside window, PIK should not appear
            if s.flows.iter().any(|cf| {
                cf.date == d && cf.kind == CFKind::PIK
            }) {
                non_increasing_before_after = false;
            }
        }
    }
    assert!(increasing_inside && non_increasing_before_after);
}
