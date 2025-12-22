//! Integration test for bond P&L attribution.
//!
//! Tests attribution across carry, curve shifts, and other factors for a
//! simple fixed-rate bond.

use finstack_core::currency::Currency;
use finstack_core::dates::create_date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::math::interp::InterpStyle;
use finstack_core::config::FinstackConfig;
use finstack_core::money::Money;
use finstack_valuations::attribution::{attribute_pnl_parallel, AttributionMethod};
use finstack_valuations::instruments::bond::Bond;
use finstack_valuations::instruments::Instrument;
use std::sync::Arc;
use time::Month;

#[test]
fn test_bond_attribution_parallel() {
    let as_of_t0 = create_date(2025, Month::January, 15).unwrap();
    let as_of_t1 = create_date(2025, Month::January, 16).unwrap();

    // Create a 5-year bond
    let issue = create_date(2025, Month::January, 1).unwrap();
    let maturity = create_date(2030, Month::January, 1).unwrap();

    let bond = Bond::fixed(
        "US-BOND-001",
        Money::new(1_000_000.0, Currency::USD),
        0.05, // 5% coupon
        issue,
        maturity,
        "USD-OIS",
    );

    // Create discount curve at T₀ (flat 4%)
    let curve_t0 = DiscountCurve::builder("USD-OIS")
        .base_date(as_of_t0)
        .knots(vec![(0.0, 1.0), (5.0, 0.82)])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();

    // Create discount curve at T₁ (rates increased to 5%)
    let curve_t1 = DiscountCurve::builder("USD-OIS")
        .base_date(as_of_t1)
        .knots(vec![(0.0, 1.0), (5.0, 0.78)])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();

    let market_t0 = MarketContext::new().insert_discount(curve_t0);
    let market_t1 = MarketContext::new().insert_discount(curve_t1);

    let config = FinstackConfig::default();

    // Run parallel attribution
    let bond_instrument: Arc<dyn Instrument> = Arc::new(bond);
    let attribution = attribute_pnl_parallel(
        &bond_instrument,
        &market_t0,
        &market_t1,
        as_of_t0,
        as_of_t1,
        &config,
        None,
    )
    .unwrap();

    // Should have some P&L from curve shift
    assert_ne!(attribution.total_pnl.amount(), 0.0);

    // Currency must be USD
    assert_eq!(attribution.total_pnl.currency(), Currency::USD);

    // Directional assertions for market-standard bond behavior:
    //
    // 1. Carry should be non-negative for a coupon-bearing bond
    //    (bond earns coupon income over time, though may be small for 1-day period)
    assert!(
        attribution.carry.amount() >= -0.01,
        "Carry should be non-negative for coupon bond, got {}",
        attribution.carry.amount()
    );

    // 2. Rates increased (DF at 5Y went from 0.82 to 0.78, implying higher rates)
    //    Bond value should decrease when rates increase → negative rates P&L
    assert!(
        attribution.rates_curves_pnl.amount() < 0.0,
        "Rates P&L should be negative when rates increase (bond value decreases), got {}",
        attribution.rates_curves_pnl.amount()
    );

    // 3. Residual should be small for a simple bond with single-factor attribution
    // For parallel attribution, market-standard tolerance is:
    // - Single factor (rates only): < 1%
    // - Multiple interacting factors: < 5%
    // - Large market moves with convexity: < 10%
    let residual_pct = attribution.meta.residual_pct.abs();
    assert!(
        residual_pct < 5.0,
        "Residual percentage should be < 5% for parallel attribution, got {:.2}%",
        residual_pct
    );
}

#[test]
fn test_bond_attribution_structure() {
    // Test that attribution structure is correctly populated
    let as_of_t0 = create_date(2025, Month::January, 15).unwrap();
    let as_of_t1 = create_date(2025, Month::January, 16).unwrap();

    let issue = create_date(2025, Month::January, 1).unwrap();
    let maturity = create_date(2030, Month::January, 1).unwrap();

    let bond = Bond::fixed(
        "US-BOND-001",
        Money::new(1_000_000.0, Currency::USD),
        0.05,
        issue,
        maturity,
        "USD-OIS",
    );

    let curve = DiscountCurve::builder("USD-OIS")
        .base_date(as_of_t0)
        .knots(vec![(0.0, 1.0), (5.0, 0.82)])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();

    let market_t0 = MarketContext::new().insert_discount(curve);
    let market_t1 = market_t0.clone();

    let config = FinstackConfig::default();

    let bond_instrument: Arc<dyn Instrument> = Arc::new(bond);
    let attribution = attribute_pnl_parallel(
        &bond_instrument,
        &market_t0,
        &market_t1,
        as_of_t0,
        as_of_t1,
        &config,
        None,
    )
    .unwrap();

    // Check structure
    assert_eq!(attribution.meta.instrument_id, "US-BOND-001");
    assert_eq!(attribution.meta.t0, as_of_t0);
    assert_eq!(attribution.meta.t1, as_of_t1);
    assert!(matches!(
        attribution.meta.method,
        AttributionMethod::Parallel
    ));
}
