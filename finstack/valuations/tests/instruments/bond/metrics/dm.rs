//! Discount margin calculator tests.

use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_valuations::instruments::bond::Bond;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::metrics::MetricId;
use time::macros::date;

#[test]
fn test_dm_fixed_bond_zero() {
    let as_of = date!(2025 - 01 - 01);
    let bond = Bond::fixed(
        "DM1",
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
        .price_with_metrics(&market, as_of, &[MetricId::DiscountMargin])
        .unwrap();
    let dm = *result.measures.get("discount_margin").unwrap();
    assert_eq!(dm, 0.0); // Fixed bonds return 0 DM
}
