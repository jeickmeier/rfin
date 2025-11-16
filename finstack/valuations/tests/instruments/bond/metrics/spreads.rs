//! Z-spread and I-spread calculator tests.

use finstack_core::currency::Currency;
use finstack_core::error::{Error, InputError};
use finstack_core::money::Money;
use finstack_valuations::instruments::bond::metrics::ZSpreadCalculator;
use finstack_valuations::instruments::bond::Bond;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::instruments::PricingOverrides;
use finstack_valuations::metrics::{MetricCalculator, MetricContext, MetricId};
use std::sync::Arc;
use time::macros::date;

#[test]
fn test_z_spread_discount_bond() {
    let as_of = date!(2025 - 01 - 01);
    let mut bond = Bond::fixed(
        "ZSPR1",
        Money::new(100.0, Currency::USD),
        0.05,
        as_of,
        date!(2030 - 01 - 01),
        "USD-OIS",
    );
    bond.pricing_overrides = PricingOverrides::default().with_clean_price(95.0);

    let curve = finstack_core::market_data::term_structures::DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .knots([(0.0, 1.0), (5.0, 0.80)])
        .build()
        .unwrap();
    let market = finstack_core::market_data::context::MarketContext::new().insert_discount(curve);

    let result = bond
        .price_with_metrics(&market, as_of, &[MetricId::ZSpread])
        .unwrap();
    let z = *result.measures.get("z_spread").unwrap();
    assert!(z > 0.0); // Discount bond has positive spread
}

/// Z-spread should surface a missing discount curve error instead of silently returning 0.0
/// when pricing fails inside the root-finding objective (e.g., missing discount curve).
#[test]
fn test_z_spread_missing_discount_curve_returns_error() {
    let as_of = date!(2025 - 01 - 01);
    let mut bond = Bond::fixed(
        "ZSPR-MISSING-DC",
        Money::new(100.0, Currency::USD),
        0.05,
        as_of,
        date!(2030 - 01 - 01),
        "USD-OIS",
    );
    bond.pricing_overrides = PricingOverrides::default().with_clean_price(95.0);

    // Market context with NO discount curves – any attempt to build a Z-spread PV should fail
    let market = finstack_core::market_data::context::MarketContext::new();

    // Minimal metric context: base value is arbitrary since Z-spread uses quoted clean price
    let base_value = Money::new(100.0, Currency::USD);
    let mut mctx =
        MetricContext::new(Arc::new(bond), Arc::new(market), as_of, base_value);

    // Pre-populate accrued to bypass the metric dependency and force the failure into
    // the Z-spread pricing helper (missing discount curve), not missing accrued.
    mctx.computed.insert(MetricId::Accrued, 0.0);

    let calc = ZSpreadCalculator;
    let result = calc.calculate(&mut mctx);

    // Expect a propagated input error (missing discount curve), never an apparent "perfect fit" z=0.0.
    match result {
        Err(Error::Input(InputError::NotFound { id })) => {
            assert!(
                id.contains("USD-OIS"),
                "expected missing discount curve id to mention USD-OIS, got {}",
                id
            );
        }
        Err(e) => panic!("expected InputError::NotFound for missing discount curve, got {}", e),
        Ok(z) => panic!(
            "expected Z-spread calculation to fail for missing discount curve, but got z={}",
            z
        ),
    }
}
