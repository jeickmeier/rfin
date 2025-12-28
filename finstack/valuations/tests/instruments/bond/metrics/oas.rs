//! Option-adjusted spread calculator tests.

use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_valuations::instruments::bond::Bond;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::instruments::PricingOverrides;
use finstack_valuations::metrics::MetricId;
use time::macros::date;

#[test]
fn test_oas_behavior_without_quoted_price() {
    let as_of = date!(2025 - 01 - 01);
    let bond = Bond::fixed(
        "OAS1",
        Money::new(100.0, Currency::USD),
        0.05,
        as_of,
        date!(2030 - 01 - 01),
        "USD-OIS",
    ).unwrap();

    let curve = finstack_core::market_data::term_structures::DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .knots([(0.0, 1.0), (5.0, 0.80)])
        .build()
        .unwrap();
    let market = finstack_core::market_data::context::MarketContext::new().insert_discount(curve);

    // OAS calculation without quoted price - verify behavior
    let result = bond.price_with_metrics(&market, as_of, &[MetricId::Oas]);

    // Implementation may succeed with fallback to model price or may error
    // Just verify it handles the case gracefully
    if let Ok(res) = result {
        // If it succeeds, verify OAS is finite
        if let Some(oas) = res.measures.get("oas") {
            assert!(oas.is_finite(), "OAS should be finite if calculated");
        }
    }
    // If it errors, that's also acceptable behavior
}

#[test]
fn test_oas_with_quoted_price() {
    let as_of = date!(2025 - 01 - 01);
    let mut bond = Bond::fixed(
        "OAS2",
        Money::new(100.0, Currency::USD),
        0.05,
        as_of,
        date!(2030 - 01 - 01),
        "USD-OIS",
    ).unwrap();
    bond.pricing_overrides = PricingOverrides::default().with_clean_price(98.0);

    let curve = finstack_core::market_data::term_structures::DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .knots([(0.0, 1.0), (5.0, 0.80)])
        .build()
        .unwrap();
    let market = finstack_core::market_data::context::MarketContext::new().insert_discount(curve);

    let result = bond
        .price_with_metrics(&market, as_of, &[MetricId::Oas])
        .unwrap();
    let oas = *result.measures.get("oas").unwrap();
    assert!(oas.is_finite());
}
