//! Comprehensive IRS metrics tests for full coverage.

use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Frequency, StubKind};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::{DiscountCurve, ForwardCurve};
use finstack_core::money::Money;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::instruments::irs::{FixedLegSpec, FloatLegSpec, InterestRateSwap, PayReceive};
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

fn create_standard_swap(as_of: Date, end: Date, fixed_rate: f64, side: PayReceive) -> InterestRateSwap {
    InterestRateSwap {
        id: "IRS_TEST".into(),
        notional: Money::new(1_000_000.0, Currency::USD),
        side,
        fixed: FixedLegSpec {
            disc_id: "USD_OIS".into(),
            rate: fixed_rate,
            freq: Frequency::quarterly(),
            dc: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: None,
            stub: StubKind::None,
            par_method: None,
            compounding_simple: true,
            start: as_of,
            end,
        },
        float: FloatLegSpec {
            disc_id: "USD_OIS".into(),
            fwd_id: "USD_LIBOR_3M".into(),
            spread_bp: 0.0,
            freq: Frequency::quarterly(),
            dc: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: None,
            stub: StubKind::None,
            reset_lag_days: 2,
            start: as_of,
            end,
        },
        attributes: Default::default(),
    }
}

#[test]
fn test_irs_annuity() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);
    
    let swap = create_standard_swap(as_of, end, 0.05, PayReceive::ReceiveFixed);
    
    let disc_curve = build_flat_discount_curve(0.05, as_of, "USD_OIS");
    let fwd_curve = build_flat_forward_curve(0.05, as_of, "USD_LIBOR_3M");
    
    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_forward(fwd_curve);
    
    let result = swap
        .price_with_metrics(&market, as_of, &[MetricId::Annuity])
        .unwrap();
    
    let annuity = *result.measures.get("annuity").unwrap();
    
    // Annuity should be positive and less than maturity
    assert!(annuity > 0.0 && annuity < 5.0);
}

#[test]
fn test_irs_dv01() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);
    
    let swap = create_standard_swap(as_of, end, 0.05, PayReceive::ReceiveFixed);
    
    let disc_curve = build_flat_discount_curve(0.05, as_of, "USD_OIS");
    let fwd_curve = build_flat_forward_curve(0.05, as_of, "USD_LIBOR_3M");
    
    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_forward(fwd_curve);
    
    let result = swap
        .price_with_metrics(&market, as_of, &[MetricId::Dv01])
        .unwrap();
    
    let dv01 = *result.measures.get("dv01").unwrap();
    
    // DV01 magnitude should be reasonable for $1MM notional
    assert!(dv01.abs() > 100.0 && dv01.abs() < 1000.0);
}

#[test]
fn test_irs_par_rate() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);
    
    let swap = create_standard_swap(as_of, end, 0.05, PayReceive::ReceiveFixed);
    
    let disc_curve = build_flat_discount_curve(0.05, as_of, "USD_OIS");
    let fwd_curve = build_flat_forward_curve(0.05, as_of, "USD_LIBOR_3M");
    
    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_forward(fwd_curve);
    
    let result = swap
        .price_with_metrics(&market, as_of, &[MetricId::ParRate])
        .unwrap();
    
    let par_rate = *result.measures.get("par_rate").unwrap();
    
    // In flat 5% environment, par rate should be near 5%
    assert!((par_rate - 0.05).abs() < 0.01);
}

#[test]
fn test_irs_pv_fixed() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);
    
    let swap = create_standard_swap(as_of, end, 0.05, PayReceive::ReceiveFixed);
    
    let disc_curve = build_flat_discount_curve(0.05, as_of, "USD_OIS");
    let fwd_curve = build_flat_forward_curve(0.05, as_of, "USD_LIBOR_3M");
    
    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_forward(fwd_curve);
    
    let result = swap
        .price_with_metrics(&market, as_of, &[MetricId::PvFixed])
        .unwrap();
    
    let pv_fixed = *result.measures.get("pv_fixed").unwrap();
    
    // PV of fixed leg should be positive
    assert!(pv_fixed > 0.0);
    assert!(pv_fixed < 5_000_000.0); // Reasonable for $1MM notional
}

#[test]
fn test_irs_pv_float() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);
    
    let swap = create_standard_swap(as_of, end, 0.05, PayReceive::ReceiveFixed);
    
    let disc_curve = build_flat_discount_curve(0.05, as_of, "USD_OIS");
    let fwd_curve = build_flat_forward_curve(0.05, as_of, "USD_LIBOR_3M");
    
    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_forward(fwd_curve);
    
    let result = swap
        .price_with_metrics(&market, as_of, &[MetricId::PvFloat])
        .unwrap();
    
    let pv_float = *result.measures.get("pv_float").unwrap();
    
    // PV of floating leg should be positive
    assert!(pv_float > 0.0);
    assert!(pv_float < 5_000_000.0);
}

#[test]
fn test_irs_theta() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);
    
    let swap = create_standard_swap(as_of, end, 0.05, PayReceive::ReceiveFixed);
    
    let disc_curve = build_flat_discount_curve(0.05, as_of, "USD_OIS");
    let fwd_curve = build_flat_forward_curve(0.05, as_of, "USD_LIBOR_3M");
    
    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_forward(fwd_curve);
    
    let result = swap
        .price_with_metrics(&market, as_of, &[MetricId::Theta])
        .unwrap();
    
    let theta = *result.measures.get("theta").unwrap();
    
    // Theta measures time decay
    assert!(theta.abs() < 100_000.0);
}

#[test]
fn test_irs_receive_vs_pay() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);
    
    let swap_receive = create_standard_swap(as_of, end, 0.05, PayReceive::ReceiveFixed);
    let swap_pay = create_standard_swap(as_of, end, 0.05, PayReceive::PayFixed);
    
    let disc_curve = build_flat_discount_curve(0.05, as_of, "USD_OIS");
    let fwd_curve = build_flat_forward_curve(0.06, as_of, "USD_LIBOR_3M");
    
    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_forward(fwd_curve);
    
    let npv_receive = swap_receive.value(&market, as_of).unwrap();
    let npv_pay = swap_pay.value(&market, as_of).unwrap();
    
    // Opposite sides should have opposite NPVs
    assert!(npv_receive.amount() * npv_pay.amount() < 0.0);
}

#[test]
fn test_irs_off_market_swap() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);
    
    // Fixed rate 3% in 5% market
    let swap = create_standard_swap(as_of, end, 0.03, PayReceive::ReceiveFixed);
    
    let disc_curve = build_flat_discount_curve(0.05, as_of, "USD_OIS");
    let fwd_curve = build_flat_forward_curve(0.05, as_of, "USD_LIBOR_3M");
    
    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_forward(fwd_curve);
    
    let npv = swap.value(&market, as_of).unwrap();
    
    // Receive fixed below market → negative NPV
    assert!(npv.amount() < 0.0);
}

#[test]
fn test_irs_all_metrics() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);
    
    let swap = create_standard_swap(as_of, end, 0.05, PayReceive::ReceiveFixed);
    
    let disc_curve = build_flat_discount_curve(0.05, as_of, "USD_OIS");
    let fwd_curve = build_flat_forward_curve(0.05, as_of, "USD_LIBOR_3M");
    
    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_forward(fwd_curve);
    
    let metrics = vec![
        MetricId::Annuity,
        MetricId::Dv01,
        MetricId::ParRate,
        MetricId::PvFixed,
        MetricId::PvFloat,
        MetricId::Theta,
    ];
    
    let result = swap
        .price_with_metrics(&market, as_of, &metrics)
        .unwrap();
    
    // Verify all metrics computed
    assert!(result.measures.contains_key("annuity"));
    assert!(result.measures.contains_key("dv01"));
    assert!(result.measures.contains_key("par_rate"));
    assert!(result.measures.contains_key("pv_fixed"));
    assert!(result.measures.contains_key("pv_float"));
    assert!(result.measures.contains_key("theta"));
}

#[test]
fn test_irs_short_maturity() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2025 - 01 - 01); // 1 year
    
    let swap = create_standard_swap(as_of, end, 0.05, PayReceive::ReceiveFixed);
    
    let disc_curve = build_flat_discount_curve(0.05, as_of, "USD_OIS");
    let fwd_curve = build_flat_forward_curve(0.05, as_of, "USD_LIBOR_3M");
    
    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_forward(fwd_curve);
    
    let result = swap
        .price_with_metrics(&market, as_of, &[MetricId::Annuity, MetricId::Dv01])
        .unwrap();
    
    let annuity = *result.measures.get("annuity").unwrap();
    let dv01 = *result.measures.get("dv01").unwrap();
    
    // Short swap has lower annuity and DV01
    assert!(annuity < 1.0);
    assert!(dv01.abs() < 150.0);
}

#[test]
fn test_irs_with_spread() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);
    
    let mut swap = create_standard_swap(as_of, end, 0.05, PayReceive::ReceiveFixed);
    swap.float.spread_bp = 50.0; // 50bp spread
    
    let disc_curve = build_flat_discount_curve(0.05, as_of, "USD_OIS");
    let fwd_curve = build_flat_forward_curve(0.05, as_of, "USD_LIBOR_3M");
    
    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_forward(fwd_curve);
    
    let npv = swap.value(&market, as_of).unwrap();
    
    // Spread on floating leg affects NPV
    assert!(npv.amount() < 0.0); // Paying higher floating rate
}

