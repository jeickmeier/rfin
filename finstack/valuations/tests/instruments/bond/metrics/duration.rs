//! Duration calculator tests (Macaulay and Modified).

use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_valuations::cashflow::traits::CashflowProvider;
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

#[test]
fn test_modified_duration_matches_macaulay_over_yield() {
    // For a simple fixed bond, Duration_mod ≈ Duration_mac / (1 + y/m)
    use finstack_core::dates::DayCount;
    use finstack_core::market_data::term_structures::DiscountCurve;

    let as_of = date!(2025 - 01 - 01);
    let maturity = date!(2030 - 01 - 01);
    let bond = Bond::fixed(
        "DUR2",
        Money::new(100.0, Currency::USD),
        0.06,
        as_of,
        maturity,
        "USD-OIS",
    );

    // Flat 5% curve
    let curve = DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .day_count(DayCount::Act360)
        .knots([(0.0, 1.0), (10.0, (-(0.05_f64 * 10.0_f64)).exp())])
        .build()
        .unwrap();
    let market = finstack_core::market_data::context::MarketContext::new().insert_discount(curve);

    let res = bond
        .price_with_metrics(
            &market,
            as_of,
            &[MetricId::Ytm, MetricId::DurationMac, MetricId::DurationMod],
        )
        .unwrap();
    let ytm = *res.measures.get("ytm").unwrap();
    let d_mac = *res.measures.get("duration_mac").unwrap();
    let d_mod = *res.measures.get("duration_mod").unwrap();

    // Semiannual frequency → m = 2 by default in helper
    let expected = d_mac / (1.0 + ytm / 2.0);
    assert!((d_mod - expected).abs() < 0.05);
}

#[test]
fn test_convexity_matches_numerical_second_derivative() {
    // Closed-form convexity should align with a numerical second derivative.
    use finstack_core::market_data::term_structures::DiscountCurve;

    let as_of = date!(2025 - 01 - 01);
    let maturity = date!(2030 - 01 - 01);
    let bond = Bond::fixed(
        "CONV1",
        Money::new(100.0, Currency::USD),
        0.05,
        as_of,
        maturity,
        "USD-OIS",
    );

    let curve = DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .knots([(0.0, 1.0), (10.0, 0.6)])
        .build()
        .unwrap();
    let market = finstack_core::market_data::context::MarketContext::new().insert_discount(curve);

    let res = bond
        .price_with_metrics(&market, as_of, &[MetricId::Ytm, MetricId::Convexity])
        .unwrap();
    let ytm = *res.measures.get("ytm").unwrap();
    let conv_closed = *res.measures.get("convexity").unwrap();

    let flows = bond.build_schedule(&market, as_of).unwrap();
    let dy = 1e-4;
    let p0 = finstack_valuations::instruments::bond::pricing::quote_engine::price_from_ytm(
        &bond, &flows, as_of, ytm,
    )
    .unwrap();
    let p_up = finstack_valuations::instruments::bond::pricing::quote_engine::price_from_ytm(
        &bond,
        &flows,
        as_of,
        ytm + dy,
    )
    .unwrap();
    let p_dn = finstack_valuations::instruments::bond::pricing::quote_engine::price_from_ytm(
        &bond,
        &flows,
        as_of,
        ytm - dy,
    )
    .unwrap();
    let conv_numeric = (p_up + p_dn - 2.0 * p0) / (p0 * dy * dy);

    let rel = (conv_closed - conv_numeric).abs() / conv_numeric.abs().max(1e-9);
    assert!(rel < 1e-3, "convexity rel diff {}", rel);
}
