//! Convexity calculator tests.

use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_valuations::instruments::fixed_income::bond::Bond;
use finstack_valuations::instruments::Instrument;
use finstack_valuations::metrics::MetricId;
use time::macros::date;

/// Convexity validation for vanilla bonds
///
/// All vanilla (non-callable) bonds have positive convexity.
///
/// For a 5-year par bond at ~5% YTM, convexity is typically in the range 15-40.
/// The analytical formula: C = Σ[t(t+1) × CF_t × DF_t] / (P × (1+y/m)²)
#[test]
fn test_convexity_positive() {
    let as_of = date!(2025 - 01 - 01);
    let mut bond = Bond::fixed(
        "CVX1",
        Money::new(100.0, Currency::USD),
        0.05,
        as_of,
        date!(2030 - 01 - 01),
        "USD-OIS",
    )
    .unwrap();
    bond.pricing_overrides =
        finstack_valuations::instruments::PricingOverrides::default().with_clean_price(100.0);

    let curve = finstack_core::market_data::term_structures::DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .knots([(0.0, 1.0), (5.0, 0.80)])
        .build()
        .unwrap();
    let market = finstack_core::market_data::context::MarketContext::new().insert_discount(curve);

    let result = bond
        .price_with_metrics(&market, as_of, &[MetricId::Convexity])
        .unwrap();
    let cvx = *result.measures.get("convexity").unwrap();

    // All vanilla bonds have positive convexity
    assert!(cvx > 0.0, "Vanilla bonds should have positive convexity");

    // For 5-year par bond at ~5% YTM, convexity typically 15-40
    assert!(
        cvx > 15.0 && cvx < 40.0,
        "5Y par bond convexity {:.2} outside expected range [15, 40]",
        cvx
    );
}
