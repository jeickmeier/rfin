//! Tests for swaption pricers.

#![allow(clippy::unwrap_used)]

use crate::swaption::common::*;
use finstack_core::money::Money;
use finstack_valuations::instruments::rates::swaption::{BermudanSchedule, BermudanSwaption};
use finstack_valuations::instruments::rates::swaption::{
    BermudanSwaptionPricer, CalibratedHullWhiteModel, HullWhiteParams, SimpleSwaptionBlackPricer,
};
use finstack_valuations::instruments::Instrument;
use finstack_valuations::pricer::{ModelKey, Pricer};
use time::macros::date;

#[test]
fn test_simple_swaption_black_pricer_forces_black() {
    let (as_of, expiry, swap_start, swap_end) = standard_dates();
    let mut swaption = create_standard_payer_swaption(expiry, swap_start, swap_end, 0.05);
    swaption.vol_model = finstack_valuations::instruments::rates::swaption::VolatilityModel::Normal;
    swaption.pricing_overrides = swaption.pricing_overrides.clone().with_implied_vol(0.25);

    let market = create_flat_market(as_of, 0.03, 0.2);
    let expected_black = swaption.price_black(&market, 0.25, as_of).unwrap();
    let pricer = SimpleSwaptionBlackPricer::with_model(ModelKey::Black76);
    let result = pricer.price_dyn(&swaption, &market, as_of).unwrap().value;

    assert_approx_eq(
        result.amount(),
        expected_black.amount(),
        1e-10,
        "pricer black result",
    );
}

#[test]
fn test_simple_swaption_pricer_fallback_uses_instrument_value() {
    let (as_of, expiry, swap_start, swap_end) = standard_dates();
    let mut swaption = create_standard_payer_swaption(expiry, swap_start, swap_end, 0.05);
    swaption.vol_model = finstack_valuations::instruments::rates::swaption::VolatilityModel::Normal;
    swaption.pricing_overrides = swaption.pricing_overrides.clone().with_implied_vol(0.35);

    let market = create_flat_market(as_of, 0.03, 0.2);
    let pricer = SimpleSwaptionBlackPricer::with_model(ModelKey::Discounting);
    let result = pricer.price_dyn(&swaption, &market, as_of).unwrap().value;

    let expected = swaption.value(&market, as_of).unwrap();
    assert_approx_eq(
        result.amount(),
        expected.amount(),
        1e-10,
        "pricer fallback result",
    );
}

#[test]
fn test_bermudan_pricer_cached_model_sets_measure() {
    let as_of = date!(2025 - 01 - 01);
    let swap_start = as_of;
    let swap_end = date!(2030 - 01 - 01);
    let first_exercise = date!(2026 - 01 - 01);
    let swaption = BermudanSwaption::new_payer(
        "BERM-CACHED",
        Money::new(1_000_000.0, finstack_core::currency::Currency::USD),
        0.03,
        swap_start,
        swap_end,
        BermudanSchedule::co_terminal(
            first_exercise,
            swap_end,
            finstack_core::dates::Tenor::semi_annual(),
        )
        .expect("valid Bermudan schedule"),
        "USD_OIS",
        "USD_OIS",
        "USD-SWPNVOL",
    );

    let market = create_flat_market(as_of, 0.03, 0.2);
    let disc = market.get_discount("USD_OIS").unwrap();
    let ttm = swaption.time_to_maturity(as_of).unwrap();
    let model =
        CalibratedHullWhiteModel::calibrate(HullWhiteParams::default(), 50, disc.as_ref(), ttm)
            .unwrap();

    let pricer = BermudanSwaptionPricer::tree_pricer(HullWhiteParams::default())
        .with_calibrated_model(model);
    let result = pricer.price_dyn(&swaption, &market, as_of).unwrap();

    let used_cached = result.measures.get("used_cached_model").copied().unwrap();
    assert_eq!(used_cached, 1.0);
    assert!(result.value.amount() >= 0.0);
}

#[test]
fn test_bermudan_pricer_expired_returns_zero() {
    let as_of = date!(2035 - 01 - 01);
    let swap_start = date!(2025 - 01 - 01);
    let swap_end = date!(2030 - 01 - 01);
    let first_exercise = date!(2026 - 01 - 01);
    let swaption = BermudanSwaption::new_receiver(
        "BERM-EXPIRED",
        Money::new(2_000_000.0, finstack_core::currency::Currency::USD),
        0.04,
        swap_start,
        swap_end,
        BermudanSchedule::co_terminal(
            first_exercise,
            swap_end,
            finstack_core::dates::Tenor::semi_annual(),
        )
        .expect("valid Bermudan schedule"),
        "USD_OIS",
        "USD_OIS",
        "USD-SWPNVOL",
    );

    let market = create_flat_market(as_of, 0.03, 0.2);
    let pricer = BermudanSwaptionPricer::tree_pricer(HullWhiteParams::default());
    let result = pricer.price_dyn(&swaption, &market, as_of).unwrap();

    assert_approx_eq(result.value.amount(), 0.0, 1e-12, "expired bermudan pv");
}

#[test]
fn test_bermudan_tree_pricer_rejects_mixed_curves() {
    let as_of = date!(2025 - 01 - 01);
    let swap_start = as_of;
    let swap_end = date!(2030 - 01 - 01);
    let first_exercise = date!(2026 - 01 - 01);
    let swaption = BermudanSwaption::new_payer(
        "BERM-MIXED",
        Money::new(1_000_000.0, finstack_core::currency::Currency::USD),
        0.03,
        swap_start,
        swap_end,
        BermudanSchedule::co_terminal(
            first_exercise,
            swap_end,
            finstack_core::dates::Tenor::semi_annual(),
        )
        .expect("valid Bermudan schedule"),
        "USD_OIS",
        "USD-SOFR-3M",
        "USD-SWPNVOL",
    );

    let market = create_flat_market(as_of, 0.03, 0.2);
    let pricer = BermudanSwaptionPricer::tree_pricer(HullWhiteParams::default());
    let err = pricer
        .price_dyn(&swaption, &market, as_of)
        .expect_err("mixed-curve Bermudan tree pricer should reject pricing");
    let msg = err.to_string();
    assert!(
        msg.contains("single-curve"),
        "expected single-curve rejection error, got: {msg}"
    );
}
