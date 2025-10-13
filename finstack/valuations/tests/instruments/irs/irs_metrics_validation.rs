//! Interest Rate Swap metrics validation tests against known market benchmarks.
//!
//! These tests validate our IRS metric calculations against:
//! - Standard swap pricing formulas
//! - Market conventions (ISDA, ICMA)
//! - Textbook examples
//!
//! References:
//! - Hull, "Options, Futures, and Other Derivatives"
//! - ISDA documentation
//! - Market practice for USD swaps

use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Frequency, StubKind};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::{DiscountCurve, ForwardCurve};
use finstack_core::money::Money;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::instruments::irs::{FixedLegSpec, FloatLegSpec, InterestRateSwap, PayReceive};
use finstack_valuations::metrics::MetricId;
use time::macros::date;

/// Build a flat forward curve for testing
fn build_flat_forward_curve(rate: f64, base_date: Date, curve_id: &str) -> ForwardCurve {
    ForwardCurve::builder(curve_id, 0.25) // 3M tenor
        .base_date(base_date)
        .day_count(DayCount::Act360)
        .knots([(0.0, rate), (10.0, rate)])
        .build()
        .unwrap()
}

/// Build a flat discount curve for testing
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

#[test]
fn test_irs_par_rate_market_standard() {
    // Market standard: For a new (at-inception) swap, par rate makes NPV = 0
    // 5-year USD swap, quarterly payments
    
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);
    
    // Build curves: flat 5% rate environment
    let disc_curve = build_flat_discount_curve(0.05, as_of, "USD_OIS");
    let fwd_curve = build_flat_forward_curve(0.05, as_of, "USD_LIBOR_3M");
    
    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_forward(fwd_curve);
    
    // Create swap receiving fixed at par rate
    let swap = InterestRateSwap {
        id: "SWAP_PAR_TEST".into(),
        notional: Money::new(1_000_000.0, Currency::USD),
        side: PayReceive::ReceiveFixed,
        fixed: FixedLegSpec {
            disc_id: "USD_OIS".into(),
            rate: 0.05, // At par
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
    };
    
    // At-the-money swap should have NPV ≈ 0
    let npv = swap.value(&market, as_of).unwrap();
    
    assert!(
        npv.amount().abs() < 100.0, // Within $100 on $1MM notional
        "At-the-money swap NPV={:.2} should be near zero",
        npv.amount()
    );
}

#[test]
fn test_irs_annuity_calculation() {
    // Annuity = Sum of discounted year fractions
    // For 5-year swap at 5% flat curve, quarterly payments
    // Expected: ~4.28 years (approximate)
    
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);
    
    let disc_curve = build_flat_discount_curve(0.05, as_of, "USD_OIS");
    let fwd_curve = build_flat_forward_curve(0.05, as_of, "USD_LIBOR_3M");
    
    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_forward(fwd_curve);
    
    let swap = InterestRateSwap {
        id: "SWAP_ANNUITY_TEST".into(),
        notional: Money::new(1_000_000.0, Currency::USD),
        side: PayReceive::ReceiveFixed,
        fixed: FixedLegSpec {
            disc_id: "USD_OIS".into(),
            rate: 0.05,
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
    };
    
    let result = swap
        .price_with_metrics(&market, as_of, &[MetricId::Annuity])
        .unwrap();
    
    let annuity = *result.measures.get("annuity").unwrap();
    
    // Expected: ~4.28 for 5-year swap at 5%
    // Annuity is always less than maturity due to discounting
    assert!(
        annuity > 4.0 && annuity < 4.5,
        "Annuity={:.3} outside expected range 4.0-4.5 years",
        annuity
    );
    
    // Annuity should be less than time to maturity
    assert!(
        annuity < 5.0,
        "Annuity={:.3} should be less than maturity 5.0 years",
        annuity
    );
}

#[test]
fn test_irs_dv01_market_standard() {
    // DV01 = Annuity × Notional × 1bp
    // Market standard formula for IRS
    
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);
    
    let disc_curve = build_flat_discount_curve(0.05, as_of, "USD_OIS");
    let fwd_curve = build_flat_forward_curve(0.05, as_of, "USD_LIBOR_3M");
    
    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_forward(fwd_curve);
    
    let notional = 1_000_000.0;
    
    let swap = InterestRateSwap {
        id: "SWAP_DV01_TEST".into(),
        notional: Money::new(notional, Currency::USD),
        side: PayReceive::ReceiveFixed,
        fixed: FixedLegSpec {
            disc_id: "USD_OIS".into(),
            rate: 0.05,
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
    };
    
    let result = swap
        .price_with_metrics(&market, as_of, &[MetricId::Annuity, MetricId::Dv01])
        .unwrap();
    
    let annuity = *result.measures.get("annuity").unwrap();
    let dv01 = *result.measures.get("dv01").unwrap();
    
    // DV01 should equal Annuity × Notional × 0.0001
    let expected_dv01 = annuity * notional * 0.0001;
    
    assert!(
        (dv01.abs() - expected_dv01.abs()).abs() < 1.0,
        "DV01={:.2} vs expected {:.2} (Annuity={:.4} × Notional={} × 0.0001)",
        dv01,
        expected_dv01,
        annuity,
        notional
    );
    
    // For $1MM notional, 5-year swap, DV01 magnitude should be around $430
    assert!(
        dv01.abs() > 400.0 && dv01.abs() < 450.0,
        "DV01={:.2} outside typical range $400-$450 for $1MM 5Y swap",
        dv01
    );
}

#[test]
fn test_irs_receive_vs_pay_fixed() {
    // Receive fixed and pay fixed should have opposite NPVs
    
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);
    
    let disc_curve = build_flat_discount_curve(0.05, as_of, "USD_OIS");
    let fwd_curve = build_flat_forward_curve(0.06, as_of, "USD_LIBOR_3M"); // Forward > fixed
    
    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_forward(fwd_curve);
    
    let fixed_leg = FixedLegSpec {
        disc_id: "USD_OIS".into(),
        rate: 0.05, // Fixed at 5%
        freq: Frequency::quarterly(),
        dc: DayCount::Act360,
        bdc: BusinessDayConvention::ModifiedFollowing,
        calendar_id: None,
        stub: StubKind::None,
        par_method: None,
        compounding_simple: true,
        start: as_of,
        end,
    };
    
    let float_leg = FloatLegSpec {
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
    };
    
    // Receive fixed swap
    let swap_receive = InterestRateSwap {
        id: "SWAP_RECEIVE".into(),
        notional: Money::new(1_000_000.0, Currency::USD),
        side: PayReceive::ReceiveFixed,
        fixed: fixed_leg.clone(),
        float: float_leg.clone(),
        attributes: Default::default(),
    };
    
    // Pay fixed swap
    let swap_pay = InterestRateSwap {
        id: "SWAP_PAY".into(),
        notional: Money::new(1_000_000.0, Currency::USD),
        side: PayReceive::PayFixed,
        fixed: fixed_leg,
        float: float_leg,
        attributes: Default::default(),
    };
    
    let npv_receive = swap_receive.value(&market, as_of).unwrap();
    let npv_pay = swap_pay.value(&market, as_of).unwrap();
    
    // NPVs should be opposite signs
    assert!(
        npv_receive.amount() * npv_pay.amount() < 0.0,
        "Receive fixed and pay fixed should have opposite NPVs: receive={:.2}, pay={:.2}",
        npv_receive.amount(),
        npv_pay.amount()
    );
    
    // With forward rate (6%) > fixed rate (5%):
    // Receive fixed (pay floating) should be negative
    // Pay fixed (receive floating) should be positive
    assert!(
        npv_receive.amount() < 0.0,
        "Receive fixed below market should be negative: {:.2}",
        npv_receive.amount()
    );
    assert!(
        npv_pay.amount() > 0.0,
        "Pay fixed below market should be positive: {:.2}",
        npv_pay.amount()
    );
}

#[test]
fn test_irs_rate_sensitivity() {
    // As rates increase, receive fixed position loses value
    
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);
    
    let fixed_leg = FixedLegSpec {
        disc_id: "USD_OIS".into(),
        rate: 0.05,
        freq: Frequency::quarterly(),
        dc: DayCount::Act360,
        bdc: BusinessDayConvention::ModifiedFollowing,
        calendar_id: None,
        stub: StubKind::None,
        par_method: None,
        compounding_simple: true,
        start: as_of,
        end,
    };
    
    let float_leg = FloatLegSpec {
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
    };
    
    let swap = InterestRateSwap {
        id: "SWAP_RATE_SENS".into(),
        notional: Money::new(1_000_000.0, Currency::USD),
        side: PayReceive::ReceiveFixed,
        fixed: fixed_leg,
        float: float_leg,
        attributes: Default::default(),
    };
    
    let mut npvs = Vec::new();
    
    // Test at different rate levels
    for rate in [0.03, 0.04, 0.05, 0.06, 0.07] {
        let disc_curve = build_flat_discount_curve(rate, as_of, "USD_OIS");
        let fwd_curve = build_flat_forward_curve(rate, as_of, "USD_LIBOR_3M");
        
        let market = MarketContext::new()
            .insert_discount(disc_curve)
            .insert_forward(fwd_curve);
        
        let npv = swap.value(&market, as_of).unwrap();
        npvs.push((rate, npv.amount()));
    }
    
    // Verify inverse relationship for receive fixed:
    // Higher rates → Lower NPV (losing value)
    for i in 1..npvs.len() {
        assert!(
            npvs[i].1 < npvs[i - 1].1,
            "Receive fixed swap value should decrease as rates rise: \
             rate {:.2}% NPV={:.2} >= rate {:.2}% NPV={:.2}",
            npvs[i].0 * 100.0,
            npvs[i].1,
            npvs[i - 1].0 * 100.0,
            npvs[i - 1].1
        );
    }
    
    // At par rate (5%), NPV should be near zero
    let par_npv = npvs[2].1; // 0.05 rate
    assert!(
        par_npv.abs() < 1000.0,
        "At par rate, NPV should be near zero: {:.2}",
        par_npv
    );
}

#[test]
fn test_irs_leg_pvs_consistency() {
    // For receive fixed swap: NPV = PV(fixed leg) - PV(floating leg)
    
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);
    
    let disc_curve = build_flat_discount_curve(0.05, as_of, "USD_OIS");
    let fwd_curve = build_flat_forward_curve(0.06, as_of, "USD_LIBOR_3M");
    
    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_forward(fwd_curve);
    
    let swap = InterestRateSwap {
        id: "SWAP_LEG_PVS".into(),
        notional: Money::new(1_000_000.0, Currency::USD),
        side: PayReceive::ReceiveFixed,
        fixed: FixedLegSpec {
            disc_id: "USD_OIS".into(),
            rate: 0.05,
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
    };
    
    let result = swap
        .price_with_metrics(&market, as_of, &[MetricId::PvFixed, MetricId::PvFloat])
        .unwrap();
    
    let pv_fixed = *result.measures.get("pv_fixed").unwrap();
    let pv_float = *result.measures.get("pv_float").unwrap();
    let base_npv = result.value.amount();
    
    // NPV should equal PV(fixed) - PV(float) for receive fixed
    let calculated_npv = pv_fixed - pv_float;
    
    assert!(
        (calculated_npv - base_npv).abs() < 10.0,
        "NPV from legs ({:.2}) should match total NPV ({:.2})",
        calculated_npv,
        base_npv
    );
}
