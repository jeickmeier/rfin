//! Comprehensive tests for Black-Scholes and Black76 formulas.
//!
//! Tests cover:
//! - d1/d2 calculations
//! - Black-Scholes pricing
//! - Black76 (forward-based) pricing
//! - Edge cases and numerical stability
//! - Put-call parity

use finstack_core::math::norm_cdf;
use finstack_valuations::instruments::common::models::{d1, d1_black76, d2, d2_black76};

use super::super::test_helpers::*;

// ============================================================================
// d1/d2 Calculation Tests
// ============================================================================

#[test]
fn test_d1_atm_option() {
    // Arrange: At-the-money option
    let spot = 100.0;
    let strike = 100.0;
    let r = 0.05;
    let sigma = 0.20;
    let t = 1.0;
    let q = 0.0;

    // Act
    let d1_value = d1(spot, strike, r, sigma, t, q);

    // Assert: ATM d1 should be positive due to drift
    assert!(d1_value > 0.0, "ATM d1 is positive");
    assert_approx_eq(d1_value, 0.35, 0.05, "ATM d1 value");
}

#[test]
fn test_d2_equals_d1_minus_vol_sqrt_t() {
    // Arrange
    let spot = 100.0;
    let strike = 100.0;
    let r = 0.05;
    let sigma = 0.20;
    let t = 1.0;
    let q = 0.0;

    // Act
    let d1_value = d1(spot, strike, r, sigma, t, q);
    let d2_value = d2(spot, strike, r, sigma, t, q);

    // Assert: d2 = d1 - σ√T
    let expected_d2 = d1_value - sigma * t.sqrt();
    assert_approx_eq(d2_value, expected_d2, TIGHT_TOLERANCE, "d2 = d1 - σ√T");
}

#[test]
fn test_d1_itm_call() {
    // Arrange: In-the-money call
    let spot = 110.0;
    let strike = 100.0;
    let r = 0.05;
    let sigma = 0.20;
    let t = 1.0;
    let q = 0.0;

    // Act
    let d1_value = d1(spot, strike, r, sigma, t, q);

    // Assert: ITM call has higher d1
    assert!(d1_value > 0.5, "ITM call has high d1");
}

#[test]
fn test_d1_otm_call() {
    // Arrange: Out-of-the-money call
    let spot = 90.0;
    let strike = 100.0;
    let r = 0.05;
    let sigma = 0.20;
    let t = 1.0;
    let q = 0.0;

    // Act
    let d1_value = d1(spot, strike, r, sigma, t, q);

    // Assert: OTM call has lower d1
    assert!(d1_value < 0.0, "OTM call has negative d1");
}

#[test]
fn test_d1_with_dividend_yield() {
    // Arrange
    let spot = 100.0;
    let strike = 100.0;
    let r = 0.05;
    let sigma = 0.20;
    let t = 1.0;

    // Act: Compare with and without dividends
    let d1_no_div = d1(spot, strike, r, sigma, t, 0.0);
    let d1_with_div = d1(spot, strike, r, sigma, t, 0.02);

    // Assert: Dividends reduce d1 (reduce call value)
    assert!(d1_with_div < d1_no_div, "Dividends reduce d1");
}

#[test]
fn test_d1_zero_time() {
    // Arrange: Zero time to expiry
    let spot = 105.0;
    let strike = 100.0;

    // Act
    let d1_value = d1(spot, strike, 0.05, 0.20, 0.0, 0.0);

    // Assert: Returns 0 for zero time (handled gracefully)
    assert_approx_eq(d1_value, 0.0, TIGHT_TOLERANCE, "d1 = 0 for t=0");
}

#[test]
fn test_d1_zero_volatility() {
    // Arrange: Zero volatility
    let spot = 105.0;
    let strike = 100.0;

    // Act
    let d1_value = d1(spot, strike, 0.05, 0.0, 1.0, 0.0);

    // Assert: Returns 0 for zero vol (handled gracefully)
    assert_approx_eq(d1_value, 0.0, TIGHT_TOLERANCE, "d1 = 0 for σ=0");
}

// ============================================================================
// Black76 d1/d2 Tests
// ============================================================================

#[test]
fn test_black76_d1_atm() {
    // Arrange: ATM forward option
    let forward = 100.0;
    let strike = 100.0;
    let sigma = 0.20;
    let t = 1.0;

    // Act
    let d1_value = d1_black76(forward, strike, sigma, t);

    // Assert: ATM with no drift should be σ√T/2
    let expected = 0.5 * sigma * t.sqrt();
    assert_approx_eq(d1_value, expected, 0.01, "Black76 ATM d1");
}

#[test]
fn test_black76_d2_relationship() {
    // Arrange
    let forward = 100.0;
    let strike = 100.0;
    let sigma = 0.20;
    let t = 1.0;

    // Act
    let d1_value = d1_black76(forward, strike, sigma, t);
    let d2_value = d2_black76(forward, strike, sigma, t);

    // Assert
    let expected_d2 = d1_value - sigma * t.sqrt();
    assert_approx_eq(
        d2_value,
        expected_d2,
        TIGHT_TOLERANCE,
        "Black76 d2 = d1 - σ√T",
    );
}

#[test]
fn test_black76_vs_black_scholes_equivalence() {
    // Arrange: Black76 with r=q=0 should match BS
    let spot = 100.0;
    let strike = 100.0;
    let sigma = 0.20;
    let t = 1.0;

    // Act
    let bs_d1 = d1(spot, strike, 0.0, sigma, t, 0.0);
    let black76_d1 = d1_black76(spot, strike, sigma, t);

    // Assert
    assert_approx_eq(
        bs_d1,
        black76_d1,
        TIGHT_TOLERANCE,
        "Black76 ~ BS when r=q=0",
    );
}

#[test]
fn test_black76_zero_time() {
    // Arrange
    let forward = 105.0;
    let strike = 100.0;

    // Act
    let d1_value = d1_black76(forward, strike, 0.20, 0.0);

    // Assert
    assert_approx_eq(d1_value, 0.0, TIGHT_TOLERANCE, "d1 = 0 for t=0");
}

// ============================================================================
// Black-Scholes Formula Tests (using helper function)
// ============================================================================

#[test]
fn test_black_scholes_call_atm() {
    // Arrange
    let spot = 100.0;
    let strike = 100.0;
    let rate = 0.05;
    let vol = 0.20;
    let time = 1.0;

    // Act
    let call_value = black_scholes_call(spot, strike, rate, vol, time, 0.0);

    // Assert: ATM call should be around 10% of spot
    assert!(call_value > 8.0 && call_value < 12.0, "ATM call ~ 10");
}

#[test]
fn test_black_scholes_put_atm() {
    // Arrange
    let spot = 100.0;
    let strike = 100.0;
    let rate = 0.05;
    let vol = 0.20;
    let time = 1.0;

    // Act
    let put_value = black_scholes_put(spot, strike, rate, vol, time, 0.0);

    // Assert: ATM put should be slightly less than call due to drift
    let call_value = black_scholes_call(spot, strike, rate, vol, time, 0.0);
    assert!(put_value < call_value, "ATM put < ATM call");
    assert!(put_value > 5.0, "ATM put has value");
}

#[test]
fn test_black_scholes_put_call_parity() {
    // Arrange
    let spot = 100.0;
    let strike = 100.0;
    let rate = 0.05;
    let vol = 0.20;
    let time = 1.0;
    let div = 0.02;

    // Act
    let call = black_scholes_call(spot, strike, rate, vol, time, div);
    let put = black_scholes_put(spot, strike, rate, vol, time, div);

    // Assert: C - P = S*e^(-qT) - K*e^(-rT)
    let lhs = call - put;
    let rhs = spot * (-div * time).exp() as f64 - strike * (-rate * time).exp() as f64;

    assert_approx_eq(lhs, rhs, TIGHT_TOLERANCE, "Put-call parity");
}

#[test]
fn test_black_scholes_call_itm() {
    // Arrange: Deep ITM
    let spot = 120.0;
    let strike = 100.0;
    let rate = 0.05;
    let vol = 0.20;
    let time = 1.0;

    // Act
    let call_value = black_scholes_call(spot, strike, rate, vol, time, 0.0);

    // Assert: Should be close to intrinsic value
    let intrinsic = spot - strike;
    assert!(call_value >= intrinsic, "Call >= intrinsic");
    assert!(call_value < spot, "Call < spot");
}

#[test]
fn test_black_scholes_call_otm() {
    // Arrange: Out-of-the-money
    let spot = 90.0;
    let strike = 100.0;
    let rate = 0.05;
    let vol = 0.20;
    let time = 1.0;

    // Act
    let call_value = black_scholes_call(spot, strike, rate, vol, time, 0.0);

    // Assert: Should be small but positive
    assert!(call_value > 0.0, "OTM call has time value");
    assert!(call_value < 5.0, "OTM call has low value");
}

#[test]
fn test_black_scholes_boundary_conditions() {
    // Arrange
    let spot = 100.0;
    let strike = 100.0;
    let rate = 0.05;
    let vol = 0.20;

    // Act & Assert: Zero time to expiry
    let call_zero_time = black_scholes_call(spot, strike, rate, vol, 0.0, 0.0);
    assert_approx_eq(
        call_zero_time,
        0.0,
        TIGHT_TOLERANCE,
        "Call = 0 at expiry (ATM)",
    );

    // Act & Assert: ITM at expiry
    let call_itm_expiry = black_scholes_call(110.0, 100.0, rate, vol, 0.0, 0.0);
    assert_approx_eq(
        call_itm_expiry,
        10.0,
        TIGHT_TOLERANCE,
        "Call = intrinsic at expiry",
    );
}

#[test]
fn test_black_scholes_monotonicity() {
    // Arrange: Base case
    let spot = 100.0;
    let strike = 100.0;
    let rate = 0.05;
    let vol = 0.20;
    let time = 1.0;

    let base_call = black_scholes_call(spot, strike, rate, vol, time, 0.0);

    // Act & Assert: Higher spot increases call value
    let higher_spot = black_scholes_call(105.0, strike, rate, vol, time, 0.0);
    assert!(higher_spot > base_call, "Call increases with spot");

    // Act & Assert: Higher volatility increases call value
    let higher_vol = black_scholes_call(spot, strike, rate, 0.30, time, 0.0);
    assert!(higher_vol > base_call, "Call increases with vol");

    // Act & Assert: Longer time increases call value
    let longer_time = black_scholes_call(spot, strike, rate, vol, 2.0, 0.0);
    assert!(longer_time > base_call, "Call increases with time");

    // Act & Assert: Higher rate increases call value
    let higher_rate = black_scholes_call(spot, strike, 0.10, vol, time, 0.0);
    assert!(higher_rate > base_call, "Call increases with rate");
}

#[test]
fn test_black_scholes_symmetry() {
    // Arrange: Test put-call symmetry for specific case
    let spot = 100.0;
    let strike = 100.0;
    let rate = 0.05;
    let vol = 0.20;
    let time = 1.0;

    // Act
    let call = black_scholes_call(spot, strike, rate, vol, time, 0.0);
    let put = black_scholes_put(spot, strike, rate, vol, time, 0.0);

    // Assert: For ATM with r>0, call > put
    assert!(call > put, "ATM call > put when r > 0");
}

// ============================================================================
// Numerical Stability Tests
// ============================================================================

#[test]
fn test_extreme_spot_values() {
    // Arrange: Very high and very low spot
    let strike = 100.0;
    let rate = 0.05;
    let vol = 0.20;
    let time = 1.0;

    // Act: Very high spot (deep ITM call)
    let high_spot_call = black_scholes_call(10000.0, strike, rate, vol, time, 0.0);
    assert!(high_spot_call.is_finite(), "Finite for high spot");
    assert!(high_spot_call > 9000.0, "Deep ITM call ~ intrinsic");

    // Act: Very low spot (deep OTM call)
    let low_spot_call = black_scholes_call(1.0, strike, rate, vol, time, 0.0);
    assert!(low_spot_call.is_finite(), "Finite for low spot");
    assert!(low_spot_call < 0.01, "Deep OTM call ~ 0");
}

#[test]
fn test_extreme_volatility_values() {
    // Arrange
    let spot = 100.0;
    let strike = 100.0;
    let rate = 0.05;
    let time = 1.0;

    // Act: Very high volatility
    let high_vol_call = black_scholes_call(spot, strike, rate, 2.0, time, 0.0);
    assert!(high_vol_call.is_finite(), "Finite for high vol");
    assert!(high_vol_call > 50.0, "High vol creates high value");

    // Act: Low volatility
    let low_vol_call = black_scholes_call(spot, strike, rate, 0.01, time, 0.0);
    assert!(low_vol_call.is_finite(), "Finite for low vol");
}

#[test]
fn test_extreme_time_values() {
    // Arrange
    let spot = 100.0;
    let strike = 100.0;
    let rate = 0.05;
    let vol = 0.20;

    // Act: Very long time
    let long_time_call = black_scholes_call(spot, strike, rate, vol, 30.0, 0.0);
    assert!(long_time_call.is_finite(), "Finite for long time");
    assert!(long_time_call < spot, "Long time call < spot");

    // Act: Very short time
    let short_time_call = black_scholes_call(spot, strike, rate, vol, 0.001, 0.0);
    assert!(short_time_call.is_finite(), "Finite for short time");
}

#[test]
fn test_negative_rates() {
    // Arrange: Negative interest rates
    let spot = 100.0;
    let strike = 100.0;
    let rate = -0.01;
    let vol = 0.20;
    let time = 1.0;

    // Act
    let call = black_scholes_call(spot, strike, rate, vol, time, 0.0);
    let put = black_scholes_put(spot, strike, rate, vol, time, 0.0);

    // Assert: Should still be valid
    assert!(
        call.is_finite() && call > 0.0,
        "Call valid with negative rates"
    );
    assert!(
        put.is_finite() && put > 0.0,
        "Put valid with negative rates"
    );

    // Put-call parity should still hold
    let lhs = call - put;
    let rhs = spot - strike * (-rate * time).exp() as f64;
    assert_approx_eq(lhs, rhs, TOLERANCE, "Parity holds with negative rates");
}
