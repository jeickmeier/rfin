//! Discount margin calculator tests.

use finstack_core::currency::Currency;
use finstack_core::error::{Error, InputError};
use finstack_core::money::Money;
use finstack_valuations::instruments::bond::Bond;
use finstack_valuations::instruments::bond::metrics::DiscountMarginCalculator;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::metrics::{MetricCalculator, MetricContext, MetricId};
use std::sync::Arc;
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

/// DM should surface a missing discount curve error instead of silently returning 0.0
/// when pricing fails inside the root-finding objective (e.g., missing discount curve).
#[test]
fn test_dm_missing_forward_curve_returns_error() {
    let as_of = date!(2025 - 01 - 01);

    // Floating-rate bond referencing a discount curve that will be missing in the market
    let bond = Bond::floating(
        "DM-FRN-MISSING-FWD",
        Money::new(100.0, Currency::USD),
        "USD-SOFR-3M",
        200.0,
        as_of,
        date!(2030 - 01 - 01),
        finstack_core::dates::Frequency::quarterly(),
        finstack_core::dates::DayCount::Act360,
        "USD-OIS",
    );

    // Market with NO discount curves – any attempt to price from DM should fail
    let market = finstack_core::market_data::context::MarketContext::new();

    // Build a minimal metric context without relying on successful base pricing;
    // base_value is arbitrary here since we're testing failure in the DM objective.
    let base_value = Money::new(100.0, Currency::USD);
    let mut mctx =
        MetricContext::new(Arc::new(bond), Arc::new(market), as_of, base_value);

    // No need to pre-compute Accrued; DM calculator will treat missing accrued as 0.
    let calc = DiscountMarginCalculator;
    let result = calc.calculate(&mut mctx);

    // Expect a propagated input error (missing discount curve), never an apparent "perfect fit" DM of 0.0.
    match result {
        Err(Error::Input(InputError::NotFound { id })) => {
            assert!(
                id.contains("USD-OIS"),
                "expected missing discount curve id to mention USD-OIS, got {}",
                id
            );
        }
        Err(e) => panic!("expected InputError::NotFound for missing discount curve, got {}", e),
        Ok(dm) => panic!(
            "expected DM calculation to fail for missing discount curve, but got DM={}",
            dm
        ),
    }
}
