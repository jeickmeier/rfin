//! Theta calculator tests.

use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_valuations::instruments::fixed_income::bond::Bond;
use finstack_valuations::instruments::internal::InstrumentExt as Instrument;
use finstack_valuations::metrics::MetricId;
use time::macros::date;

#[test]
fn test_theta_finite() {
    let as_of = date!(2025 - 01 - 01);
    let bond = Bond::fixed(
        "THETA1",
        Money::new(100.0, Currency::USD),
        0.05,
        as_of,
        date!(2030 - 01 - 01),
        "USD-OIS",
    )
    .unwrap();

    let curve = finstack_core::market_data::term_structures::DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .knots([(0.0, 1.0), (5.0, 0.80)])
        .build()
        .unwrap();
    let market = finstack_core::market_data::context::MarketContext::new().insert(curve);

    let result = bond
        .price_with_metrics(
            &market,
            as_of,
            &[MetricId::Theta],
            finstack_valuations::instruments::PricingOptions::default(),
        )
        .unwrap();
    let theta = *result.measures.get("theta").unwrap();
    assert!(theta.is_finite());
}
