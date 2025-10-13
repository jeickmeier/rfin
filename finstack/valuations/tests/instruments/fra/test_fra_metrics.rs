//! Comprehensive FRA metrics tests for full coverage.

use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::{DiscountCurve, ForwardCurve};
use finstack_core::money::Money;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::instruments::fra::ForwardRateAgreement;
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

fn create_standard_fra(start: Date, end: Date, fra_rate: f64) -> ForwardRateAgreement {
    ForwardRateAgreement {
        id: "FRA_TEST".into(),
        notional: Money::new(1_000_000.0, Currency::USD),
        fixing_date: start,
        start_date: start,
        end_date: end,
        fixed_rate: fra_rate,
        day_count: DayCount::Act360,
        reset_lag: 2,
        disc_id: "USD_OIS".into(),
        forward_id: "USD_LIBOR_3M".into(),
        pay_fixed: true,
        attributes: Default::default(),
    }
}

#[test]
fn test_fra_pv() {
    let as_of = date!(2024 - 01 - 01);
    let start = date!(2024 - 07 - 01);
    let end = date!(2024 - 10 - 01);
    
    let fra = create_standard_fra(start, end, 0.05);
    
    let disc_curve = build_flat_discount_curve(0.05, as_of, "USD_OIS");
    let fwd_curve = build_flat_forward_curve(0.05, as_of, "USD_LIBOR_3M");
    
    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_forward(fwd_curve);
    
    let pv = fra.value(&market, as_of).unwrap();
    
    // At-market FRA should have near-zero PV
    assert!(pv.amount().abs() < 1000.0, "At-market FRA PV should be near zero");
}

// Removed test_fra_forward_rate - ForwardRate metric no longer exists

#[test]
fn test_fra_dv01() {
    let as_of = date!(2024 - 01 - 01);
    let start = date!(2024 - 07 - 01);
    let end = date!(2024 - 10 - 01);
    
    let fra = create_standard_fra(start, end, 0.05);
    
    let disc_curve = build_flat_discount_curve(0.05, as_of, "USD_OIS");
    let fwd_curve = build_flat_forward_curve(0.05, as_of, "USD_LIBOR_3M");
    
    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_forward(fwd_curve);
    
    let result = fra
        .price_with_metrics(&market, as_of, &[MetricId::Dv01])
        .unwrap();
    
    let dv01 = *result.measures.get("dv01").unwrap();
    
    // DV01 should be reasonable for 3-month FRA on $1MM
    assert!(dv01.abs() > 10.0 && dv01.abs() < 500.0);
}

#[test]
fn test_fra_theta() {
    let as_of = date!(2024 - 01 - 01);
    let start = date!(2024 - 07 - 01);
    let end = date!(2024 - 10 - 01);
    
    let fra = create_standard_fra(start, end, 0.05);
    
    let disc_curve = build_flat_discount_curve(0.05, as_of, "USD_OIS");
    let fwd_curve = build_flat_forward_curve(0.05, as_of, "USD_LIBOR_3M");
    
    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_forward(fwd_curve);
    
    let result = fra
        .price_with_metrics(&market, as_of, &[MetricId::Theta])
        .unwrap();
    
    let theta = *result.measures.get("theta").unwrap();
    
    // Theta measures time decay
    assert!(theta.abs() < 10_000.0);
}

#[test]
fn test_fra_off_market() {
    let as_of = date!(2024 - 01 - 01);
    let start = date!(2024 - 07 - 01);
    let end = date!(2024 - 10 - 01);
    
    // FRA at 4% when market is 6%
    let fra = create_standard_fra(start, end, 0.04);
    
    let disc_curve = build_flat_discount_curve(0.05, as_of, "USD_OIS");
    let fwd_curve = build_flat_forward_curve(0.06, as_of, "USD_LIBOR_3M");
    
    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_forward(fwd_curve);
    
    let pv = fra.value(&market, as_of).unwrap();
    
    // Receiving fixed 4% when market is 6% → negative PV
    assert!(pv.amount() < 0.0, "Below-market FRA should have negative PV");
}

#[test]
fn test_fra_all_metrics() {
    let as_of = date!(2024 - 01 - 01);
    let start = date!(2024 - 07 - 01);
    let end = date!(2024 - 10 - 01);
    
    let fra = create_standard_fra(start, end, 0.05);
    
    let disc_curve = build_flat_discount_curve(0.05, as_of, "USD_OIS");
    let fwd_curve = build_flat_forward_curve(0.05, as_of, "USD_LIBOR_3M");
    
    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_forward(fwd_curve);
    
    let metrics = vec![
        MetricId::Dv01,
        MetricId::Theta,
    ];
    
    let result = fra
        .price_with_metrics(&market, as_of, &metrics)
        .unwrap();
    
    // Verify all metrics computed
    assert!(result.measures.contains_key("dv01"));
    assert!(result.measures.contains_key("theta"));
}

#[test]
fn test_fra_short_period() {
    let as_of = date!(2024 - 01 - 01);
    let start = date!(2024 - 02 - 01);
    let end = date!(2024 - 03 - 01); // 1 month
    
    let fra = create_standard_fra(start, end, 0.05);
    
    let disc_curve = build_flat_discount_curve(0.05, as_of, "USD_OIS");
    let fwd_curve = build_flat_forward_curve(0.05, as_of, "USD_LIBOR_3M");
    
    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_forward(fwd_curve);
    
    let result = fra
        .price_with_metrics(&market, as_of, &[MetricId::Dv01])
        .unwrap();
    
    let dv01 = *result.measures.get("dv01").unwrap();
    
    // Short FRA has lower DV01
    assert!(dv01.abs() < 100.0);
}

#[test]
fn test_fra_long_period() {
    let as_of = date!(2024 - 01 - 01);
    let start = date!(2024 - 07 - 01);
    let end = date!(2025 - 01 - 01); // 6 months
    
    let fra = create_standard_fra(start, end, 0.05);
    
    let disc_curve = build_flat_discount_curve(0.05, as_of, "USD_OIS");
    let fwd_curve = build_flat_forward_curve(0.05, as_of, "USD_LIBOR_3M");
    
    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_forward(fwd_curve);
    
    let result = fra
        .price_with_metrics(&market, as_of, &[MetricId::Dv01])
        .unwrap();
    
    let dv01 = *result.measures.get("dv01").unwrap();
    
    // Longer FRA should have reasonable DV01
    assert!(dv01.is_finite(), "DV01 should be finite, got: {}", dv01);
}

