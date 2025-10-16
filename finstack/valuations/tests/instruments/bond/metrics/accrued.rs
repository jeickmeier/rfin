//! Accrued interest calculator tests.

use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::money::Money;
use finstack_valuations::instruments::bond::Bond;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::metrics::MetricId;
use time::macros::date;

fn create_curve(base_date: Date) -> MarketContext {
    let curve = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots([(0.0, 1.0), (5.0, 0.80)])
        .build()
        .unwrap();
    MarketContext::new().insert_discount(curve)
}

#[test]
fn test_accrued_at_issue() {
    let as_of = date!(2025 - 01 - 01);
    let bond = Bond::fixed(
        "ACCR1",
        Money::new(100.0, Currency::USD),
        0.06,
        as_of,
        date!(2030 - 01 - 01),
        "USD-OIS",
    );
    let market = create_curve(as_of);
    let result = bond
        .price_with_metrics(&market, as_of, &[MetricId::Accrued])
        .unwrap();
    let accrued = *result.measures.get("accrued").unwrap();
    assert_eq!(accrued, 0.0);
}

#[test]
fn test_accrued_mid_period() {
    let issue = date!(2025 - 01 - 01);
    let mid = date!(2025 - 04 - 01); // ~3 months later
    let bond = Bond::fixed(
        "ACCR2",
        Money::new(100.0, Currency::USD),
        0.06,
        issue,
        date!(2030 - 01 - 01),
        "USD-OIS",
    );
    let market = create_curve(mid);
    let result = bond
        .price_with_metrics(&market, mid, &[MetricId::Accrued])
        .unwrap();
    let accrued = *result.measures.get("accrued").unwrap();
    assert!(accrued > 0.0 && accrued < 3.0); // Semi-annual 6% = 3% per period
}
