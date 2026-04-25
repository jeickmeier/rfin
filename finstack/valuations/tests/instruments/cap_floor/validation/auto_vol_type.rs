//! Tests for `CapFloorVolType::Auto` model selection.
//!
//! Validates that `Auto` correctly selects Black (lognormal) for positive rates
//! and Normal (Bachelier) for negative/zero rates, both for single caplets
//! and portfolio-level cap/floor pricing.

use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, StubKind, Tenor};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::surfaces::VolSurface;
use finstack_core::market_data::term_structures::{DiscountCurve, ForwardCurve};
use finstack_core::money::Money;
use finstack_valuations::instruments::rates::cap_floor::{
    CapFloor, CapFloorVolType, RateOptionType,
};
use finstack_valuations::instruments::ExerciseStyle;
use finstack_valuations::instruments::Instrument;
use rust_decimal::Decimal;
use time::macros::date;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn flat_fwd(rate: f64, base: Date, id: &str) -> ForwardCurve {
    ForwardCurve::builder(id, 0.25)
        .base_date(base)
        .day_count(DayCount::Act360)
        .knots([(0.0, rate), (5.0, rate)])
        .build()
        .unwrap()
}

fn flat_disc(rate: f64, base: Date, id: &str) -> DiscountCurve {
    DiscountCurve::builder(id)
        .base_date(base)
        .day_count(DayCount::Act360)
        .knots([(0.0, 1.0), (1.0, (-rate).exp()), (5.0, (-rate * 5.0).exp())])
        .build()
        .unwrap()
}

fn flat_vol_surface(sigma: f64, id: &str) -> VolSurface {
    VolSurface::builder(id)
        .expiries(&[0.25, 0.5, 1.0, 2.0])
        .strikes(&[-0.02, -0.01, 0.0, 0.01, 0.02, 0.03, 0.05])
        .row(&[sigma; 7])
        .row(&[sigma; 7])
        .row(&[sigma; 7])
        .row(&[sigma; 7])
        .build()
        .unwrap()
}

fn make_caplet(
    fixing: Date,
    payment: Date,
    strike: f64,
    vol_type: CapFloorVolType,
    is_cap: bool,
) -> CapFloor {
    let rate_option_type = if is_cap {
        RateOptionType::Caplet
    } else {
        RateOptionType::Floorlet
    };
    CapFloor {
        id: "TEST-AUTO".into(),
        rate_option_type,
        notional: Money::new(1_000_000.0, Currency::EUR),
        strike: Decimal::try_from(strike).unwrap(),
        start_date: fixing,
        maturity: payment,
        frequency: Tenor::quarterly(),
        day_count: DayCount::Act360,
        stub: StubKind::None,
        bdc: BusinessDayConvention::ModifiedFollowing,
        calendar_id: None,
        exercise_style: ExerciseStyle::European,
        settlement: finstack_valuations::instruments::SettlementType::Cash,
        discount_curve_id: "DISC".into(),
        forward_curve_id: "FWD".into(),
        vol_surface_id: "VOL".into(),
        vol_type,
        vol_shift: 0.0,
        pricing_overrides: Default::default(),
        attributes: Default::default(),
    }
}

fn context_from(as_of: Date, fwd_rate: f64, sigma: f64) -> MarketContext {
    MarketContext::new()
        .insert(flat_disc(fwd_rate.max(0.001), as_of, "DISC"))
        .insert(flat_fwd(fwd_rate, as_of, "FWD"))
        .insert_surface(flat_vol_surface(sigma, "VOL"))
}

// ---------------------------------------------------------------------------
// Auto selects Black for positive rates
// ---------------------------------------------------------------------------

#[test]
fn auto_selects_black_for_positive_rates() {
    let as_of = date!(2024 - 01 - 01);
    let fixing = date!(2024 - 04 - 01);
    let payment = date!(2024 - 07 - 01);
    let fwd_rate = 0.03; // positive
    let strike = 0.03; // positive
    let sigma = 0.20; // lognormal vol

    let auto_cap = make_caplet(fixing, payment, strike, CapFloorVolType::Auto, true);
    let black_cap = make_caplet(fixing, payment, strike, CapFloorVolType::Lognormal, true);

    let ctx = context_from(as_of, fwd_rate, sigma);

    let auto_pv = auto_cap.value(&ctx, as_of).expect("Auto should succeed");
    let black_pv = black_cap.value(&ctx, as_of).expect("Black should succeed");

    // Auto should produce the same result as Black for positive rates
    let diff = (auto_pv.amount() - black_pv.amount()).abs();
    assert!(
        diff < 1e-10,
        "Auto should match Black for positive rates: auto={}, black={}, diff={}",
        auto_pv.amount(),
        black_pv.amount(),
        diff
    );
}

// ---------------------------------------------------------------------------
// Auto selects Normal for negative rates
// ---------------------------------------------------------------------------

#[test]
fn auto_selects_normal_for_negative_forward() {
    let as_of = date!(2024 - 01 - 01);
    let fixing = date!(2024 - 04 - 01);
    let payment = date!(2024 - 07 - 01);
    let fwd_rate = -0.005; // negative forward (EUR environment)
    let strike = 0.0; // ATM-ish
    let sigma = 0.005; // normal vol (50bp)

    let auto_cap = make_caplet(fixing, payment, strike, CapFloorVolType::Auto, true);
    let normal_cap = make_caplet(fixing, payment, strike, CapFloorVolType::Normal, true);

    let ctx = context_from(as_of, fwd_rate, sigma);

    let auto_pv = auto_cap
        .value(&ctx, as_of)
        .expect("Auto should succeed with negative forward");
    let normal_pv = normal_cap
        .value(&ctx, as_of)
        .expect("Normal should succeed with negative forward");

    // Auto should produce the same result as Normal for negative rates
    let diff = (auto_pv.amount() - normal_pv.amount()).abs();
    assert!(
        diff < 1e-10,
        "Auto should match Normal for negative forward: auto={}, normal={}, diff={}",
        auto_pv.amount(),
        normal_pv.amount(),
        diff
    );
}

#[test]
fn auto_does_not_error_on_negative_forward() {
    let as_of = date!(2024 - 01 - 01);
    let fixing = date!(2024 - 04 - 01);
    let payment = date!(2024 - 07 - 01);
    let fwd_rate = -0.005;
    let strike = -0.002;
    let sigma = 0.005;

    let auto_cap = make_caplet(fixing, payment, strike, CapFloorVolType::Auto, true);
    let ctx = context_from(as_of, fwd_rate, sigma);

    // Auto should NOT error (selects normal when Black domain is invalid)
    let result = auto_cap.value(&ctx, as_of);
    assert!(
        result.is_ok(),
        "Auto vol type should handle negative forward without error: {:?}",
        result.err()
    );
}

#[test]
fn lognormal_falls_back_to_normal_on_negative_forward() {
    let as_of = date!(2024 - 01 - 01);
    let fixing = date!(2024 - 04 - 01);
    let payment = date!(2024 - 07 - 01);
    let fwd_rate = -0.005;
    let strike = 0.0;
    let sigma = 0.20;

    let black_cap = make_caplet(fixing, payment, strike, CapFloorVolType::Lognormal, true);
    let ctx = context_from(as_of, fwd_rate, sigma);

    // Lognormal (Black) is undefined for F <= 0; we fall back to normal (Bachelier) pricing.
    let pv = black_cap
        .value(&ctx, as_of)
        .expect("lognormal should auto-fallback when forward is non-positive");
    assert!(
        pv.amount().is_finite() && pv.amount() >= 0.0,
        "expected finite non-negative PV, got {}",
        pv.amount()
    );
}

// ---------------------------------------------------------------------------
// Serde round-trip
// ---------------------------------------------------------------------------

#[test]
fn auto_vol_type_serde_round_trip() {
    let json = serde_json::to_string(&CapFloorVolType::Auto).unwrap();
    assert_eq!(json, "\"auto\"");

    let deserialized: CapFloorVolType = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, CapFloorVolType::Auto);
}

#[test]
fn auto_vol_type_from_str() {
    let parsed: CapFloorVolType = "auto".parse().unwrap();
    assert_eq!(parsed, CapFloorVolType::Auto);
}

#[test]
fn auto_vol_type_display() {
    assert_eq!(CapFloorVolType::Auto.to_string(), "auto");
}
