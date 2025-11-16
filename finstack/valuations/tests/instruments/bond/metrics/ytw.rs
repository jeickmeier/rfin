//! Yield to worst calculator tests.

use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_valuations::instruments::bond::{Bond, CallPut, CallPutSchedule};
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::instruments::PricingOverrides;
use finstack_valuations::metrics::MetricId;
use time::macros::date;

#[test]
fn test_ytw_equals_ytm_for_non_callable_bond_from_price() {
    let as_of = date!(2025 - 01 - 01);
    let mut bond = Bond::fixed(
        "YTW_NON_CALL",
        Money::new(100.0, Currency::USD),
        0.05,
        as_of,
        date!(2030 - 01 - 01),
        "USD-OIS",
    );
    // Market-quoted clean price (percent of par)
    bond.pricing_overrides = PricingOverrides::default().with_clean_price(99.5);

    let curve = finstack_core::market_data::term_structures::DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .knots([(0.0, 1.0), (5.0, 0.80)])
        .build()
        .unwrap();
    let market = finstack_core::market_data::context::MarketContext::new().insert_discount(curve);

    // Request both YTM and YTW so they are computed off the same quoted price
    let result = bond
        .price_with_metrics(&market, as_of, &[MetricId::Ytm, MetricId::Ytw])
        .unwrap();
    let ytm = *result.measures.get("ytm").unwrap();
    let ytw = *result.measures.get("ytw").unwrap();

    // For a non-callable bond, YTW should collapse to YTM (same cashflows/price)
    assert!(ytw.is_finite());
    assert!(
        (ytw - ytm).abs() <= 1e-6,
        "expected YTW ≈ YTM for non-callable bond, got ytm={} ytw={}",
        ytm,
        ytw
    );
}

#[test]
fn test_ytw_not_greater_than_ytm_for_callable_bond_from_price() {
    let as_of = date!(2025 - 01 - 01);
    let mut bond = Bond::fixed(
        "YTW_CALL",
        Money::new(100.0, Currency::USD),
        0.05,
        as_of,
        date!(2030 - 01 - 01),
        "USD-OIS",
    );
    // Add a single call prior to maturity
    let call_date = date!(2028 - 01 - 01);
    bond.call_put = Some(CallPutSchedule {
        calls: vec![CallPut {
            date: call_date,
            price_pct_of_par: 100.0,
        }],
        puts: Vec::new(),
    });
    // Premium market price to make the call path potentially worse for the holder
    bond.pricing_overrides = PricingOverrides::default().with_clean_price(105.0);

    let curve = finstack_core::market_data::term_structures::DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .knots([(0.0, 1.0), (5.0, 0.80)])
        .build()
        .unwrap();
    let market = finstack_core::market_data::context::MarketContext::new().insert_discount(curve);

    let result = bond
        .price_with_metrics(&market, as_of, &[MetricId::Ytm, MetricId::Ytw])
        .unwrap();
    let ytm = *result.measures.get("ytm").unwrap();
    let ytw = *result.measures.get("ytw").unwrap();

    // By construction, YTW takes the minimum yield across call/maturity paths,
    // so it should never exceed YTM (which is one of the candidates).
    assert!(ytm.is_finite() && ytw.is_finite());
    assert!(
        ytw <= ytm + 1e-6,
        "expected YTW <= YTM for callable bond, got ytm={} ytw={}",
        ytm,
        ytw
    );
}

#[test]
fn test_ytw_tracks_quoted_price_not_model_pv() {
    let as_of = date!(2025 - 01 - 01);
    let mut bond = Bond::fixed(
        "YTW_PRICE_SENSITIVE",
        Money::new(100.0, Currency::USD),
        0.04,
        as_of,
        date!(2030 - 01 - 01),
        "USD-OIS",
    );

    let curve = finstack_core::market_data::term_structures::DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .knots([(0.0, 1.0), (5.0, 0.90)]) // deliberately simple curve
        .build()
        .unwrap();
    let market = finstack_core::market_data::context::MarketContext::new().insert_discount(curve);

    // Compute model PV for reference (should differ from at least one of the quoted prices)
    let pv = bond.value(&market, as_of).unwrap().amount();
    assert!(pv.is_finite());

    // Two different quoted clean prices with the same curves
    bond.pricing_overrides = PricingOverrides::default().with_clean_price(95.0);
    let result_low = bond
        .price_with_metrics(&market, as_of, &[MetricId::Ytw])
        .unwrap();
    let ytw_low = *result_low.measures.get("ytw").unwrap();

    bond.pricing_overrides = PricingOverrides::default().with_clean_price(105.0);
    let result_high = bond
        .price_with_metrics(&market, as_of, &[MetricId::Ytw])
        .unwrap();
    let ytw_high = *result_high.measures.get("ytw").unwrap();

    // YTW must respond to the quoted price overrides, not stay tied to model PV.
    assert!(
        (ytw_low - ytw_high).abs() > 1e-4,
        "expected YTW to differ for misaligned quoted prices; pv={} ytw_low={} ytw_high={}",
        pv,
        ytw_low,
        ytw_high
    );
}
