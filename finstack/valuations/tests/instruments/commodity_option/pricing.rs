//! Pricing tests for commodity options.

use crate::finstack_test_utils::{
    date, flat_discount_with_tenor, flat_price_curve, flat_vol_surface,
};
use finstack_core::currency::Currency;
use finstack_core::dates::{DayCount, DayCountCtx};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::scalars::MarketScalar;
use finstack_core::math::norm_cdf;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::commodity::commodity_option::CommodityOption;
use finstack_valuations::instruments::Attributes;
use finstack_valuations::instruments::Instrument;
use finstack_valuations::instruments::{
    ExerciseStyle, OptionType, PricingOverrides, SettlementType,
};

fn bs_price(
    spot: f64,
    strike: f64,
    r: f64,
    q: f64,
    sigma: f64,
    t: f64,
    option_type: OptionType,
) -> f64 {
    if t <= 0.0 || sigma <= 0.0 {
        return match option_type {
            OptionType::Call => (spot - strike).max(0.0),
            OptionType::Put => (strike - spot).max(0.0),
        };
    }
    let sqrt_t = t.sqrt();
    let d1 = ((spot / strike).ln() + (r - q + 0.5 * sigma * sigma) * t) / (sigma * sqrt_t);
    let d2 = d1 - sigma * sqrt_t;
    let disc_q = (-q * t).exp();
    let disc_r = (-r * t).exp();

    match option_type {
        OptionType::Call => spot * disc_q * norm_cdf(d1) - strike * disc_r * norm_cdf(d2),
        OptionType::Put => strike * disc_r * norm_cdf(-d2) - spot * disc_q * norm_cdf(-d1),
    }
}

#[test]
fn test_black76_futures_based_pricing() {
    let as_of = date(2025, 1, 1);
    let expiry = date(2026, 1, 1);

    let discount_curve = flat_discount_with_tenor("USD-OIS", as_of, 0.03, 1.0);
    // Use PriceCurve with flat forward price of 100
    let price_curve = flat_price_curve("CL-FWD", as_of, 100.0, 1.0);
    let vol_surface = flat_vol_surface("CL-VOL", &[1.0], &[80.0, 100.0, 120.0], 0.20);

    let market = MarketContext::new()
        .insert_discount(discount_curve)
        .insert_price_curve(price_curve)
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

    let pv = option.value(&market, as_of).expect("should price");
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
    // Use PriceCurve with flat forward price of 100
    let price_curve = flat_price_curve("CL-FWD", as_of, 100.0, 1.0);
    let vol_surface = flat_vol_surface("CL-VOL", &[1.0], &[90.0, 100.0, 110.0], 0.25);

    let market = MarketContext::new()
        .insert_discount(discount_curve)
        .insert_price_curve(price_curve)
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

    let pv_eur = european.value(&market, as_of).expect("price european");
    let pv_amer = american.value(&market, as_of).expect("price american");

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
    // Price curve shows forward at 90 * exp(0.02 * 1) ≈ 91.8 (contango)
    let price_curve = flat_price_curve("CL-FWD", as_of, 91.8, 1.0);
    let vol_surface = flat_vol_surface("CL-VOL", &[1.0], &[80.0, 100.0, 120.0], 0.30);

    let market = MarketContext::new()
        .insert_discount(discount_curve)
        .insert_price_curve(price_curve)
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

    let pv_eur = european.value(&market, as_of).expect("price european");
    let pv_amer = american.value(&market, as_of).expect("price american");

    assert!(pv_amer.amount() + 1e-6 >= pv_eur.amount());
}

#[test]
fn test_post_expiry_returns_zero() {
    let expiry = date(2025, 6, 15);
    let as_of_after_expiry = date(2025, 6, 16); // 1 day after expiry

    let discount_curve = flat_discount_with_tenor("USD-OIS", as_of_after_expiry, 0.03, 1.0);
    let price_curve = flat_price_curve("CL-FWD", as_of_after_expiry, 100.0, 1.0);
    let vol_surface = flat_vol_surface("CL-VOL", &[1.0], &[80.0, 100.0, 120.0], 0.20);

    let market = MarketContext::new()
        .insert_discount(discount_curve)
        .insert_price_curve(price_curve)
        .insert_surface(vol_surface);

    // ITM call option (forward 100 > strike 90)
    let itm_call = CommodityOption::builder()
        .id(InstrumentId::new("CL-CALL-EXPIRED"))
        .commodity_type("Energy".to_string())
        .ticker("CL".to_string())
        .strike(90.0) // ITM: forward 100 > strike 90
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

    // After expiry, NPV should be 0 (option is fully settled)
    let pv = itm_call
        .value(&market, as_of_after_expiry)
        .expect("should price");
    assert_eq!(
        pv.amount(),
        0.0,
        "Post-expiry option NPV should be 0, got {}",
        pv.amount()
    );

    // ITM put option
    let itm_put = CommodityOption::builder()
        .id(InstrumentId::new("CL-PUT-EXPIRED"))
        .commodity_type("Energy".to_string())
        .ticker("CL".to_string())
        .strike(110.0) // ITM: strike 110 > forward 100
        .option_type(OptionType::Put)
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

    let pv_put = itm_put
        .value(&market, as_of_after_expiry)
        .expect("should price");
    assert_eq!(
        pv_put.amount(),
        0.0,
        "Post-expiry put NPV should be 0, got {}",
        pv_put.amount()
    );
}

#[test]
fn test_at_expiry_returns_intrinsic() {
    let expiry = date(2025, 6, 15);
    let as_of_at_expiry = expiry; // Exactly at expiry

    let discount_curve = flat_discount_with_tenor("USD-OIS", as_of_at_expiry, 0.03, 1.0);
    let price_curve = flat_price_curve("CL-FWD", as_of_at_expiry, 100.0, 1.0);
    let vol_surface = flat_vol_surface("CL-VOL", &[1.0], &[80.0, 100.0, 120.0], 0.20);

    let market = MarketContext::new()
        .insert_discount(discount_curve)
        .insert_price_curve(price_curve)
        .insert_surface(vol_surface);

    // ITM call: forward 100 > strike 90, intrinsic = 10
    let itm_call = CommodityOption::builder()
        .id(InstrumentId::new("CL-CALL-AT-EXPIRY"))
        .commodity_type("Energy".to_string())
        .ticker("CL".to_string())
        .strike(90.0)
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

    let pv = itm_call
        .value(&market, as_of_at_expiry)
        .expect("should price");
    // Forward = 100, Strike = 90, intrinsic = max(100 - 90, 0) = 10
    assert!(
        (pv.amount() - 10.0).abs() < 0.01,
        "At-expiry ITM call should have intrinsic value ~10, got {}",
        pv.amount()
    );

    // OTM call: forward 100 < strike 110, intrinsic = 0
    let otm_call = CommodityOption::builder()
        .id(InstrumentId::new("CL-CALL-OTM-AT-EXPIRY"))
        .commodity_type("Energy".to_string())
        .ticker("CL".to_string())
        .strike(110.0)
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

    let pv_otm = otm_call
        .value(&market, as_of_at_expiry)
        .expect("should price");
    assert!(
        pv_otm.amount().abs() < 0.01,
        "At-expiry OTM call should have intrinsic value ~0, got {}",
        pv_otm.amount()
    );
}
