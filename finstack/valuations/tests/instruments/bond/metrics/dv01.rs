//! DV01 calculator tests.

use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_valuations::instruments::bond::Bond;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::metrics::MetricId;
use time::macros::date;

#[test]
fn test_dv01_signed_negative_for_bond() {
    let as_of = date!(2025 - 01 - 01);
    let bond = Bond::fixed(
        "DV01_1",
        Money::new(100.0, Currency::USD),
        0.05,
        as_of,
        date!(2030 - 01 - 01),
        "USD-OIS",
    );

    let curve = finstack_core::market_data::term_structures::DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .knots([(0.0, 1.0), (5.0, 0.80)])
        .build()
        .unwrap();
    let market = finstack_core::market_data::context::MarketContext::new().insert_discount(curve);

    let result = bond
        .price_with_metrics(&market, as_of, &[MetricId::Dv01])
        .unwrap();
    let dv01 = *result.measures.get("dv01").unwrap();
    // Signed convention: bonds lose value when rates rise ⇒ DV01 < 0
    assert!(dv01 < 0.0 && dv01 > -1.0, "DV01 = {}, expected negative between 0 and -1", dv01); // Reasonable magnitude for 5Y bond
}
