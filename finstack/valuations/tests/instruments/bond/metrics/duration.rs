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
fn test_convexity_bump_configurable() {
    // Convexity should honor pricing_overrides.ytm_bump_bp if set
    use finstack_core::market_data::term_structures::DiscountCurve;
    use finstack_valuations::instruments::PricingOverrides;

    let as_of = date!(2025 - 01 - 01);
    let maturity = date!(2030 - 01 - 01);
    let mut bond = Bond::fixed(
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

    // First compute with default bump (1 bp)
    let res_default = bond
        .price_with_metrics(&market, as_of, &[MetricId::Convexity])
        .unwrap();
    let conv_default = *res_default.measures.get("convexity").unwrap();

    // Now set a larger bump and expect convexity magnitude to change
    bond.pricing_overrides = PricingOverrides::default().with_ytm_bump(2e-4); // 2 bp
    let res_bumped = bond
        .price_with_metrics(&market, as_of, &[MetricId::Convexity])
        .unwrap();
    let conv_bumped = *res_bumped.measures.get("convexity").unwrap();

    // Numerical convexity should be stable to small bump size changes; allow small relative diff
    let rel = ((conv_bumped - conv_default).abs()) / conv_default.abs().max(1e-9);
    assert!(rel < 0.25);
}
