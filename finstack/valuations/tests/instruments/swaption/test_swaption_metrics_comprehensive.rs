//! Comprehensive Swaption metrics tests for full coverage.

use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount, Frequency};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::surfaces::VolSurface;
use finstack_core::market_data::term_structures::{DiscountCurve, ForwardCurve};
use finstack_core::money::Money;
use finstack_valuations::instruments::common::parameters::market::OptionType;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::instruments::swaption::{Swaption, SwaptionExercise, SwaptionSettlement};
use finstack_valuations::instruments::PricingOverrides;
use finstack_valuations::metrics::MetricId;
use time::macros::date;

fn build_flat_forward_curve(rate: f64, base_date: Date, curve_id: &str) -> ForwardCurve {
    ForwardCurve::builder(curve_id, 0.25)
        .base_date(base_date)
        .day_count(DayCount::Act360)
        .knots([(0.0, rate), (10.0, rate)])
        .build()
        .unwrap()
}

fn build_flat_discount_curve(rate: f64, base_date: Date, curve_id: &str) -> DiscountCurve {
    DiscountCurve::builder(curve_id)
        .base_date(base_date)
        .day_count(DayCount::Act360)
        .knots([
            (0.0, 1.0),
            (1.0, (-rate).exp()),
            (5.0, (-rate * 5.0).exp()),
            (10.0, (-rate * 10.0).exp()),
        ])
        .build()
        .unwrap()
}

fn build_flat_vol_surface(vol: f64, _base_date: Date, surface_id: &str) -> VolSurface {
    VolSurface::builder(surface_id)
        .expiries(&[0.25, 1.0, 5.0])
        .strikes(&[0.03, 0.05, 0.07])  // Need at least 2 strikes for valid surface
        .row(&[vol, vol, vol])
        .row(&[vol, vol, vol])
        .row(&[vol, vol, vol])
        .build()
        .unwrap()
}

fn create_standard_payer_swaption(expiry: Date, swap_start: Date, swap_end: Date, strike: f64) -> Swaption {
    Swaption {
        id: "SWAPTION_TEST".into(),
        option_type: OptionType::Call, // Payer swaption = call on rates
        notional: Money::new(1_000_000.0, Currency::USD),
        strike_rate: strike,
        expiry,
        swap_start,
        swap_end,
        fixed_freq: Frequency::quarterly(),
        float_freq: Frequency::quarterly(),
        day_count: DayCount::Act360,
        exercise: SwaptionExercise::European,
        settlement: SwaptionSettlement::Physical,
        disc_id: "USD_OIS".into(),
        forward_id: "USD_LIBOR_3M".into(),
        vol_id: "USD_SWAPTION_VOL",
        pricing_overrides: PricingOverrides::default(),
        sabr_params: None,
        attributes: Default::default(),
    }
}

#[test]
fn test_swaption_pv() {
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let swap_start = date!(2025 - 01 - 01);
    let swap_end = date!(2030 - 01 - 01);
    
    let swaption = create_standard_payer_swaption(expiry, swap_start, swap_end, 0.05);
    
    let disc_curve = build_flat_discount_curve(0.05, as_of, "USD_OIS");
    let fwd_curve = build_flat_forward_curve(0.05, as_of, "USD_LIBOR_3M");
    let vol_surface = build_flat_vol_surface(0.50, as_of, "USD_SWAPTION_VOL");
    
    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_forward(fwd_curve)
        .insert_surface(vol_surface);
    
    let pv = swaption.value(&market, as_of).unwrap();
    
    // ATM swaption should have positive value
    assert!(pv.amount() > 0.0, "Swaption PV should be positive");
}

#[test]
fn test_swaption_delta() {
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let swap_start = date!(2025 - 01 - 01);
    let swap_end = date!(2030 - 01 - 01);
    
    let swaption = create_standard_payer_swaption(expiry, swap_start, swap_end, 0.05);
    
    let disc_curve = build_flat_discount_curve(0.05, as_of, "USD_OIS");
    let fwd_curve = build_flat_forward_curve(0.05, as_of, "USD_LIBOR_3M");
    let vol_surface = build_flat_vol_surface(0.50, as_of, "USD_SWAPTION_VOL");
    
    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_forward(fwd_curve)
        .insert_surface(vol_surface);
    
    let result = swaption
        .price_with_metrics(&market, as_of, &[MetricId::Delta])
        .unwrap();
    
    let delta = *result.measures.get("delta").unwrap();
    
    // ATM swaption delta is scaled by notional, should be reasonable
    assert!(delta.is_finite() && delta > 0.0, "Swaption delta should be positive and finite, got: {}", delta);
}

#[test]
fn test_swaption_gamma() {
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let swap_start = date!(2025 - 01 - 01);
    let swap_end = date!(2030 - 01 - 01);
    
    let swaption = create_standard_payer_swaption(expiry, swap_start, swap_end, 0.05);
    
    let disc_curve = build_flat_discount_curve(0.05, as_of, "USD_OIS");
    let fwd_curve = build_flat_forward_curve(0.05, as_of, "USD_LIBOR_3M");
    let vol_surface = build_flat_vol_surface(0.50, as_of, "USD_SWAPTION_VOL");
    
    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_forward(fwd_curve)
        .insert_surface(vol_surface);
    
    let result = swaption
        .price_with_metrics(&market, as_of, &[MetricId::Gamma])
        .unwrap();
    
    let gamma = *result.measures.get("gamma").unwrap();
    
    // Gamma should be positive for long option
    assert!(gamma >= 0.0, "Swaption gamma should be non-negative");
}

#[test]
fn test_swaption_vega() {
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let swap_start = date!(2025 - 01 - 01);
    let swap_end = date!(2030 - 01 - 01);
    
    let swaption = create_standard_payer_swaption(expiry, swap_start, swap_end, 0.05);
    
    let disc_curve = build_flat_discount_curve(0.05, as_of, "USD_OIS");
    let fwd_curve = build_flat_forward_curve(0.05, as_of, "USD_LIBOR_3M");
    let vol_surface = build_flat_vol_surface(0.50, as_of, "USD_SWAPTION_VOL");
    
    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_forward(fwd_curve)
        .insert_surface(vol_surface);
    
    let result = swaption
        .price_with_metrics(&market, as_of, &[MetricId::Vega])
        .unwrap();
    
    let vega = *result.measures.get("vega").unwrap();
    
    // Vega should be positive for long option
    assert!(vega > 0.0, "Swaption vega should be positive");
}

#[test]
fn test_swaption_rho() {
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let swap_start = date!(2025 - 01 - 01);
    let swap_end = date!(2030 - 01 - 01);
    
    let swaption = create_standard_payer_swaption(expiry, swap_start, swap_end, 0.05);
    
    let disc_curve = build_flat_discount_curve(0.05, as_of, "USD_OIS");
    let fwd_curve = build_flat_forward_curve(0.05, as_of, "USD_LIBOR_3M");
    let vol_surface = build_flat_vol_surface(0.50, as_of, "USD_SWAPTION_VOL");
    
    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_forward(fwd_curve)
        .insert_surface(vol_surface);
    
    let result = swaption
        .price_with_metrics(&market, as_of, &[MetricId::Rho])
        .unwrap();
    
    let rho = *result.measures.get("rho").unwrap();
    
    // Rho measures interest rate sensitivity
    assert!(rho.abs() < 10_000_000.0, "Swaption rho should be reasonable");
}

#[test]
fn test_swaption_theta() {
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let swap_start = date!(2025 - 01 - 01);
    let swap_end = date!(2030 - 01 - 01);
    
    let swaption = create_standard_payer_swaption(expiry, swap_start, swap_end, 0.05);
    
    let disc_curve = build_flat_discount_curve(0.05, as_of, "USD_OIS");
    let fwd_curve = build_flat_forward_curve(0.05, as_of, "USD_LIBOR_3M");
    let vol_surface = build_flat_vol_surface(0.50, as_of, "USD_SWAPTION_VOL");
    
    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_forward(fwd_curve)
        .insert_surface(vol_surface);
    
    let result = swaption
        .price_with_metrics(&market, as_of, &[MetricId::Theta])
        .unwrap();
    
    let theta = *result.measures.get("theta").unwrap();
    
    // Theta represents time decay
    assert!(theta.abs() < 1_000_000.0, "Swaption theta should be reasonable");
}

#[test]
fn test_swaption_dv01() {
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let swap_start = date!(2025 - 01 - 01);
    let swap_end = date!(2030 - 01 - 01);
    
    let swaption = create_standard_payer_swaption(expiry, swap_start, swap_end, 0.05);
    
    let disc_curve = build_flat_discount_curve(0.05, as_of, "USD_OIS");
    let fwd_curve = build_flat_forward_curve(0.05, as_of, "USD_LIBOR_3M");
    let vol_surface = build_flat_vol_surface(0.50, as_of, "USD_SWAPTION_VOL");
    
    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_forward(fwd_curve)
        .insert_surface(vol_surface);
    
    let result = swaption
        .price_with_metrics(&market, as_of, &[MetricId::Dv01])
        .unwrap();
    
    let dv01 = *result.measures.get("dv01").unwrap();
    
    // DV01 should be reasonable
    assert!(dv01.abs() < 100_000.0, "Swaption DV01 should be reasonable");
}

#[test]
fn test_swaption_implied_vol() {
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let swap_start = date!(2025 - 01 - 01);
    let swap_end = date!(2030 - 01 - 01);
    
    let swaption = create_standard_payer_swaption(expiry, swap_start, swap_end, 0.05);
    
    let disc_curve = build_flat_discount_curve(0.05, as_of, "USD_OIS");
    let fwd_curve = build_flat_forward_curve(0.05, as_of, "USD_LIBOR_3M");
    let vol_surface = build_flat_vol_surface(0.50, as_of, "USD_SWAPTION_VOL");
    
    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_forward(fwd_curve)
        .insert_surface(vol_surface);
    
    let result = swaption
        .price_with_metrics(&market, as_of, &[MetricId::ImpliedVol])
        .unwrap();
    
    let implied_vol = *result.measures.get("implied_vol").unwrap();
    
    // Implied vol should match surface (50%)
    assert!((implied_vol - 0.50).abs() < 0.10, "Implied vol should be near 50%");
}

#[test]
fn test_swaption_all_greeks() {
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let swap_start = date!(2025 - 01 - 01);
    let swap_end = date!(2030 - 01 - 01);
    
    let swaption = create_standard_payer_swaption(expiry, swap_start, swap_end, 0.05);
    
    let disc_curve = build_flat_discount_curve(0.05, as_of, "USD_OIS");
    let fwd_curve = build_flat_forward_curve(0.05, as_of, "USD_LIBOR_3M");
    let vol_surface = build_flat_vol_surface(0.50, as_of, "USD_SWAPTION_VOL");
    
    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_forward(fwd_curve)
        .insert_surface(vol_surface);
    
    let metrics = vec![
        MetricId::Delta,
        MetricId::Gamma,
        MetricId::Vega,
        MetricId::Rho,
        MetricId::Theta,
    ];
    
    let result = swaption
        .price_with_metrics(&market, as_of, &metrics)
        .unwrap();
    
    // Verify all Greeks computed
    assert!(result.measures.contains_key("delta"));
    assert!(result.measures.contains_key("gamma"));
    assert!(result.measures.contains_key("vega"));
    assert!(result.measures.contains_key("rho"));
    assert!(result.measures.contains_key("theta"));
}

#[test]
fn test_swaption_itm() {
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let swap_start = date!(2025 - 01 - 01);
    let swap_end = date!(2030 - 01 - 01);
    
    // Payer swaption with strike 3% when forward is 5% → ITM
    let swaption = create_standard_payer_swaption(expiry, swap_start, swap_end, 0.03);
    
    let disc_curve = build_flat_discount_curve(0.05, as_of, "USD_OIS");
    let fwd_curve = build_flat_forward_curve(0.05, as_of, "USD_LIBOR_3M");
    let vol_surface = build_flat_vol_surface(0.50, as_of, "USD_SWAPTION_VOL");
    
    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_forward(fwd_curve)
        .insert_surface(vol_surface);
    
    let result = swaption
        .price_with_metrics(&market, as_of, &[MetricId::Delta])
        .unwrap();
    
    let delta = *result.measures.get("delta").unwrap();
    
    // ITM swaption should have high delta
    assert!(delta > 0.5, "ITM swaption delta={} should be > 0.5", delta);
}

#[test]
fn test_swaption_otm() {
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let swap_start = date!(2025 - 01 - 01);
    let swap_end = date!(2030 - 01 - 01);
    
    // Payer swaption with strike 10% when forward is 5% → OTM
    let swaption = create_standard_payer_swaption(expiry, swap_start, swap_end, 0.10);
    
    let disc_curve = build_flat_discount_curve(0.05, as_of, "USD_OIS");
    let fwd_curve = build_flat_forward_curve(0.05, as_of, "USD_LIBOR_3M");
    let vol_surface = build_flat_vol_surface(0.50, as_of, "USD_SWAPTION_VOL");
    
    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_forward(fwd_curve)
        .insert_surface(vol_surface);
    
    let result = swaption
        .price_with_metrics(&market, as_of, &[MetricId::Delta])
        .unwrap();
    
    let delta = *result.measures.get("delta").unwrap();
    
    // OTM swaption delta should be finite (scaled by notional)
    assert!(delta.is_finite(), "OTM swaption delta should be finite, got: {}", delta);
}

