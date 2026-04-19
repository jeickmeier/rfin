//! CS01 calculator tests.

use finstack_core::currency::Currency;
use finstack_core::market_data::term_structures::HazardCurve;
use finstack_core::money::Money;
use finstack_core::types::CurveId;
use finstack_valuations::instruments::fixed_income::bond::Bond;
use finstack_valuations::instruments::Instrument;
use finstack_valuations::metrics::MetricId;
use time::macros::date;

#[test]
fn test_cs01_negative_for_long_bond() {
    let as_of = date!(2025 - 01 - 01);
    let mut bond = Bond::fixed(
        "CS1",
        Money::new(100.0, Currency::USD),
        0.05,
        as_of,
        date!(2030 - 01 - 01),
        "USD-OIS",
    )
    .unwrap();

    bond.credit_curve_id = Some(CurveId::new("USD-CREDIT"));

    let disc = finstack_core::market_data::term_structures::DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .knots([(0.0, 1.0), (5.0, 0.80)])
        .build()
        .unwrap();

    let hazard = HazardCurve::builder("USD-CREDIT")
        .base_date(as_of)
        .recovery_rate(0.4)
        .knots([(0.0, 0.02), (5.0, 0.02)])
        .build()
        .unwrap();

    let market = finstack_core::market_data::context::MarketContext::new()
        .insert(disc)
        .insert(hazard);

    let result = bond
        .price_with_metrics(
            &market,
            as_of,
            &[MetricId::Cs01],
            finstack_valuations::instruments::PricingOptions::default(),
        )
        .unwrap();
    let cs01 = *result.measures.get("cs01").unwrap();
    assert!(
        cs01 < 0.0,
        "Long bond CS01 should be negative (wider spreads reduce PV), got {}",
        cs01
    );
    assert!(
        cs01.abs() < 1.0,
        "Bond CS01 should be small for $100 notional, got {}",
        cs01.abs()
    );
}
