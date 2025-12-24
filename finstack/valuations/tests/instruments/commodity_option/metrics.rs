//! Metrics tests for commodity options.

use finstack_core::currency::Currency;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::scalars::MarketScalar;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::commodity_option::CommodityOption;
use finstack_valuations::instruments::common::traits::Attributes;
use finstack_valuations::instruments::{ExerciseStyle, OptionType, PricingOverrides, SettlementType};
use finstack_valuations::instruments::Instrument;
use finstack_valuations::metrics::{standard_registry, MetricContext, MetricId};
use finstack_valuations::test_utils::{
    date, flat_discount_with_tenor, flat_forward_with_tenor, flat_vol_surface,
};
use std::sync::Arc;

#[test]
fn test_commodity_option_core_greeks_registered() -> finstack_core::Result<()> {
    let as_of = date(2025, 1, 1);
    let expiry = date(2026, 1, 1);

    let discount_curve = flat_discount_with_tenor("USD-OIS", as_of, 0.03, 2.0);
    let forward_curve = flat_forward_with_tenor("CL-FWD", as_of, 0.0, 2.0);
    let vol_surface = flat_vol_surface("CL-VOL", &[1.0], &[80.0, 100.0, 120.0], 0.20);

    let market = MarketContext::new()
        .insert_discount(discount_curve)
        .insert_forward(forward_curve)
        .insert_surface(vol_surface)
        .insert_price("CL-SPOT", MarketScalar::Unitless(100.0));

    let option = CommodityOption::builder()
        .id(InstrumentId::new("CL-CALL-GREeks"))
        .commodity_type("Energy".to_string())
        .ticker("CL".to_string())
        .strike(100.0)
        .option_type(OptionType::Call)
        .exercise_style(ExerciseStyle::European)
        .expiry(expiry)
        .quantity(1.0)
        .unit("BBL".to_string())
        .multiplier(1.0)
        .settlement(SettlementType::Cash)
        .currency(Currency::USD)
        .forward_curve_id(CurveId::new("CL-FWD"))
        .discount_curve_id(CurveId::new("USD-OIS"))
        .vol_surface_id(CurveId::new("CL-VOL"))
        .spot_price_id_opt(Some("CL-SPOT".to_string()))
        .day_count(finstack_core::dates::DayCount::Act365F)
        .pricing_overrides(PricingOverrides::default())
        .attributes(Attributes::new())
        .build()
        .expect("should build");

    let pv = option.value(&market, as_of)?;
    let mut ctx = MetricContext::new(Arc::new(option), Arc::new(market), as_of, pv);
    let registry = standard_registry();
    let res = registry.compute(&[MetricId::Gamma, MetricId::Vanna, MetricId::Volga], &mut ctx)?;

    let gamma = *res.get(&MetricId::Gamma).expect("gamma");
    let vanna = *res.get(&MetricId::Vanna).expect("vanna");
    let volga = *res.get(&MetricId::Volga).expect("volga");

    assert!(gamma.is_finite(), "gamma should be finite");
    assert!(vanna.is_finite(), "vanna should be finite");
    assert!(volga.is_finite(), "volga should be finite");
    assert!(gamma >= -1e-8, "gamma should be non-negative");

    Ok(())
}
