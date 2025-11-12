//! Integration tests for FX attribution.
//!
//! Tests FX translation and internal FX exposure effects in parallel
//! and waterfall attribution methodologies.

use finstack_core::currency::Currency;
use finstack_core::dates::create_date;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::MarketContext;
use finstack_core::math::interp::InterpStyle;
use finstack_core::money::fx::{FxConversionPolicy, FxMatrix, FxProvider};
use finstack_core::money::Money;
use finstack_core::prelude::FinstackConfig;
use finstack_valuations::attribution::{
    attribute_pnl_parallel, attribute_pnl_waterfall, default_waterfall_order, AttributionFactor,
};
use finstack_valuations::instruments::bond::Bond;
use finstack_valuations::instruments::Instrument;
use std::sync::Arc;
use time::Month;

// Test FX provider with configurable rates
struct TestFxProvider {
    rate: f64,
}

impl FxProvider for TestFxProvider {
    fn rate(
        &self,
        from: Currency,
        to: Currency,
        _on: finstack_core::dates::Date,
        _policy: FxConversionPolicy,
    ) -> finstack_core::Result<f64> {
        if from == to {
            Ok(1.0)
        } else if from == Currency::EUR && to == Currency::USD {
            Ok(self.rate)
        } else if from == Currency::USD && to == Currency::EUR {
            Ok(1.0 / self.rate)
        } else {
            Err(finstack_core::Error::Validation(
                "FX rate not found".to_string(),
            ))
        }
    }
}

#[test]
fn test_fx_attribution_parallel_internal_exposure() {
    // Test that FX attribution captures internal FX exposure changes
    // For a USD bond (no cross-currency exposure), FX P&L should be near zero
    let as_of_t0 = create_date(2025, Month::January, 15).unwrap();
    let as_of_t1 = create_date(2025, Month::January, 16).unwrap();

    // Create a USD bond (no FX exposure)
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

    // Create discount curves (unchanged)
    let curve_t0 = DiscountCurve::builder("USD-OIS")
        .base_date(as_of_t0)
        .knots(vec![(0.0, 1.0), (5.0, 0.82)])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();

    let curve_t1 = DiscountCurve::builder("USD-OIS")
        .base_date(as_of_t1)
        .knots(vec![(0.0, 1.0), (5.0, 0.82)])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();

    // Create FX matrices with different EUR/USD rates
    let fx_t0 = FxMatrix::new(Arc::new(TestFxProvider { rate: 1.1 }));
    let fx_t1 = FxMatrix::new(Arc::new(TestFxProvider { rate: 1.2 }));

    let market_t0 = MarketContext::new()
        .insert_discount(curve_t0)
        .insert_fx(fx_t0);

    let market_t1 = MarketContext::new()
        .insert_discount(curve_t1)
        .insert_fx(fx_t1);

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
    )
    .unwrap();

    // For USD bond with no cross-currency exposure, FX P&L should be minimal
    // (Only internal pricing effects, which are zero for single-currency bond)
    let fx_pnl_abs = attribution.fx_pnl.amount().abs();
    assert!(
        fx_pnl_abs < 0.01,
        "FX P&L should be near zero for single-currency bond, got {}",
        fx_pnl_abs
    );

    // Metadata should be populated
    assert_eq!(attribution.meta.instrument_id, "US-BOND-001");
    assert!(attribution.meta.num_repricings > 0);
}

#[test]
fn test_waterfall_attribution_sum_equality() {
    // Test that waterfall attribution sums to total P&L
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

    // Create curves with a shift
    let curve_t0 = DiscountCurve::builder("USD-OIS")
        .base_date(as_of_t0)
        .knots(vec![(0.0, 1.0), (5.0, 0.82)])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();

    let curve_t1 = DiscountCurve::builder("USD-OIS")
        .base_date(as_of_t1)
        .knots(vec![(0.0, 1.0), (5.0, 0.78)]) // Rates increased
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();

    let market_t0 = MarketContext::new().insert_discount(curve_t0);
    let market_t1 = MarketContext::new().insert_discount(curve_t1);

    let config = FinstackConfig::default();

    // Run waterfall attribution
    let bond_instrument: Arc<dyn Instrument> = Arc::new(bond);
    let attribution = attribute_pnl_waterfall(
        &bond_instrument,
        &market_t0,
        &market_t1,
        as_of_t0,
        as_of_t1,
        &config,
        default_waterfall_order(),
    )
    .unwrap();

    // Sum of all factors should equal total P&L (within tolerance)
    let sum_of_factors = attribution.carry.amount()
        + attribution.rates_curves_pnl.amount()
        + attribution.credit_curves_pnl.amount()
        + attribution.inflation_curves_pnl.amount()
        + attribution.correlations_pnl.amount()
        + attribution.fx_pnl.amount()
        + attribution.vol_pnl.amount()
        + attribution.model_params_pnl.amount()
        + attribution.market_scalars_pnl.amount();

    let expected_total = attribution.total_pnl.amount() - attribution.residual.amount();
    let diff = (sum_of_factors - expected_total).abs();

    assert!(
        diff < 0.01,
        "Sum of factors ({}) should equal total - residual ({}), diff = {}",
        sum_of_factors,
        expected_total,
        diff
    );

    // Residual should be very small for waterfall
    assert!(attribution.residual_within_meta_tolerance());
}

#[test]
fn test_waterfall_factor_ordering_sensitivity() {
    // Test that different factor orders produce different attributions
    // but same total P&L and minimal residual
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

    let curve_t0 = DiscountCurve::builder("USD-OIS")
        .base_date(as_of_t0)
        .knots(vec![(0.0, 1.0), (5.0, 0.82)])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();

    let curve_t1 = DiscountCurve::builder("USD-OIS")
        .base_date(as_of_t1)
        .knots(vec![(0.0, 1.0), (5.0, 0.78)])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();

    let market_t0 = MarketContext::new().insert_discount(curve_t0);
    let market_t1 = MarketContext::new().insert_discount(curve_t1);

    let config = FinstackConfig::default();
    let bond_instrument: Arc<dyn Instrument> = Arc::new(bond);

    // Order 1: Carry then Rates
    let order1 = vec![AttributionFactor::Carry, AttributionFactor::RatesCurves];

    let attr1 = attribute_pnl_waterfall(
        &bond_instrument,
        &market_t0,
        &market_t1,
        as_of_t0,
        as_of_t1,
        &config,
        order1,
    )
    .unwrap();

    // Order 2: Rates then Carry
    let order2 = vec![AttributionFactor::RatesCurves, AttributionFactor::Carry];

    let attr2 = attribute_pnl_waterfall(
        &bond_instrument,
        &market_t0,
        &market_t1,
        as_of_t0,
        as_of_t1,
        &config,
        order2,
    )
    .unwrap();

    // Total P&L should be identical
    assert_eq!(attr1.total_pnl.amount(), attr2.total_pnl.amount());

    // Both should have minimal residual
    assert!(attr1.residual_within_meta_tolerance());
    assert!(attr2.residual_within_meta_tolerance());

    // Factor attributions may differ due to ordering
    // (This is expected and correct for waterfall methodology)
}

