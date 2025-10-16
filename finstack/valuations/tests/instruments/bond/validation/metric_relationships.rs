//! Cross-metric validation tests.
//!
//! Tests fundamental relationships between bond metrics:
//! - Modified Duration = Macaulay Duration / (1 + YTM/m)
//! - DV01 = Price × Modified Duration × 0.0001
//! - Convexity and duration approximations

use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::money::Money;
use finstack_valuations::instruments::bond::Bond;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::metrics::MetricId;
use time::macros::date;

fn create_flat_curve(rate: f64, base_date: Date) -> DiscountCurve {
    DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .day_count(DayCount::Act365F)
        .knots([(0.0, 1.0), (5.0, (-rate * 5.0).exp() as f64)])
        .build()
        .unwrap()
}

#[test]
fn test_modified_macaulay_duration_relationship() {
    let as_of = date!(2025 - 01 - 01);
    let bond = Bond::fixed(
        "DUR_REL",
        Money::new(100.0, Currency::USD),
        0.06,
        as_of,
        date!(2030 - 01 - 01),
        "USD-OIS",
    );

    let curve = create_flat_curve(0.06, as_of);
    let market = MarketContext::new().insert_discount(curve);

    let result = bond
        .price_with_metrics(
            &market,
            as_of,
            &[MetricId::DurationMac, MetricId::DurationMod, MetricId::Ytm],
        )
        .unwrap();

    let mac_dur = *result.measures.get("duration_mac").unwrap();
    let mod_dur = *result.measures.get("duration_mod").unwrap();
    let ytm = *result.measures.get("ytm").unwrap();

    // ModDur = MacDur / (1 + ytm/m) for semi-annual
    let m = 2.0; // Semi-annual
    let expected_mod_dur = mac_dur / (1.0 + ytm / m);

    assert!((mod_dur - expected_mod_dur).abs() < 0.01);
}

#[test]
fn test_dv01_duration_price_relationship() {
    let as_of = date!(2025 - 01 - 01);
    let bond = Bond::fixed(
        "DV01_REL",
        Money::new(100.0, Currency::USD),
        0.05,
        as_of,
        date!(2030 - 01 - 01),
        "USD-OIS",
    );

    let curve = create_flat_curve(0.05, as_of);
    let market = MarketContext::new().insert_discount(curve);

    let result = bond
        .price_with_metrics(&market, as_of, &[MetricId::DurationMod, MetricId::Dv01])
        .unwrap();

    let mod_dur = *result.measures.get("duration_mod").unwrap();
    let dv01 = *result.measures.get("dv01").unwrap();
    let price = result.value.amount();

    // DV01 = Price × ModDur × 0.0001
    let expected_dv01 = price * mod_dur * 0.0001;

    assert!((dv01 - expected_dv01).abs() < 0.001);
}
