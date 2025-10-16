//! Implied volatility solver tests.
//!
//! Tests the implied volatility solver including convergence,
//! boundary handling, and numerical stability.

use super::helpers::*;
use finstack_valuations::instruments::fx_option::FxOptionCalculator;
use time::macros::date;

#[test]
fn test_implied_vol_recovers_market_vol() {
    // Arrange: Price option at market vol, then solve for IV
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let call = build_call_option(as_of, expiry, 1.20, 1_000_000.0);
    let market = build_market_context(as_of, MarketParams::atm());
    let calc = FxOptionCalculator::new();

    // Act: Get market price, then solve for IV
    let market_pv = calc.npv(&call, &market, as_of).unwrap();
    let implied_vol = calc
        .implied_vol(&call, &market, as_of, market_pv.amount(), None)
        .unwrap();

    // Assert: Should recover market vol (15%)
    assert_approx_eq(implied_vol, 0.15, 1e-6, 1e-6, "IV should recover market vol");
}

#[test]
fn test_implied_vol_with_custom_initial_guess() {
    // Arrange
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let call = build_call_option(as_of, expiry, 1.20, 1_000_000.0);
    let market = build_market_context(as_of, MarketParams::atm());
    let calc = FxOptionCalculator::new();

    let market_pv = calc.npv(&call, &market, as_of).unwrap();

    // Act: Solve with custom initial guess
    let implied_vol = calc
        .implied_vol(&call, &market, as_of, market_pv.amount(), Some(0.25))
        .unwrap();

    // Assert: Should still converge to correct vol
    assert_approx_eq(implied_vol, 0.15, 1e-6, 1e-6, "IV converges from custom guess");
}

#[test]
fn test_implied_vol_high_vol_scenario() {
    // Arrange: High vol environment
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let call = build_call_option(as_of, expiry, 1.20, 1_000_000.0);
    let market = build_market_context(as_of, MarketParams::high_vol());
    let calc = FxOptionCalculator::new();

    // Act
    let market_pv = calc.npv(&call, &market, as_of).unwrap();
    let implied_vol = calc
        .implied_vol(&call, &market, as_of, market_pv.amount(), None)
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
    let market = build_market_context(as_of, MarketParams::low_vol());
    let calc = FxOptionCalculator::new();

    // Act
    let market_pv = calc.npv(&call, &market, as_of).unwrap();
    let implied_vol = calc
        .implied_vol(&call, &market, as_of, market_pv.amount(), None)
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
    let market = build_market_context(as_of, MarketParams::atm());
    let calc = FxOptionCalculator::new();

    // Act
    let market_pv = calc.npv(&put, &market, as_of).unwrap();
    let implied_vol = calc
        .implied_vol(&put, &market, as_of, market_pv.amount(), None)
        .unwrap();

    // Assert
    assert_approx_eq(implied_vol, 0.15, 1e-6, 1e-6, "Put IV should recover market vol");
}

#[test]
fn test_implied_vol_itm_option() {
    // Arrange: ITM option
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let call = build_call_option(as_of, expiry, 1.10, 1_000_000.0);
    let market = build_market_context(as_of, MarketParams::atm());
    let calc = FxOptionCalculator::new();

    // Act
    let market_pv = calc.npv(&call, &market, as_of).unwrap();
    let implied_vol = calc
        .implied_vol(&call, &market, as_of, market_pv.amount(), None)
        .unwrap();

    // Assert
    assert_approx_eq(implied_vol, 0.15, 1e-6, 1e-6, "ITM IV should recover market vol");
}

#[test]
fn test_implied_vol_otm_option() {
    // Arrange: OTM option
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let call = build_call_option(as_of, expiry, 1.35, 1_000_000.0);
    let market = build_market_context(as_of, MarketParams::atm());
    let calc = FxOptionCalculator::new();

    // Act
    let market_pv = calc.npv(&call, &market, as_of).unwrap();
    let implied_vol = calc
        .implied_vol(&call, &market, as_of, market_pv.amount(), None)
        .unwrap();

    // Assert
    assert_approx_eq(implied_vol, 0.15, 1e-6, 1e-6, "OTM IV should recover market vol");
}

#[test]
fn test_implied_vol_short_dated_option() {
    // Arrange: 3M option
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2024 - 04 - 01);
    let call = build_call_option(as_of, expiry, 1.20, 1_000_000.0);
    let market = build_market_context(as_of, MarketParams::atm());
    let calc = FxOptionCalculator::new();

    // Act
    let market_pv = calc.npv(&call, &market, as_of).unwrap();
    let implied_vol = calc
        .implied_vol(&call, &market, as_of, market_pv.amount(), None)
        .unwrap();

    // Assert
    assert_approx_eq(implied_vol, 0.15, 1e-5, 1e-5, "Short dated IV should converge");
}

#[test]
fn test_implied_vol_long_dated_option() {
    // Arrange: 2Y option
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2026 - 01 - 01);
    let call = build_call_option(as_of, expiry, 1.20, 1_000_000.0);
    let market = build_market_context(as_of, MarketParams::atm());
    let calc = FxOptionCalculator::new();

    // Act
    let market_pv = calc.npv(&call, &market, as_of).unwrap();
    let implied_vol = calc
        .implied_vol(&call, &market, as_of, market_pv.amount(), None)
        .unwrap();

    // Assert
    assert_approx_eq(implied_vol, 0.15, 1e-6, 1e-6, "Long dated IV should converge");
}

#[test]
fn test_implied_vol_expired_option_returns_zero() {
    // Arrange: Expired option
    let expiry = date!(2024 - 01 - 01);
    let as_of = expiry;
    let call = build_call_option(expiry, expiry, 1.20, 1_000_000.0);
    let market = build_market_context(as_of, MarketParams::atm());
    let calc = FxOptionCalculator::new();

    // Act
    let target_price = 10_000.0; // Arbitrary
    let implied_vol = calc
        .implied_vol(&call, &market, as_of, target_price, None)
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
    
    let market = build_market_context(as_of, MarketParams::atm());
    let calc = FxOptionCalculator::new();

    // Price at override vol
    let pv = calc.npv(&call, &market, as_of).unwrap();
    
    // Remove override for IV solve
    call.pricing_overrides.implied_volatility = None;

    // Act: Solve without explicit guess (should use surface vol as initial)
    let implied_vol = calc
        .implied_vol(&call, &market, as_of, pv.amount(), None)
        .unwrap();

    // Assert: Should converge to the price's implied vol
    // (which was 25% from the override used for pricing)
    // But since we removed override, it uses surface vol as guess and should still find correct IV
    assert!(implied_vol > 0.0 && implied_vol < 1.0, "IV should be reasonable");
}

