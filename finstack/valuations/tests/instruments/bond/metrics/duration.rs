//! Duration calculator tests (Macaulay and Modified).

use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_valuations::instruments::bond::Bond;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::metrics::MetricId;
use time::macros::date;

#[test]
fn test_duration_zero_coupon() {
    let as_of = date!(2025 - 01 - 01);
    let maturity = date!(2030 - 01 - 01);
    let mut bond = Bond::fixed(
        "DUR1",
        Money::new(100.0, Currency::USD),
        0.0,
        as_of,
        maturity,
        "USD-OIS",
    );
    bond.pricing_overrides =
        finstack_valuations::instruments::PricingOverrides::default().with_clean_price(70.0);

    let curve = finstack_core::market_data::term_structures::DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .knots([(0.0, 1.0), (5.0, 0.70)])
        .build()
        .unwrap();
    let market = finstack_core::market_data::context::MarketContext::new().insert_discount(curve);

    let result = bond
        .price_with_metrics(&market, as_of, &[MetricId::DurationMac])
        .unwrap();
    let mac_dur = *result.measures.get("duration_mac").unwrap();
    assert!((mac_dur - 5.0).abs() < 0.2); // Zero coupon duration ≈ maturity
}
