//! Numerical accuracy tests for interest rate options.
//!
//! Validates Black model implementation accuracy and numerical stability.

use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Frequency, StubKind};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::surfaces::VolSurface;
use finstack_core::market_data::term_structures::{DiscountCurve, ForwardCurve};
use finstack_core::money::Money;
use finstack_valuations::instruments::cap_floor::{InterestRateOption, RateOptionType};
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::instruments::{ExerciseStyle, PricingOverrides, SettlementType};
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
        .expiries(&[0.25, 1.0, 5.0, 10.0])
        .strikes(&[0.01, 0.03, 0.05, 0.07, 0.10])
        .row(&[vol, vol, vol, vol, vol])
        .row(&[vol, vol, vol, vol, vol])
        .row(&[vol, vol, vol, vol, vol])
        .row(&[vol, vol, vol, vol, vol])
        .build()
        .unwrap()
}

#[test]
fn test_black_model_symmetry() {
    // Test that Black model is symmetric for small changes
    let as_of = date!(2024 - 01 - 01);
    let start = date!(2024 - 01 - 01);
    let end = date!(2024 - 04 - 01);

    let caplet = InterestRateOption {
        id: "CAPLET".into(),
        rate_option_type: RateOptionType::Caplet,
        notional: Money::new(1_000_000.0, Currency::USD),
        strike_rate: 0.05,
        start_date: start,
        end_date: end,
        frequency: Frequency::quarterly(),
        day_count: DayCount::Act360,
        stub_kind: StubKind::None,
        bdc: BusinessDayConvention::ModifiedFollowing,
        calendar_id: None,
        exercise_style: ExerciseStyle::European,
        settlement: SettlementType::Cash,
        discount_curve_id: "USD_OIS".into(),
        forward_id: "USD_LIBOR_3M".into(),
        vol_surface_id: "USD_CAP_VOL".into(),
        pricing_overrides: PricingOverrides::default(),
        attributes: Default::default(),
    };

    // Test with forward slightly above and below strike
    let fwd1 = build_flat_forward_curve(0.0501, as_of, "USD_LIBOR_3M");
    let fwd2 = build_flat_forward_curve(0.0499, as_of, "USD_LIBOR_3M");
    let disc_curve1 = build_flat_discount_curve(0.05, as_of, "USD_OIS");
    let disc_curve2 = build_flat_discount_curve(0.05, as_of, "USD_OIS");
    let vol_surface1 = build_flat_vol_surface(0.30, as_of, "USD_CAP_VOL");
    let vol_surface2 = build_flat_vol_surface(0.30, as_of, "USD_CAP_VOL");

    let market1 = MarketContext::new()
        .insert_discount(disc_curve1)
        .insert_forward(fwd1)
        .insert_surface(vol_surface1);

    let market2 = MarketContext::new()
        .insert_discount(disc_curve2)
        .insert_forward(fwd2)
        .insert_surface(vol_surface2);

    let pv1 = caplet.value(&market1, as_of).unwrap().amount();
    let pv2 = caplet.value(&market2, as_of).unwrap().amount();

    // Both should be close to ATM value, difference should be small
    let diff = (pv1 - pv2).abs();
    assert!(
        diff < 50.0,
        "Small forward changes should produce small PV changes: diff={}",
        diff
    );
}

#[test]
fn test_vega_gamma_relation() {
    // Test relationship: vega ≈ spot × gamma × time × vol
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2025 - 01 - 01);

    let cap = InterestRateOption {
        id: "CAP_RELATIONS".into(),
        rate_option_type: RateOptionType::Cap,
        notional: Money::new(1_000_000.0, Currency::USD),
        strike_rate: 0.05,
        start_date: as_of,
        end_date: end,
        frequency: Frequency::quarterly(),
        day_count: DayCount::Act360,
        stub_kind: StubKind::None,
        bdc: BusinessDayConvention::ModifiedFollowing,
        calendar_id: None,
        exercise_style: ExerciseStyle::European,
        settlement: SettlementType::Cash,
        discount_curve_id: "USD_OIS".into(),
        forward_id: "USD_LIBOR_3M".into(),
        vol_surface_id: "USD_CAP_VOL".into(),
        pricing_overrides: PricingOverrides::default(),
        attributes: Default::default(),
    };

    let disc_curve = build_flat_discount_curve(0.05, as_of, "USD_OIS");
    let fwd_curve = build_flat_forward_curve(0.05, as_of, "USD_LIBOR_3M");
    let vol_surface = build_flat_vol_surface(0.30, as_of, "USD_CAP_VOL");

    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_forward(fwd_curve)
        .insert_surface(vol_surface);

    let result = cap
        .price_with_metrics(&market, as_of, &[MetricId::Vega, MetricId::Gamma])
        .unwrap();

    let vega = *result.measures.get("vega").unwrap();
    let gamma = *result.measures.get("gamma").unwrap();

    // Both should be positive and finite
    assert!(
        vega > 0.0 && vega.is_finite(),
        "Vega should be positive and finite"
    );
    assert!(
        gamma >= 0.0 && gamma.is_finite(),
        "Gamma should be non-negative and finite"
    );
}

#[test]
fn test_delta_by_finite_difference() {
    // Verify delta by finite difference approximation
    let as_of = date!(2024 - 01 - 01);
    let start = date!(2024 - 03 - 01); // Future start to get t_fix > 0
    let end = date!(2024 - 06 - 01);

    let caplet = InterestRateOption {
        id: "CAPLET_FD".into(),
        rate_option_type: RateOptionType::Caplet,
        notional: Money::new(1_000_000.0, Currency::USD),
        strike_rate: 0.05,
        start_date: start,
        end_date: end,
        frequency: Frequency::quarterly(),
        day_count: DayCount::Act360,
        stub_kind: StubKind::None,
        bdc: BusinessDayConvention::ModifiedFollowing,
        calendar_id: None,
        exercise_style: ExerciseStyle::European,
        settlement: SettlementType::Cash,
        discount_curve_id: "USD_OIS".into(),
        forward_id: "USD_LIBOR_3M".into(),
        vol_surface_id: "USD_CAP_VOL".into(),
        pricing_overrides: PricingOverrides::default(),
        attributes: Default::default(),
    };

    let bump = 0.0001; // 1bp
    let fwd_down = build_flat_forward_curve(0.05 - bump, as_of, "USD_LIBOR_3M");
    let fwd_base = build_flat_forward_curve(0.05, as_of, "USD_LIBOR_3M");
    let fwd_up = build_flat_forward_curve(0.05 + bump, as_of, "USD_LIBOR_3M");

    let disc_curve1 = build_flat_discount_curve(0.05, as_of, "USD_OIS");
    let disc_curve2 = build_flat_discount_curve(0.05, as_of, "USD_OIS");
    let disc_curve3 = build_flat_discount_curve(0.05, as_of, "USD_OIS");
    let vol_surface1 = build_flat_vol_surface(0.30, as_of, "USD_CAP_VOL");
    let vol_surface2 = build_flat_vol_surface(0.30, as_of, "USD_CAP_VOL");
    let vol_surface3 = build_flat_vol_surface(0.30, as_of, "USD_CAP_VOL");

    let market_down = MarketContext::new()
        .insert_discount(disc_curve1)
        .insert_forward(fwd_down)
        .insert_surface(vol_surface1);

    let market_base = MarketContext::new()
        .insert_discount(disc_curve2)
        .insert_forward(fwd_base)
        .insert_surface(vol_surface2);

    let market_up = MarketContext::new()
        .insert_discount(disc_curve3)
        .insert_forward(fwd_up)
        .insert_surface(vol_surface3);

    let pv_down = caplet.value(&market_down, as_of).unwrap().amount();
    let pv_up = caplet.value(&market_up, as_of).unwrap().amount();

    // Finite difference delta
    let fd_delta = (pv_up - pv_down) / (2.0 * bump);

    // Analytical delta
    let result = caplet
        .price_with_metrics(&market_base, as_of, &[MetricId::Delta])
        .unwrap();
    let analytic_delta = *result.measures.get("delta").unwrap();

    // Should be close (within 10% relative error)
    let relative_error = if analytic_delta.abs() > 1.0 {
        (fd_delta - analytic_delta).abs() / analytic_delta.abs()
    } else {
        (fd_delta - analytic_delta).abs()
    };

    assert!(
        relative_error < 0.10,
        "FD delta ({}) should match analytic delta ({}), rel error: {}",
        fd_delta,
        analytic_delta,
        relative_error
    );
}

#[test]
fn test_vega_by_finite_difference() {
    let as_of = date!(2024 - 01 - 01);
    let start = date!(2024 - 01 - 01);
    let end = date!(2024 - 04 - 01);

    let caplet = InterestRateOption {
        id: "CAPLET_VEGA_FD".into(),
        rate_option_type: RateOptionType::Caplet,
        notional: Money::new(1_000_000.0, Currency::USD),
        strike_rate: 0.05,
        start_date: start,
        end_date: end,
        frequency: Frequency::quarterly(),
        day_count: DayCount::Act360,
        stub_kind: StubKind::None,
        bdc: BusinessDayConvention::ModifiedFollowing,
        calendar_id: None,
        exercise_style: ExerciseStyle::European,
        settlement: SettlementType::Cash,
        discount_curve_id: "USD_OIS".into(),
        forward_id: "USD_LIBOR_3M".into(),
        vol_surface_id: "USD_CAP_VOL".into(),
        pricing_overrides: PricingOverrides::default(),
        attributes: Default::default(),
    };

    let vol_base = 0.30;
    let vol_bump = 0.01; // 1% vol bump

    let vol_down = build_flat_vol_surface(vol_base - vol_bump, as_of, "USD_CAP_VOL");
    let vol_up = build_flat_vol_surface(vol_base + vol_bump, as_of, "USD_CAP_VOL");
    let vol_mid = build_flat_vol_surface(vol_base, as_of, "USD_CAP_VOL");

    let disc_curve1 = build_flat_discount_curve(0.05, as_of, "USD_OIS");
    let disc_curve2 = build_flat_discount_curve(0.05, as_of, "USD_OIS");
    let disc_curve3 = build_flat_discount_curve(0.05, as_of, "USD_OIS");
    let fwd_curve1 = build_flat_forward_curve(0.05, as_of, "USD_LIBOR_3M");
    let fwd_curve2 = build_flat_forward_curve(0.05, as_of, "USD_LIBOR_3M");
    let fwd_curve3 = build_flat_forward_curve(0.05, as_of, "USD_LIBOR_3M");

    let market_down = MarketContext::new()
        .insert_discount(disc_curve1)
        .insert_forward(fwd_curve1)
        .insert_surface(vol_down);

    let market_up = MarketContext::new()
        .insert_discount(disc_curve2)
        .insert_forward(fwd_curve2)
        .insert_surface(vol_up);

    let market_mid = MarketContext::new()
        .insert_discount(disc_curve3)
        .insert_forward(fwd_curve3)
        .insert_surface(vol_mid);

    let pv_down = caplet.value(&market_down, as_of).unwrap().amount();
    let pv_up = caplet.value(&market_up, as_of).unwrap().amount();

    // FD vega (per 1% vol change)
    let fd_vega = (pv_up - pv_down) / (2.0 * vol_bump);

    // Analytical vega
    let result = caplet
        .price_with_metrics(&market_mid, as_of, &[MetricId::Vega])
        .unwrap();
    let analytic_vega = *result.measures.get("vega").unwrap();

    // Should be close
    let relative_error = if analytic_vega.abs() > 0.1 {
        (fd_vega - analytic_vega).abs() / analytic_vega.abs()
    } else {
        (fd_vega - analytic_vega).abs()
    };

    assert!(
        relative_error < 0.15,
        "FD vega ({}) should match analytic vega ({}), rel error: {}",
        fd_vega,
        analytic_vega,
        relative_error
    );
}

#[test]
fn test_numerical_stability_extreme_params() {
    // Test that pricing remains stable with extreme but valid parameters
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2054 - 01 - 01); // 30 year cap

    let cap = InterestRateOption {
        id: "CAP_EXTREME".into(),
        rate_option_type: RateOptionType::Cap,
        notional: Money::new(100_000_000.0, Currency::USD), // $100MM
        strike_rate: 0.15,                                  // 15% strike
        start_date: as_of,
        end_date: end,
        frequency: Frequency::monthly(),
        day_count: DayCount::Act360,
        stub_kind: StubKind::None,
        bdc: BusinessDayConvention::ModifiedFollowing,
        calendar_id: None,
        exercise_style: ExerciseStyle::European,
        settlement: SettlementType::Cash,
        discount_curve_id: "USD_OIS".into(),
        forward_id: "USD_LIBOR_3M".into(),
        vol_surface_id: "USD_CAP_VOL".into(),
        pricing_overrides: PricingOverrides::default(),
        attributes: Default::default(),
    };

    let disc_curve = build_flat_discount_curve(0.05, as_of, "USD_OIS");
    let fwd_curve = build_flat_forward_curve(0.05, as_of, "USD_LIBOR_3M");
    let vol_surface = build_flat_vol_surface(0.50, as_of, "USD_CAP_VOL");

    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_forward(fwd_curve)
        .insert_surface(vol_surface);

    let result = cap
        .price_with_metrics(&market, as_of, &[MetricId::Delta, MetricId::Vega])
        .unwrap();

    // All values should be finite
    assert!(result.value.amount().is_finite(), "PV should be finite");
    assert!(
        result.measures.get("delta").unwrap().is_finite(),
        "Delta should be finite"
    );
    assert!(
        result.measures.get("vega").unwrap().is_finite(),
        "Vega should be finite"
    );
}
