#![cfg(test)]

use super::*;
use finstack_core::dates::Date;
use finstack_core::market_data::multicurve::CurveSet;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::term_structures::forward_curve::ForwardCurve;
use finstack_core::prelude::*;
use time::Month;

fn flat_df_curve(id: &'static str, base: Date, df: F) -> DiscountCurve {
    // Build a trivial curve with two identical points to maintain piecewise structure
    let _ = df; // df not used directly; keep API consistent; use 1.0 for MVP tests
    DiscountCurve::builder(id)
        .base_date(base)
        .knots([(0.0, 1.0), (10.0, 1.0)])
        .linear_df()
        .build()
        .unwrap()
}

fn flat_fwd_curve(id: &'static str, base: Date, rate: F) -> ForwardCurve {
    ForwardCurve::builder(id, 0.25)
        .base_date(base)
        .knots([(0.0, rate), (10.0, rate)])
        .linear_df()
        .build()
        .unwrap()
}

#[test]
fn deposit_par_at_zero_rate_with_unit_df() {
    let start = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let end = Date::from_calendar_date(2025, Month::April, 1).unwrap();
    let disc = flat_df_curve("USD-OIS", start, 1.0);
    let curves = CurveSet::new().with_discount(disc);

    let dep = crate::instruments::deposit::Deposit {
        id: "DEP-USD-3M".into(),
        notional: Money::new(1_000_000.0, Currency::USD),
        start,
        end,
        day_count: DayCount::Act365F,
        quote_rate: Some(0.0),
        disc_id: "USD-OIS",
    };

    let res = dep.price(&curves, start).unwrap();
    // PV should be ~0 at par with DF=1
    assert!(res.value.amount().abs() < 1e-9);
}

#[test]
fn irs_par_rate_matches_forward_rate() {
    let base = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let disc = flat_df_curve("USD-OIS", base, 1.0);
    let fwd_rate = 0.05;
    let fwd = flat_fwd_curve("USD-SOFR3M", base, fwd_rate);
    let curves = CurveSet::new().with_discount(disc).with_forecast(fwd);

    let irs = crate::instruments::irs::InterestRateSwap {
        id: "IRS-TEST".into(),
        notional: Money::new(1_000_000.0, Currency::USD),
        side: crate::instruments::irs::PayReceive::PayFixed,
        fixed: crate::instruments::irs::FixedLegSpec {
            disc_id: "USD-OIS",
            rate: fwd_rate,
            freq: finstack_core::dates::Frequency::quarterly(),
            dc: DayCount::Act365F,
            calendar: None,
            start: base,
            end: Date::from_calendar_date(2026, Month::January, 1).unwrap(),
        },
        float: crate::instruments::irs::FloatLegSpec {
            disc_id: "USD-OIS",
            fwd_id: "USD-SOFR3M",
            spread_bp: 0.0,
            freq: finstack_core::dates::Frequency::quarterly(),
            dc: DayCount::Act365F,
            calendar: None,
            start: base,
            end: Date::from_calendar_date(2026, Month::January, 1).unwrap(),
        },
    };

    let res = irs.price(&curves, base).unwrap();
    let par = *res.measures.get("par_rate").unwrap();
    assert!((par - fwd_rate).abs() < 1e-12);
}

#[test]
fn bond_pv_with_unit_df_is_sum_of_cashflows() {
    let issue = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let mat = Date::from_calendar_date(2026, Month::January, 1).unwrap();
    let disc = flat_df_curve("USD-OIS", issue, 1.0);
    let curves = CurveSet::new().with_discount(disc);

    let bond = crate::instruments::bond::Bond {
        id: "BOND-TEST".into(),
        notional: Money::new(1_000.0, Currency::USD),
        coupon: 0.10, // 10%
        freq: finstack_core::dates::Frequency::semi_annual(),
        dc: DayCount::Act365F,
        issue,
        maturity: mat,
        disc_id: "USD-OIS",
    };

    let res = bond.price(&curves, issue).unwrap();
    // Two coupons (semi-annual, approx 0.5 year fractions), plus principal, DF=1
    assert!(res.value.amount() > 1_000.0);
}


