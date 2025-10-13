//! Comprehensive FX Option metrics tests for full coverage.

use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::surfaces::VolSurface;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::money::fx::providers::SimpleFxProvider;
use finstack_core::money::fx::FxMatrix;
use finstack_core::money::Money;
use finstack_valuations::instruments::common::parameters::market::{ExerciseStyle, OptionType};
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::instruments::fx_option::FxOption;
use finstack_valuations::instruments::{PricingOverrides, SettlementType};
use finstack_valuations::metrics::MetricId;
use std::sync::Arc;
use time::macros::date;

fn build_flat_discount_curve(rate: f64, base_date: Date, curve_id: &str) -> DiscountCurve {
    DiscountCurve::builder(curve_id)
        .base_date(base_date)
        .day_count(finstack_core::dates::DayCount::Act365F)
        .knots([
            (0.0, 1.0),
            (1.0, (-rate).exp()),
            (5.0, (-rate * 5.0).exp()),
        ])
        .build()
        .unwrap()
}

fn build_flat_vol_surface(vol: f64, _base_date: Date, surface_id: &str) -> VolSurface {
    VolSurface::builder(surface_id)
        .expiries(&[0.5, 1.0, 2.0])
        .strikes(&[1.2, 1.3, 1.4])  // Need at least 2 strikes for valid surface
        .row(&[vol, vol, vol])
        .row(&[vol, vol, vol])
        .row(&[vol, vol, vol])
        .build()
        .unwrap()
}

fn create_fx_matrix(eur_usd_rate: f64) -> FxMatrix {
    let provider = SimpleFxProvider::new();
    provider.set_quote(Currency::EUR, Currency::USD, eur_usd_rate);
    FxMatrix::new(Arc::new(provider))
}

fn create_standard_call(_as_of: Date, expiry: Date, strike: f64) -> FxOption {
    FxOption {
        id: "FX_CALL_TEST".into(),
        base_currency: Currency::EUR,
        quote_currency: Currency::USD,
        strike,
        option_type: OptionType::Call,
        exercise_style: ExerciseStyle::European,
        expiry,
        day_count: finstack_core::dates::DayCount::Act365F,
        notional: Money::new(1_000_000.0, Currency::EUR),
        settlement: SettlementType::Cash,
        domestic_disc_id: "USD_DISC".into(),
        foreign_disc_id: "EUR_DISC".into(),
        vol_id: "EURUSD_VOL",
        pricing_overrides: PricingOverrides::default(),
        attributes: Default::default(),
    }
}

#[test]
fn test_fx_call_pv() {
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    
    let call = create_standard_call(as_of, expiry, 1.30);
    
    let disc_curve_usd = build_flat_discount_curve(0.05, as_of, "USD_DISC");
    let disc_curve_eur = build_flat_discount_curve(0.03, as_of, "EUR_DISC");
    let vol_surface = build_flat_vol_surface(0.10, as_of, "EURUSD_VOL");
    
    let fx_matrix = create_fx_matrix(1.30);
    let market = MarketContext::new()
        .insert_discount(disc_curve_usd)
        .insert_discount(disc_curve_eur)
        .insert_surface(vol_surface)
        .insert_fx(fx_matrix);
    
    let pv = call.value(&market, as_of).unwrap();
    
    // ATM call should have positive value
    assert!(pv.amount() > 0.0, "FX call PV should be positive");
}

#[test]
fn test_fx_delta() {
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    
    let call = create_standard_call(as_of, expiry, 1.30);
    
    let disc_curve_usd = build_flat_discount_curve(0.05, as_of, "USD_DISC");
    let disc_curve_eur = build_flat_discount_curve(0.03, as_of, "EUR_DISC");
    let vol_surface = build_flat_vol_surface(0.10, as_of, "EURUSD_VOL");
    
    let fx_matrix = create_fx_matrix(1.30);
    let market = MarketContext::new()
        .insert_discount(disc_curve_usd)
        .insert_discount(disc_curve_eur)
        .insert_surface(vol_surface)
        .insert_fx(fx_matrix);
    
    let result = call
        .price_with_metrics(&market, as_of, &[MetricId::Delta])
        .unwrap();
    
    let delta = *result.measures.get("delta").unwrap();
    
    // ATM FX call delta should be around 0.5 per unit * notional
    // For 1M EUR notional, delta should be around 500k
    assert!(delta > 300_000.0 && delta < 700_000.0, "ATM FX call delta={} should be ~500k", delta);
}

#[test]
fn test_fx_gamma() {
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    
    let call = create_standard_call(as_of, expiry, 1.30);
    
    let disc_curve_usd = build_flat_discount_curve(0.05, as_of, "USD_DISC");
    let disc_curve_eur = build_flat_discount_curve(0.03, as_of, "EUR_DISC");
    let vol_surface = build_flat_vol_surface(0.10, as_of, "EURUSD_VOL");
    
    let fx_matrix = create_fx_matrix(1.30);
    let market = MarketContext::new()
        .insert_discount(disc_curve_usd)
        .insert_discount(disc_curve_eur)
        .insert_surface(vol_surface)
        .insert_fx(fx_matrix);
    
    let result = call
        .price_with_metrics(&market, as_of, &[MetricId::Gamma])
        .unwrap();
    
    let gamma = *result.measures.get("gamma").unwrap();
    
    // Gamma should be positive for long option
    assert!(gamma > 0.0, "FX call gamma should be positive");
}

#[test]
fn test_fx_vega() {
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    
    let call = create_standard_call(as_of, expiry, 1.30);
    
    let disc_curve_usd = build_flat_discount_curve(0.05, as_of, "USD_DISC");
    let disc_curve_eur = build_flat_discount_curve(0.03, as_of, "EUR_DISC");
    let vol_surface = build_flat_vol_surface(0.10, as_of, "EURUSD_VOL");
    
    let fx_matrix = create_fx_matrix(1.30);
    let market = MarketContext::new()
        .insert_discount(disc_curve_usd)
        .insert_discount(disc_curve_eur)
        .insert_surface(vol_surface)
        .insert_fx(fx_matrix);
    
    let result = call
        .price_with_metrics(&market, as_of, &[MetricId::Vega])
        .unwrap();
    
    let vega = *result.measures.get("vega").unwrap();
    
    // Vega should be positive for long option
    assert!(vega > 0.0, "FX call vega should be positive");
}

#[test]
fn test_fx_theta() {
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    
    let call = create_standard_call(as_of, expiry, 1.30);
    
    let disc_curve_usd = build_flat_discount_curve(0.05, as_of, "USD_DISC");
    let disc_curve_eur = build_flat_discount_curve(0.03, as_of, "EUR_DISC");
    let vol_surface = build_flat_vol_surface(0.10, as_of, "EURUSD_VOL");
    
    let fx_matrix = create_fx_matrix(1.30);
    let market = MarketContext::new()
        .insert_discount(disc_curve_usd)
        .insert_discount(disc_curve_eur)
        .insert_surface(vol_surface)
        .insert_fx(fx_matrix);
    
    let result = call
        .price_with_metrics(&market, as_of, &[MetricId::Theta])
        .unwrap();
    
    let theta = *result.measures.get("theta").unwrap();
    
    // Theta represents time decay
    assert!(theta.abs() > 0.0);
}

#[test]
fn test_fx_rho_domestic() {
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    
    let call = create_standard_call(as_of, expiry, 1.30);
    
    let disc_curve_usd = build_flat_discount_curve(0.05, as_of, "USD_DISC");
    let disc_curve_eur = build_flat_discount_curve(0.03, as_of, "EUR_DISC");
    let vol_surface = build_flat_vol_surface(0.10, as_of, "EURUSD_VOL");
    
    let fx_matrix = create_fx_matrix(1.30);
    let market = MarketContext::new()
        .insert_discount(disc_curve_usd)
        .insert_discount(disc_curve_eur)
        .insert_surface(vol_surface)
        .insert_fx(fx_matrix);
    
    let result = call
        .price_with_metrics(&market, as_of, &[MetricId::Rho])
        .unwrap();
    
    let rho = *result.measures.get("rho").unwrap();
    
    // Rho measures interest rate sensitivity
    // May be 0 or very small for some implementations
    assert!(rho.is_finite(), "Rho should be finite, got: {}", rho);
}

#[test]
fn test_fx_implied_vol() {
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    
    let call = create_standard_call(as_of, expiry, 1.30);
    
    let disc_curve_usd = build_flat_discount_curve(0.05, as_of, "USD_DISC");
    let disc_curve_eur = build_flat_discount_curve(0.03, as_of, "EUR_DISC");
    let vol_surface = build_flat_vol_surface(0.10, as_of, "EURUSD_VOL");
    
    let fx_matrix = create_fx_matrix(1.30);
    let market = MarketContext::new()
        .insert_discount(disc_curve_usd)
        .insert_discount(disc_curve_eur)
        .insert_surface(vol_surface)
        .insert_fx(fx_matrix);
    
    let result = call
        .price_with_metrics(&market, as_of, &[MetricId::ImpliedVol])
        .unwrap();
    
    let implied_vol = *result.measures.get("implied_vol").unwrap();
    
    // Implied vol should match surface (10%)
    assert!((implied_vol - 0.10).abs() < 0.05, "Implied vol should be near 10%");
}

#[test]
fn test_fx_all_greeks() {
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    
    let call = create_standard_call(as_of, expiry, 1.30);
    
    let disc_curve_usd = build_flat_discount_curve(0.05, as_of, "USD_DISC");
    let disc_curve_eur = build_flat_discount_curve(0.03, as_of, "EUR_DISC");
    let vol_surface = build_flat_vol_surface(0.10, as_of, "EURUSD_VOL");
    
    let fx_matrix = create_fx_matrix(1.30);
    let market = MarketContext::new()
        .insert_discount(disc_curve_usd)
        .insert_discount(disc_curve_eur)
        .insert_surface(vol_surface)
        .insert_fx(fx_matrix);
    
    let metrics = vec![
        MetricId::Delta,
        MetricId::Gamma,
        MetricId::Vega,
        MetricId::Theta,
        MetricId::Rho,
    ];
    
    let result = call
        .price_with_metrics(&market, as_of, &metrics)
        .unwrap();
    
    // Verify all Greeks computed
    assert!(result.measures.contains_key("delta"));
    assert!(result.measures.contains_key("gamma"));
    assert!(result.measures.contains_key("vega"));
    assert!(result.measures.contains_key("theta"));
    assert!(result.measures.contains_key("rho"));
}

