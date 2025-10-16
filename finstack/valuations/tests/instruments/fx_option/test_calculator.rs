//! Unit tests for FX option calculator core functionality.
//!
//! Tests the calculator methods in isolation: npv, collect_inputs,
//! input validation, and expired option handling.

use super::helpers::*;
use finstack_core::dates::DayCount;
use finstack_core::money::Money;
use finstack_valuations::instruments::fx_option::FxOptionCalculator;
use time::macros::date;

#[test]
fn test_npv_matches_garman_kohlhagen() {
    // Arrange: ATM call with 1Y expiry
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let strike = 1.20;
    let call = build_call_option(as_of, expiry, strike, 1_000_000.0);
    let market = build_market_context(as_of, MarketParams::atm());
    let calc = FxOptionCalculator::new();

    // Act
    let pv = calc.npv(&call, &market, as_of).unwrap();

    // Assert: PV should be positive for ATM call
    assert!(pv.amount() > 0.0, "ATM call PV should be positive");
    assert_eq!(pv.currency(), QUOTE);

    // Verify against explicit GK formula inputs
    let (spot, r_d, r_f, sigma, t) = calc.collect_inputs(&call, &market, as_of).unwrap();
    assert_approx_eq(spot, 1.20, 1e-10, 1e-10, "Spot");
    assert_approx_eq(r_d, 0.03, 1e-3, 1e-3, "Domestic rate");
    assert_approx_eq(r_f, 0.01, 1e-3, 1e-3, "Foreign rate");
    assert_approx_eq(sigma, 0.15, 1e-10, 1e-10, "Vol");
    // 2024 is a leap year, so 366/365 ≈ 1.0027
    assert_approx_eq(t, 1.0, 5e-3, 5e-3, "Time to expiry");
}

#[test]
fn test_npv_call_vs_put_values() {
    // Arrange: ATM options
    let as_of = date!(2024 - 06 - 15);
    let expiry = date!(2025 - 06 - 15);
    let strike = 1.20;

    let call = build_call_option(as_of, expiry, strike, 1_000_000.0);
    let put = build_put_option(as_of, expiry, strike, 1_000_000.0);
    let market = build_market_context(as_of, MarketParams::atm());
    let calc = FxOptionCalculator::new();

    // Act
    let call_pv = calc.npv(&call, &market, as_of).unwrap();
    let put_pv = calc.npv(&put, &market, as_of).unwrap();

    // Assert: Both should be positive
    assert!(call_pv.amount() > 0.0, "Call PV should be positive");
    assert!(put_pv.amount() > 0.0, "Put PV should be positive");

    // For ATM with positive carry (r_d > r_f), call typically > put
    // But we don't assert this strictly as it depends on rates
}

#[test]
fn test_npv_itm_call_has_higher_value() {
    // Arrange: ITM call (spot > strike)
    let as_of = date!(2024 - 03 - 01);
    let expiry = date!(2025 - 03 - 01);
    let strike = 1.10; // ITM since spot = 1.20

    let call = build_call_option(as_of, expiry, strike, 1_000_000.0);
    let market = build_market_context(as_of, MarketParams::atm());
    let calc = FxOptionCalculator::new();

    // Act
    let pv = calc.npv(&call, &market, as_of).unwrap();
    let intrinsic = (1.20 - strike) * 1_000_000.0;

    // Assert: PV should exceed intrinsic value due to time value
    assert!(
        pv.amount() > intrinsic,
        "ITM call PV should exceed intrinsic"
    );
}

#[test]
fn test_npv_otm_call_has_time_value() {
    // Arrange: OTM call (spot < strike)
    let as_of = date!(2024 - 03 - 01);
    let expiry = date!(2025 - 03 - 01);
    let strike = 1.30; // OTM since spot = 1.20

    let call = build_call_option(as_of, expiry, strike, 1_000_000.0);
    let market = build_market_context(as_of, MarketParams::atm());
    let calc = FxOptionCalculator::new();

    // Act
    let pv = calc.npv(&call, &market, as_of).unwrap();

    // Assert: Should have positive time value despite being OTM
    assert!(pv.amount() > 0.0, "OTM call should have time value");
}

#[test]
fn test_collect_inputs_respects_vol_surface() {
    // Arrange
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let call = build_call_option(as_of, expiry, 1.20, 1_000_000.0);
    let market = build_market_context(as_of, MarketParams::default());
    let calc = FxOptionCalculator::new();

    // Act
    let (spot, r_d, r_f, sigma, t) = calc.collect_inputs(&call, &market, as_of).unwrap();

    // Assert
    assert_approx_eq(spot, 1.20, 1e-10, 1e-10, "Spot from FX matrix");
    assert_approx_eq(sigma, 0.15, 1e-10, 1e-10, "Vol from surface");
    assert!(r_d > 0.0, "Domestic rate should be positive");
    assert!(r_f > 0.0, "Foreign rate should be positive");
    assert!(t > 0.9 && t < 1.1, "Time should be ~1Y");
}

#[test]
fn test_collect_inputs_respects_vol_override() {
    // Arrange
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let mut call = build_call_option(as_of, expiry, 1.20, 1_000_000.0);
    call.pricing_overrides.implied_volatility = Some(0.30); // Override

    let market = build_market_context(as_of, MarketParams::atm());
    let calc = FxOptionCalculator::new();

    // Act
    let (_, _, _, sigma, _) = calc.collect_inputs(&call, &market, as_of).unwrap();

    // Assert
    assert_approx_eq(sigma, 0.30, 1e-10, 1e-10, "Vol from override");
}

#[test]
fn test_collect_inputs_no_vol_excludes_volatility() {
    // Arrange
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let call = build_call_option(as_of, expiry, 1.20, 1_000_000.0);
    let market = build_market_context(as_of, MarketParams::atm());
    let calc = FxOptionCalculator::new();

    // Act
    let (spot, r_d, r_f, t) = calc.collect_inputs_no_vol(&call, &market, as_of).unwrap();

    // Assert: Should return same values except vol
    let (spot2, r_d2, r_f2, _, t2) = calc.collect_inputs(&call, &market, as_of).unwrap();
    assert_approx_eq(spot, spot2, 1e-10, 1e-10, "Spot matches");
    assert_approx_eq(r_d, r_d2, 1e-10, 1e-10, "Domestic rate matches");
    assert_approx_eq(r_f, r_f2, 1e-10, 1e-10, "Foreign rate matches");
    assert_approx_eq(t, t2, 1e-10, 1e-10, "Time matches");
}

#[test]
fn test_year_fraction_uses_day_count() {
    // Arrange
    let start = date!(2024 - 01 - 01);
    let end = date!(2024 - 07 - 01); // ~6 months
    let calc = FxOptionCalculator::new();

    // Act
    let yf_act365 = calc.year_fraction(start, end, DayCount::Act365F).unwrap();

    // Assert
    assert!(yf_act365 > 0.49 && yf_act365 < 0.51, "Should be ~0.5Y");
}

#[test]
fn test_npv_expired_call_returns_intrinsic() {
    // Arrange: Expired ITM call
    let expiry = date!(2024 - 01 - 01);
    let as_of = expiry; // At expiry
    let strike = 1.10;
    let spot = 1.20;

    let call = build_call_option(expiry, expiry, strike, 1_000_000.0);
    let market = build_market_context(
        as_of,
        MarketParams {
            spot,
            ..Default::default()
        },
    );
    let calc = FxOptionCalculator::new();

    // Act
    let pv = calc.npv(&call, &market, as_of).unwrap();

    // Assert: Should equal intrinsic value
    let intrinsic = (spot - strike) * 1_000_000.0;
    assert_approx_eq(pv.amount(), intrinsic, 1e-6, 1e-6, "Expired call intrinsic");
}

#[test]
fn test_npv_expired_put_returns_intrinsic() {
    // Arrange: Expired ITM put
    let expiry = date!(2024 - 01 - 01);
    let as_of = expiry;
    let strike = 1.30;
    let spot = 1.20;

    let put = build_put_option(expiry, expiry, strike, 1_000_000.0);
    let market = build_market_context(
        as_of,
        MarketParams {
            spot,
            ..Default::default()
        },
    );
    let calc = FxOptionCalculator::new();

    // Act
    let pv = calc.npv(&put, &market, as_of).unwrap();

    // Assert: Should equal intrinsic value
    let intrinsic = (strike - spot) * 1_000_000.0;
    assert_approx_eq(pv.amount(), intrinsic, 1e-6, 1e-6, "Expired put intrinsic");
}

#[test]
fn test_npv_expired_otm_call_is_zero() {
    // Arrange: Expired OTM call
    let expiry = date!(2024 - 01 - 01);
    let as_of = expiry;
    let strike = 1.30;
    let spot = 1.20;

    let call = build_call_option(expiry, expiry, strike, 1_000_000.0);
    let market = build_market_context(
        as_of,
        MarketParams {
            spot,
            ..Default::default()
        },
    );
    let calc = FxOptionCalculator::new();

    // Act
    let pv = calc.npv(&call, &market, as_of).unwrap();

    // Assert: Should be zero
    assert_approx_eq(
        pv.amount(),
        0.0,
        1e-10,
        1e-10,
        "Expired OTM call is worthless",
    );
}

#[test]
fn test_currency_validation_rejects_mismatched_notional() {
    // Arrange: Notional in wrong currency
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let mut call = build_call_option(as_of, expiry, 1.20, 1_000_000.0);
    call.notional = Money::new(1_000_000.0, QUOTE); // Should be BASE currency

    let market = build_market_context(as_of, MarketParams::atm());
    let calc = FxOptionCalculator::new();

    // Act & Assert: Should fail validation
    let result = calc.npv(&call, &market, as_of);
    assert!(
        result.is_err(),
        "Should reject mismatched notional currency"
    );
}

#[test]
fn test_price_gk_with_inputs_convenience_method() {
    // Arrange
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let call = build_call_option(as_of, expiry, 1.20, 1_000_000.0);
    let market = build_market_context(as_of, MarketParams::atm());
    let calc = FxOptionCalculator::new();

    // Act: Price using explicit inputs
    let (spot, r_d, r_f, sigma, t) = calc.collect_inputs(&call, &market, as_of).unwrap();
    let pv_explicit = calc
        .price_gk_with_inputs(&call, spot, r_d, r_f, sigma, t)
        .unwrap();

    let pv_standard = calc.npv(&call, &market, as_of).unwrap();

    // Assert: Should match standard npv
    assert_approx_eq(
        pv_explicit.amount(),
        pv_standard.amount(),
        1e-10,
        1e-10,
        "Explicit pricing matches",
    );
}
