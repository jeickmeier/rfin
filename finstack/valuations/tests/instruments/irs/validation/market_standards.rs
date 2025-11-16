//! Market standard validation tests.
//!
//! These tests validate our IRS metric calculations against:
//! - Standard swap pricing formulas
//! - Market conventions (ISDA, ICMA)
//! - Textbook examples (Hull)
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
use finstack_valuations::instruments::irs::{
    FixedLegSpec, FloatLegSpec, InterestRateSwap, PayReceive,
};
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
    let mut builder = DiscountCurve::builder(curve_id)
        .base_date(base_date)
        .day_count(DayCount::Act360)
        .knots([
            (0.0, 1.0),
            (1.0, (-rate).exp()),
            (5.0, (-rate * 5.0).exp()),
            (10.0, (-rate * 10.0).exp()),
            (30.0, (-rate * 30.0).exp()),
        ]);

    // For zero or negative rates, DFs may be flat or increasing
    if rate.abs() < 1e-10 || rate < 0.0 {
        builder = builder.allow_non_monotonic();
    }

    builder.build().unwrap()
}

#[test]
fn test_irs_par_rate_market_standard() {
    // Market standard: For a new (at-inception) swap, par rate makes NPV = 0
    // 5-year USD swap (ISDA conventions via InterestRateSwap::new)

    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);

    let disc_curve = build_flat_discount_curve(0.05, as_of, "USD-OIS");
    let fwd_curve = build_flat_forward_curve(0.05, as_of, "USD-SOFR-3M");

    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_forward(fwd_curve);

    let swap = InterestRateSwap::new(
        "SWAP_PAR_TEST".into(),
        Money::new(1_000_000.0, Currency::USD),
        0.05,
        as_of,
        end,
        PayReceive::ReceiveFixed,
    );
    // Compute par rate under current curves
    let par = swap
        .price_with_metrics(&market, as_of, &[MetricId::ParRate])
        .unwrap()
        .measures["par_rate"];
    // Rebuild swap at par and assert PV ~ 0
    let par_swap = InterestRateSwap::new(
        "SWAP_PAR_PAR".into(),
        Money::new(1_000_000.0, Currency::USD),
        par,
        as_of,
        end,
        PayReceive::ReceiveFixed,
    );
    let npv = par_swap.value(&market, as_of).unwrap();
    assert!(
        npv.amount().abs() < 2000.0,
        "Par swap NPV={:.2} near zero",
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

    let disc_curve = build_flat_discount_curve(0.05, as_of, "USD-OIS");
    let fwd_curve = build_flat_forward_curve(0.05, as_of, "USD-SOFR-3M");

    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_forward(fwd_curve);

    let swap = InterestRateSwap::new(
        "SWAP_ANNUITY_TEST".into(),
        Money::new(1_000_000.0, Currency::USD),
        0.05,
        as_of,
        end,
        PayReceive::ReceiveFixed,
    );

    let result = swap
        .price_with_metrics(&market, as_of, &[MetricId::Annuity])
        .unwrap();

    let annuity = *result.measures.get("annuity").unwrap();

    // Expected: ~4.28 for 5-year swap at 5%
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

    let disc_curve = build_flat_discount_curve(0.05, as_of, "USD-OIS");
    let fwd_curve = build_flat_forward_curve(0.05, as_of, "USD-SOFR-3M");

    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_forward(fwd_curve);

    let notional = 1_000_000.0;

    let swap = InterestRateSwap::new(
        "SWAP_DV01_TEST".into(),
        Money::new(notional, Currency::USD),
        0.05,
        as_of,
        end,
        PayReceive::ReceiveFixed,
    );

    let result = swap
        .price_with_metrics(&market, as_of, &[MetricId::Annuity, MetricId::Dv01])
        .unwrap();

    let annuity = *result.measures.get("annuity").unwrap();
    let dv01 = *result.measures.get("dv01").unwrap();

    // DV01 is computed via parallel bump-and-reprice (GenericParallelDv01)
    // For a ReceiveFixed swap, DV01 should be negative (loses value when rates rise)
    // For $1MM notional, 5-year swap at 5% rates, expect DV01 magnitude around $430-$450
    assert!(
        dv01.abs() > 400.0 && dv01.abs() < 500.0,
        "DV01={:.2} outside typical range $400-$500 for $1MM 5Y swap",
        dv01
    );

    // ReceiveFixed swap should have negative DV01 (loses value when rates increase)
    assert!(
        dv01 < 0.0,
        "ReceiveFixed swap should have negative DV01, got {:.2}",
        dv01
    );

    // Annuity approximation: DV01 ≈ Annuity × Notional × 0.0001
    // The parallel bump method is more accurate and should be within ~5% of the approximation
    let annuity_approx = annuity * notional * 0.0001;
    assert!(
        (dv01.abs() - annuity_approx).abs() / annuity_approx < 0.05,
        "DV01={:.2} differs from annuity approximation {:.2} by more than 5% (Annuity={:.4})",
        dv01,
        annuity_approx,
        annuity
    );
}

#[test]
fn test_irs_receive_vs_pay_fixed() {
    // Receive fixed and pay fixed should have opposite NPVs

    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);

    let disc_curve = build_flat_discount_curve(0.05, as_of, "USD-OIS");
    let fwd_curve = build_flat_forward_curve(0.06, as_of, "USD-SOFR-3M");

    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_forward(fwd_curve);

    let fixed_leg = FixedLegSpec {
        discount_curve_id: "USD-OIS".into(),
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
        discount_curve_id: "USD-OIS".into(),
        forward_curve_id: "USD-SOFR-3M".into(),
        spread_bp: 0.0,
        freq: Frequency::quarterly(),
        dc: DayCount::Act360,
        bdc: BusinessDayConvention::ModifiedFollowing,
        calendar_id: None,
        stub: StubKind::None,
        reset_lag_days: 2,
            compounding: Default::default(),
        start: as_of,
        end,
    };

    let swap_receive = InterestRateSwap {
        id: "SWAP_RECEIVE".into(),
        notional: Money::new(1_000_000.0, Currency::USD),
        side: PayReceive::ReceiveFixed,
        fixed: fixed_leg.clone(),
        float: float_leg.clone(),
        attributes: Default::default(),
    };

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
        discount_curve_id: "USD-OIS".into(),
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
        discount_curve_id: "USD-OIS".into(),
        forward_curve_id: "USD-SOFR-3M".into(),
        spread_bp: 0.0,
        freq: Frequency::quarterly(),
        dc: DayCount::Act360,
        bdc: BusinessDayConvention::ModifiedFollowing,
        calendar_id: None,
        stub: StubKind::None,
        reset_lag_days: 2,
            compounding: Default::default(),
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
        let disc_curve = build_flat_discount_curve(rate, as_of, "USD-OIS");
        let fwd_curve = build_flat_forward_curve(rate, as_of, "USD-SOFR-3M");

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
        par_npv.abs() < 2000.0,
        "At par rate, NPV should be near zero: {:.2}",
        par_npv
    );
}

#[test]
fn test_irs_leg_pvs_consistency() {
    // For receive fixed swap: NPV = PV(fixed leg) - PV(floating leg)

    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);

    let disc_curve = build_flat_discount_curve(0.05, as_of, "USD-OIS");
    let fwd_curve = build_flat_forward_curve(0.06, as_of, "USD-SOFR-3M");

    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_forward(fwd_curve);

    let swap = InterestRateSwap {
        id: "SWAP_LEG_PVS".into(),
        notional: Money::new(1_000_000.0, Currency::USD),
        side: PayReceive::ReceiveFixed,
        fixed: FixedLegSpec {
            discount_curve_id: "USD-OIS".into(),
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
            discount_curve_id: "USD-OIS".into(),
            forward_curve_id: "USD-SOFR-3M".into(),
            spread_bp: 0.0,
            freq: Frequency::quarterly(),
            dc: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: None,
            stub: StubKind::None,
            reset_lag_days: 2,
            compounding: Default::default(),
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
        (calculated_npv - base_npv).abs() < 100.0,
        "NPV from legs ({:.2}) should match total NPV ({:.2})",
        calculated_npv,
        base_npv
    );
}
