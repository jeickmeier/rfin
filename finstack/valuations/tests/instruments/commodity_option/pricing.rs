//! Pricing tests for commodity options.

use finstack_core::currency::Currency;
use finstack_core::dates::{DayCount, DayCountCtx};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::scalars::MarketScalar;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::commodity_option::CommodityOption;
use finstack_valuations::instruments::common::models::bs_price;
use finstack_valuations::instruments::common::traits::Attributes;
use finstack_valuations::instruments::{ExerciseStyle, OptionType, PricingOverrides, SettlementType};
use finstack_valuations::test_utils::{
    date, flat_discount_with_tenor, flat_forward_with_tenor, flat_vol_surface,
};

#[test]
fn test_black76_futures_based_pricing() {
    let as_of = date(2025, 1, 1);
    let expiry = date(2026, 1, 1);

    let discount_curve = flat_discount_with_tenor("USD-OIS", as_of, 0.03, 1.0);
    let forward_curve = flat_forward_with_tenor("CL-FWD", as_of, 0.0, 1.0);
    let vol_surface = flat_vol_surface("CL-VOL", &[1.0], &[80.0, 100.0, 120.0], 0.20);

    let market = MarketContext::new()
        .insert_discount(discount_curve)
        .insert_forward(forward_curve)
        .insert_surface(vol_surface);

    let option = CommodityOption::builder()
        .id(InstrumentId::new("CL-CALL-TEST"))
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
        .day_count(DayCount::Act365F)
        .pricing_overrides(PricingOverrides::default())
        .attributes(Attributes::new())
        .build()
        .expect("should build");

    let pv = option.npv(&market, as_of).expect("should price");
    let t = DayCount::Act365F
        .year_fraction(as_of, expiry, DayCountCtx::default())
        .expect("year fraction");
    let forward = option.forward_price(&market, as_of).expect("forward");
    let df = (-0.03 * t).exp();
    let expected = bs_price(forward, 100.0, 0.0, 0.0, 0.20, t, OptionType::Call) * df;

    // Allow small tolerance for day count/interpolation differences
    assert!(
        (pv.amount() - expected).abs() < 0.01,
        "PV mismatch: {} vs expected {}",
        pv.amount(),
        expected
    );
}

#[test]
fn test_futures_based_american_matches_european() {
    let as_of = date(2025, 1, 1);
    let expiry = date(2026, 1, 1);

    let discount_curve = flat_discount_with_tenor("USD-OIS", as_of, 0.02, 1.0);
    let forward_curve = flat_forward_with_tenor("CL-FWD", as_of, 0.0, 1.0);
    let vol_surface = flat_vol_surface("CL-VOL", &[1.0], &[90.0, 100.0, 110.0], 0.25);

    let market = MarketContext::new()
        .insert_discount(discount_curve)
        .insert_forward(forward_curve)
        .insert_surface(vol_surface);

    let build = |style| {
        CommodityOption::builder()
            .id(InstrumentId::new("CL-CALL-BASE"))
            .commodity_type("Energy".to_string())
            .ticker("CL".to_string())
            .strike(100.0)
            .option_type(OptionType::Call)
            .exercise_style(style)
            .expiry(expiry)
            .quantity(1.0)
            .unit("BBL".to_string())
            .multiplier(1.0)
            .settlement(SettlementType::Cash)
            .currency(Currency::USD)
            .forward_curve_id(CurveId::new("CL-FWD"))
            .discount_curve_id(CurveId::new("USD-OIS"))
            .vol_surface_id(CurveId::new("CL-VOL"))
            .day_count(DayCount::Act365F)
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("should build")
    };

    let european = build(ExerciseStyle::European);
    let american = build(ExerciseStyle::American);

    let pv_eur = european.npv(&market, as_of).expect("price european");
    let pv_amer = american.npv(&market, as_of).expect("price american");

    // American call on futures should be close to European (no early exercise value)
    // Allow wider tolerance for tree vs closed-form numerical differences
    assert!(
        (pv_amer.amount() - pv_eur.amount()).abs() < 0.1,
        "American={} vs European={}",
        pv_amer.amount(),
        pv_eur.amount()
    );
}

#[test]
fn test_spot_based_american_put_above_european() {
    let as_of = date(2025, 1, 1);
    let expiry = date(2026, 1, 1);

    let discount_curve = flat_discount_with_tenor("USD-OIS", as_of, 0.05, 1.0);
    let forward_curve = flat_forward_with_tenor("CL-FWD", as_of, 0.02, 1.0);
    let vol_surface = flat_vol_surface("CL-VOL", &[1.0], &[80.0, 100.0, 120.0], 0.30);

    let market = MarketContext::new()
        .insert_discount(discount_curve)
        .insert_forward(forward_curve)
        .insert_surface(vol_surface)
        .insert_price("CL-SPOT", MarketScalar::Unitless(90.0));

    let build = |style| {
        CommodityOption::builder()
            .id(InstrumentId::new("CL-PUT-BASE"))
            .commodity_type("Energy".to_string())
            .ticker("CL".to_string())
            .strike(100.0)
            .option_type(OptionType::Put)
            .exercise_style(style)
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
            .day_count(DayCount::Act365F)
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("should build")
    };

    let european = build(ExerciseStyle::European);
    let american = build(ExerciseStyle::American);

    let pv_eur = european.npv(&market, as_of).expect("price european");
    let pv_amer = american.npv(&market, as_of).expect("price american");

    assert!(pv_amer.amount() + 1e-6 >= pv_eur.amount());
}
