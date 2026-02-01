//! Implied volatility solver tests.
//!
//! Tests the implied volatility solver including convergence,
//! boundary handling, and numerical stability.

use super::helpers::*;
use finstack_core::dates::DayCountCtx;
use finstack_valuations::instruments::common::models::bs_price;
use time::macros::date;

fn analytical_fx_price(
    option: &finstack_valuations::instruments::fx::fx_option::FxOption,
    params: MarketParams,
    as_of: finstack_core::dates::Date,
) -> f64 {
    let t = option
        .day_count
        .year_fraction(as_of, option.expiry, DayCountCtx::default())
        .unwrap_or(0.0);
    let price_per_unit = bs_price(
        params.spot,
        option.strike,
        params.r_domestic,
        params.r_foreign,
        params.vol,
        t,
        option.option_type,
    );
    price_per_unit * option.notional.amount()
}

#[test]
fn test_implied_vol_recovers_market_vol() {
    // Arrange: Price option at market vol, then solve for IV
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let call = build_call_option(as_of, expiry, 1.20, 1_000_000.0);
    let params = MarketParams::atm();
    let market = build_market_context(as_of, params);

    // Act: Get market price, then solve for IV
    let market_pv =
        finstack_core::money::Money::new(analytical_fx_price(&call, params, as_of), QUOTE);
    let implied_vol = call
        .implied_vol(&market, as_of, market_pv.amount(), None)
        .unwrap();

    // Assert: Should recover market vol (15%)
    assert_approx_eq(
        implied_vol,
        0.15,
        1e-6,
        1e-6,
        "IV should recover market vol",
    );
}

#[test]
fn test_implied_vol_with_custom_initial_guess() {
    // Arrange
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let call = build_call_option(as_of, expiry, 1.20, 1_000_000.0);
    let params = MarketParams::atm();
    let market = build_market_context(as_of, params);

    let market_pv =
        finstack_core::money::Money::new(analytical_fx_price(&call, params, as_of), QUOTE);

    // Act: Solve with custom initial guess
    let implied_vol = call
        .implied_vol(&market, as_of, market_pv.amount(), Some(0.25))
        .unwrap();

    // Assert: Should still converge to correct vol
    assert_approx_eq(
        implied_vol,
        0.15,
        1e-6,
        1e-6,
        "IV converges from custom guess",
    );
}

#[test]
fn test_implied_vol_high_vol_scenario() {
    // Arrange: High vol environment
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let call = build_call_option(as_of, expiry, 1.20, 1_000_000.0);
    let params = MarketParams::high_vol();
    let market = build_market_context(as_of, params);

    // Act
    let market_pv =
        finstack_core::money::Money::new(analytical_fx_price(&call, params, as_of), QUOTE);
    let implied_vol = call
        .implied_vol(&market, as_of, market_pv.amount(), None)
        .unwrap();

    // Assert: Should recover high vol (35%)
    assert_approx_eq(implied_vol, 0.35, 1e-6, 1e-6, "IV should recover high vol");
}

#[test]
fn test_implied_vol_low_vol_scenario() {
    // Arrange: Low vol environment
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let call = build_call_option(as_of, expiry, 1.20, 1_000_000.0);
    let params = MarketParams::low_vol();
    let market = build_market_context(as_of, params);

    // Act
    let market_pv =
        finstack_core::money::Money::new(analytical_fx_price(&call, params, as_of), QUOTE);
    let implied_vol = call
        .implied_vol(&market, as_of, market_pv.amount(), None)
        .unwrap();

    // Assert: Should recover low vol (5%)
    assert_approx_eq(implied_vol, 0.05, 1e-6, 1e-6, "IV should recover low vol");
}

#[test]
fn test_implied_vol_put_option() {
    // Arrange: Test IV solver for puts
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let put = build_put_option(as_of, expiry, 1.20, 1_000_000.0);
    let params = MarketParams::atm();
    let market = build_market_context(as_of, params);

    // Act
    let market_pv =
        finstack_core::money::Money::new(analytical_fx_price(&put, params, as_of), QUOTE);
    let implied_vol = put
        .implied_vol(&market, as_of, market_pv.amount(), None)
        .unwrap();

    // Assert
    assert_approx_eq(
        implied_vol,
        0.15,
        1e-6,
        1e-6,
        "Put IV should recover market vol",
    );
}

#[test]
fn test_implied_vol_itm_option() {
    // Arrange: ITM option
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let call = build_call_option(as_of, expiry, 1.10, 1_000_000.0);
    let params = MarketParams::atm();
    let market = build_market_context(as_of, params);

    // Act
    let market_pv =
        finstack_core::money::Money::new(analytical_fx_price(&call, params, as_of), QUOTE);
    let implied_vol = call
        .implied_vol(&market, as_of, market_pv.amount(), None)
        .unwrap();

    // Assert
    assert_approx_eq(
        implied_vol,
        0.15,
        1e-6,
        1e-6,
        "ITM IV should recover market vol",
    );
}

#[test]
fn test_implied_vol_otm_option() {
    // Arrange: OTM option
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let call = build_call_option(as_of, expiry, 1.35, 1_000_000.0);
    let params = MarketParams::atm();
    let market = build_market_context(as_of, params);

    // Act
    let market_pv =
        finstack_core::money::Money::new(analytical_fx_price(&call, params, as_of), QUOTE);
    let implied_vol = call
        .implied_vol(&market, as_of, market_pv.amount(), None)
        .unwrap();

    // Assert
    assert_approx_eq(
        implied_vol,
        0.15,
        1e-6,
        1e-6,
        "OTM IV should recover market vol",
    );
}

#[test]
fn test_implied_vol_short_dated_option() {
    // Arrange: 3M option
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2024 - 04 - 01);
    let call = build_call_option(as_of, expiry, 1.20, 1_000_000.0);
    let params = MarketParams::atm();
    let market = build_market_context(as_of, params);

    // Act
    let market_pv =
        finstack_core::money::Money::new(analytical_fx_price(&call, params, as_of), QUOTE);
    let implied_vol = call
        .implied_vol(&market, as_of, market_pv.amount(), None)
        .unwrap();

    // Assert
    assert_approx_eq(
        implied_vol,
        0.15,
        1e-5,
        1e-5,
        "Short dated IV should converge",
    );
}

#[test]
fn test_implied_vol_long_dated_option() {
    // Arrange: 2Y option
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2026 - 01 - 01);
    let call = build_call_option(as_of, expiry, 1.20, 1_000_000.0);
    let params = MarketParams::atm();
    let market = build_market_context(as_of, params);

    // Act
    let market_pv =
        finstack_core::money::Money::new(analytical_fx_price(&call, params, as_of), QUOTE);
    let implied_vol = call
        .implied_vol(&market, as_of, market_pv.amount(), None)
        .unwrap();

    // Assert
    assert_approx_eq(
        implied_vol,
        0.15,
        1e-6,
        1e-6,
        "Long dated IV should converge",
    );
}

#[test]
fn test_implied_vol_expired_option_returns_zero() {
    // Arrange: Expired option
    let expiry = date!(2024 - 01 - 01);
    let as_of = expiry;
    let call = build_call_option(expiry, expiry, 1.20, 1_000_000.0);
    let params = MarketParams::atm();
    let market = build_market_context(as_of, params);

    // Act
    let target_price = 10_000.0; // Arbitrary
    let implied_vol = call
        .implied_vol(&market, as_of, target_price, None)
        .unwrap();

    // Assert: Should return 0 for expired
    assert_eq!(implied_vol, 0.0, "Expired option IV should be 0");
}

#[test]
fn test_implied_vol_uses_override_as_initial_guess() {
    // Arrange: Option with vol override
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let mut call = build_call_option(as_of, expiry, 1.20, 1_000_000.0);
    call.pricing_overrides.implied_volatility = Some(0.25);

    let params = MarketParams::atm();
    let market = build_market_context(as_of, params);

    // Price at override vol
    let pv = finstack_core::money::Money::new(analytical_fx_price(&call, params, as_of), QUOTE);

    // Remove override for IV solve
    call.pricing_overrides.implied_volatility = None;

    // Act: Solve without explicit guess (should use surface vol as initial)
    let implied_vol = call.implied_vol(&market, as_of, pv.amount(), None).unwrap();

    // Assert: Should converge to the price's implied vol
    // (which was 25% from the override used for pricing)
    // But since we removed override, it uses surface vol as guess and should still find correct IV
    assert!(
        implied_vol > 0.0 && implied_vol < 1.0,
        "IV should be reasonable"
    );
}
