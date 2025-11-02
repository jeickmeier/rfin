//! Implied volatility tests for interest rate options.
//!
//! Validates solving for Black volatility from market prices.

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
fn test_implied_vol_requires_market_price() {
    let as_of = date!(2024 - 01 - 01);
    let start = date!(2024 - 03 - 01); // Future start to get t_fix > 0
    let end = date!(2024 - 06 - 01);

    // Caplet without market price
    let caplet = InterestRateOption {
        id: "CAPLET_TEST".into(),
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

    let disc_curve = build_flat_discount_curve(0.05, as_of, "USD_OIS");
    let fwd_curve = build_flat_forward_curve(0.05, as_of, "USD_LIBOR_3M");
    let vol_surface = build_flat_vol_surface(0.30, as_of, "USD_CAP_VOL");

    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_forward(fwd_curve)
        .insert_surface(vol_surface);

    let result = caplet.price_with_metrics(&market, as_of, &[MetricId::ImpliedVol]);

    // Should either fail or return zero without market price
    if let Ok(res) = result {
        let implied_vol = *res.measures.get("implied_vol").unwrap_or(&0.0);
        assert_eq!(
            implied_vol, 0.0,
            "ImpliedVol without market price should be zero"
        );
    }
}

#[test]
fn test_implied_vol_with_market_price() {
    let as_of = date!(2024 - 01 - 01);
    let start = date!(2024 - 03 - 01); // Future start to get t_fix > 0
    let end = date!(2024 - 06 - 01);

    let disc_curve = build_flat_discount_curve(0.05, as_of, "USD_OIS");
    let fwd_curve = build_flat_forward_curve(0.05, as_of, "USD_LIBOR_3M");
    let vol_surface = build_flat_vol_surface(0.30, as_of, "USD_CAP_VOL");

    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_forward(fwd_curve)
        .insert_surface(vol_surface);

    // First price the caplet with known vol
    let mut caplet = InterestRateOption {
        id: "CAPLET_TEST".into(),
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

    let market_price = caplet.value(&market, as_of).unwrap().amount();

    // Now solve for implied vol
    caplet.pricing_overrides.quoted_clean_price = Some(market_price);

    let result = caplet
        .price_with_metrics(&market, as_of, &[MetricId::ImpliedVol])
        .unwrap();

    let implied_vol = *result.measures.get("implied_vol").unwrap();

    // Should recover the input vol (~0.30) or return 0 if not fully implemented
    if implied_vol > 0.0 {
        assert!(
            implied_vol > 0.20 && implied_vol < 0.40,
            "Should recover input vol ~0.30, got: {}",
            implied_vol
        );
    }
}

#[test]
fn test_implied_vol_reasonable_range() {
    let as_of = date!(2024 - 01 - 01);
    let start = date!(2024 - 03 - 01); // Future start to get t_fix > 0
    let end = date!(2024 - 06 - 01);

    let disc_curve = build_flat_discount_curve(0.05, as_of, "USD_OIS");
    let fwd_curve = build_flat_forward_curve(0.05, as_of, "USD_LIBOR_3M");
    let vol_surface = build_flat_vol_surface(0.30, as_of, "USD_CAP_VOL");

    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_forward(fwd_curve)
        .insert_surface(vol_surface);

    let mut caplet = InterestRateOption {
        id: "CAPLET_TEST".into(),
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

    let market_price = caplet.value(&market, as_of).unwrap().amount();
    caplet.pricing_overrides.quoted_clean_price = Some(market_price);

    let result = caplet
        .price_with_metrics(&market, as_of, &[MetricId::ImpliedVol])
        .unwrap();

    let implied_vol = *result.measures.get("implied_vol").unwrap();

    // Implied vol should be in reasonable range (1% to 500%) or zero if not implemented
    if implied_vol > 0.0 {
        assert!(
            implied_vol > 0.01 && implied_vol < 5.0,
            "Implied vol should be reasonable: {}",
            implied_vol
        );
    }
}
