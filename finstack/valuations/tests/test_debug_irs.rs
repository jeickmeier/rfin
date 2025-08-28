#![cfg(test)]

use finstack_core::dates::Date;

use finstack_core::market_data::multicurve::CurveSet;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::term_structures::forward_curve::ForwardCurve;
use finstack_core::prelude::*;
use finstack_valuations::instruments::irs;
use finstack_valuations::traits::Priceable;
use time::Month;

#[test]
fn debug_irs_metrics() {
    let base = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let disc = DiscountCurve::builder("USD-OIS")
        .base_date(base)
        .knots([(0.0, 1.0), (10.0, 1.0)])
        .linear_df()
        .build()
        .unwrap();
    let fwd_rate = 0.05;
    let fwd = ForwardCurve::builder("USD-SOFR3M", 0.25)
        .base_date(base)
        .knots([(0.0, fwd_rate), (10.0, fwd_rate)])
        .linear_df()
        .build()
        .unwrap();
    let curves = CurveSet::new().with_discount(disc).with_forecast(fwd);

    let irs = irs::InterestRateSwap {
        id: "IRS-TEST".into(),
        notional: Money::new(1_000_000.0, Currency::USD),
        side: irs::PayReceive::PayFixed,
        fixed: irs::FixedLegSpec {
            disc_id: "USD-OIS",
            rate: fwd_rate,
            freq: finstack_core::dates::Frequency::quarterly(),
            dc: DayCount::Act365F,
            bdc: finstack_core::dates::BusinessDayConvention::Following,
            calendar_id: None,
            stub: finstack_core::dates::StubKind::None,
            start: base,
            end: Date::from_calendar_date(2026, Month::January, 1).unwrap(),
        },
        float: irs::FloatLegSpec {
            disc_id: "USD-OIS",
            fwd_id: "USD-SOFR3M",
            spread_bp: 0.0,
            freq: finstack_core::dates::Frequency::quarterly(),
            dc: DayCount::Act365F,
            bdc: finstack_core::dates::BusinessDayConvention::Following,
            calendar_id: None,
            stub: finstack_core::dates::StubKind::None,
            start: base,
            end: Date::from_calendar_date(2026, Month::January, 1).unwrap(),
        },
    };

    let res = irs.price(&curves, base).unwrap();
    println!("IRS Metrics computed:");
    for (key, value) in &res.measures {
        println!("  {}: {}", key, value);
    }
    
    // Check if par_rate exists
    assert!(res.measures.contains_key("par_rate"), "par_rate metric not found! Available metrics: {:?}", res.measures.keys().collect::<Vec<_>>());
    
    let par = *res.measures.get("par_rate").unwrap();
    assert!((par - fwd_rate).abs() < 1e-12);
}
