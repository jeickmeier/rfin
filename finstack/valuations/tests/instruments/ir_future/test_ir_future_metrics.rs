//! Comprehensive IR Future metrics tests for full coverage.

use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::{DiscountCurve, ForwardCurve};
use finstack_core::money::Money;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::instruments::ir_future::{InterestRateFuture, Position, FutureContractSpecs};
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
        ])
        .build()
        .unwrap()
}

fn create_standard_future(start: Date, end: Date) -> InterestRateFuture {
    InterestRateFuture {
        id: "IRF_TEST".into(),
        notional: Money::new(1_000_000.0, Currency::USD),
        expiry_date: start,
        fixing_date: start,
        period_start: start,
        period_end: end,
        quoted_price: 97.50, // Price of future
        day_count: DayCount::Act360,
        position: Position::Long,
        contract_specs: FutureContractSpecs::default(),
        disc_id: "USD_OIS".into(),
        forward_id: "USD_LIBOR_3M".into(),
        attributes: Default::default(),
    }
}

#[test]
fn test_ir_future_pv() {
    let as_of = date!(2024 - 01 - 01);
    let start = date!(2024 - 07 - 01);
    let end = date!(2024 - 10 - 01);
    
    let future = create_standard_future(start, end);
    
    let disc_curve = build_flat_discount_curve(0.05, as_of, "USD_OIS");
    let fwd_curve = build_flat_forward_curve(0.05, as_of, "USD_LIBOR_3M");
    
    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_forward(fwd_curve);
    
    let pv = future.value(&market, as_of).unwrap();
    
    // IR future should have a PV
    assert!(pv.amount().abs() < 100_000.0, "IR future PV should be reasonable");
}

#[test]
fn test_ir_future_dv01() {
    let as_of = date!(2024 - 01 - 01);
    let start = date!(2024 - 07 - 01);
    let end = date!(2024 - 10 - 01);
    
    let future = create_standard_future(start, end);
    
    let disc_curve = build_flat_discount_curve(0.05, as_of, "USD_OIS");
    let fwd_curve = build_flat_forward_curve(0.05, as_of, "USD_LIBOR_3M");
    
    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_forward(fwd_curve);
    
    let result = future
        .price_with_metrics(&market, as_of, &[MetricId::Dv01])
        .unwrap();
    
    let dv01 = *result.measures.get("dv01").unwrap();
    
    // DV01 should be reasonable for 3-month future on $1MM
    assert!(dv01.abs() > 10.0 && dv01.abs() < 500.0);
}

#[test]
fn test_ir_future_theta() {
    let as_of = date!(2024 - 01 - 01);
    let start = date!(2024 - 07 - 01);
    let end = date!(2024 - 10 - 01);
    
    let future = create_standard_future(start, end);
    
    let disc_curve = build_flat_discount_curve(0.05, as_of, "USD_OIS");
    let fwd_curve = build_flat_forward_curve(0.05, as_of, "USD_LIBOR_3M");
    
    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_forward(fwd_curve);
    
    let result = future
        .price_with_metrics(&market, as_of, &[MetricId::Theta])
        .unwrap();
    
    let theta = *result.measures.get("theta").unwrap();
    
    // Theta measures time decay
    assert!(theta.abs() < 10_000.0);
}

#[test]
fn test_ir_future_all_metrics() {
    let as_of = date!(2024 - 01 - 01);
    let start = date!(2024 - 07 - 01);
    let end = date!(2024 - 10 - 01);
    
    let future = create_standard_future(start, end);
    
    let disc_curve = build_flat_discount_curve(0.05, as_of, "USD_OIS");
    let fwd_curve = build_flat_forward_curve(0.05, as_of, "USD_LIBOR_3M");
    
    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_forward(fwd_curve);
    
    let metrics = vec![MetricId::Dv01, MetricId::Theta];
    
    let result = future
        .price_with_metrics(&market, as_of, &metrics)
        .unwrap();
    
    // Verify all metrics computed
    assert!(result.measures.contains_key("dv01"));
    assert!(result.measures.contains_key("theta"));
}

#[test]
fn test_ir_future_near_expiry() {
    let as_of = date!(2024 - 01 - 01);
    let start = date!(2024 - 01 - 15);
    let end = date!(2024 - 02 - 15);
    
    let future = create_standard_future(start, end);
    
    let disc_curve = build_flat_discount_curve(0.05, as_of, "USD_OIS");
    let fwd_curve = build_flat_forward_curve(0.05, as_of, "USD_LIBOR_3M");
    
    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_forward(fwd_curve);
    
    let result = future
        .price_with_metrics(&market, as_of, &[MetricId::Dv01])
        .unwrap();
    
    let dv01 = *result.measures.get("dv01").unwrap();
    
    // Short-dated future has lower DV01
    assert!(dv01.abs() < 100.0);
}

