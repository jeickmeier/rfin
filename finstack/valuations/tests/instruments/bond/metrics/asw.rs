//! Tests for asset swap (ASW) metrics.

use finstack_core::currency::Currency;
use finstack_core::dates::{DayCount, Frequency};
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::MarketContext;
use finstack_core::math::interp::InterpStyle;
use finstack_core::money::Money;
use finstack_valuations::instruments::bond::metrics::price_yield_spread::asw::AssetSwapConfig;
use finstack_valuations::instruments::bond::metrics::price_yield_spread::{
    AssetSwapMarketCalculator, AssetSwapParCalculator,
};
use finstack_valuations::instruments::bond::Bond;
use finstack_valuations::metrics::{MetricCalculator, MetricContext, MetricId};
use std::sync::Arc;
use time::macros::date;

fn simple_discount_curve(id: &str, as_of: time::Date) -> DiscountCurve {
    DiscountCurve::builder(id)
        .base_date(as_of)
        .knots([(0.0, 1.0), (5.0, 0.9)])
        .set_interp(InterpStyle::LogLinear)
        .build()
        .unwrap()
}

fn simple_fixed_bond(as_of: time::Date) -> Bond {
    Bond::fixed(
        "ASW-TEST",
        Money::new(100.0, Currency::USD),
        0.05,
        as_of,
        date!(2030 - 01 - 01),
        "USD-OIS",
    )
}

#[test]
fn test_asw_market_requires_accrued_when_clean_price_present() {
    let as_of = date!(2025 - 01 - 01);
    let mut bond = simple_fixed_bond(as_of);
    // Attach a clean price so the market ASW calculator will require Accrued.
    bond.pricing_overrides = finstack_valuations::instruments::PricingOverrides::default()
        .with_clean_price(101.0);

    // Market with a simple discount curve
    let disc = simple_discount_curve("USD-OIS", as_of);
    let market = MarketContext::new().insert_discount(disc);

    // Metric context with quoted clean price but without Accrued metric
    let mut ctx = MetricContext::new(
        Arc::new(bond),
        Arc::new(market),
        as_of,
        Money::new(100.0, Currency::USD),
    );

    let calc = AssetSwapMarketCalculator::default();
    let result = calc.calculate(&mut ctx);

    match result {
        Err(e) => {
            let msg = format!("{}", e);
            assert!(
                msg.contains("metric:Accrued"),
                "expected missing Accrued error, got {}",
                msg
            );
        }
        Ok(v) => panic!(
            "expected ASW market calculation to fail without Accrued, got {}",
            v
        ),
    }
}

#[test]
fn test_asw_par_with_config_uses_fixed_leg_conventions() {
    let as_of = date!(2025 - 01 - 01);
    let bond = simple_fixed_bond(as_of);

    let disc = simple_discount_curve("USD-OIS", as_of);
    let market = MarketContext::new().insert_discount(disc);

    let mut ctx_default = MetricContext::new(
        Arc::new(bond.clone()),
        Arc::new(market.clone()),
        as_of,
        Money::new(100.0, Currency::USD),
    );
    let asw_default = AssetSwapParCalculator::default()
        .calculate(&mut ctx_default)
        .expect("ASW par with default config should succeed");

    // Override fixed-leg conventions to annual 30E/360 and verify we still get
    // a finite result and that the value changes relative to the default.
    let config = AssetSwapConfig {
        fixed_leg_day_count: Some(DayCount::ThirtyE360),
        fixed_leg_frequency: Some(Frequency::annual()),
        fixed_leg_bdc: None,
        fixed_leg_calendar_id: None,
        fixed_leg_stub: None,
    };
    let mut ctx_custom = MetricContext::new(
        Arc::new(bond),
        Arc::new(market),
        as_of,
        Money::new(100.0, Currency::USD),
    );
    let asw_custom = AssetSwapParCalculator::with_config(config)
        .calculate(&mut ctx_custom)
        .expect("ASW par with custom config should succeed");

    assert!(
        asw_custom.is_finite(),
        "ASW par with custom config should be finite"
    );
    assert!(
        (asw_custom - asw_default).abs() > 1e-12,
        "ASW par with custom conventions should differ from default"
    );
}


