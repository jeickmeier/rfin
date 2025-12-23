//! Black/Black–Scholes common math helpers shared across instruments.
//!
//! This module provides the fundamental d1/d2 calculations used throughout
//! option pricing. All functions are inlined for performance in hot paths.

/// Calculate d1 for Black–Scholes (general form with dividends/carry)
///
/// Full Black-Scholes-Merton d1 calculation:
/// d1 = [ln(S/K) + (r - q + σ²/2)T] / (σ√T)
///
/// # Parameters
/// - `spot`: Spot price (or forward for Black76 when r=q=0)
/// - `strike`: Strike price
/// - `r`: Risk-free rate (or 0 for Black76)
/// - `sigma`: Volatility
/// - `t`: Time to expiry in years
/// - `q`: Dividend yield / cost of carry (or 0 for Black76)
///
/// # Edge Cases
/// - At expiration (t ≤ 0) or zero volatility: returns appropriate limit
///   based on intrinsic value (±∞ for ITM/OTM, 0 for ATM)
#[inline]
#[must_use]
pub fn d1(spot: f64, strike: f64, r: f64, sigma: f64, t: f64, q: f64) -> f64 {
    // Handle edge cases with proper limiting behavior
    if t <= 0.0 || sigma <= 0.0 {
        // At expiration or zero vol: d1 → ±∞ based on moneyness
        // This ensures correct delta behavior (0 or 1 for OTM/ITM)
        let intrinsic_sign = (spot - strike).signum();
        if intrinsic_sign > 0.0 {
            return f64::INFINITY; // ITM call → delta = 1
        } else if intrinsic_sign < 0.0 {
            return f64::NEG_INFINITY; // OTM call → delta = 0
        } else {
            return 0.0; // ATM → delta = 0.5
        }
    }
    ((spot / strike).ln() + (r - q + 0.5 * sigma * sigma) * t) / (sigma * t.sqrt())
}

/// Calculate d2 for Black–Scholes (general form with dividends/carry)
///
/// d2 = d1 - σ√T
#[inline]
#[must_use]
pub fn d2(spot: f64, strike: f64, r: f64, sigma: f64, t: f64, q: f64) -> f64 {
    // Handle edge cases consistently with d1
    if t <= 0.0 || sigma <= 0.0 {
        return d1(spot, strike, r, sigma, t, q); // Same limit applies
    }
    d1(spot, strike, r, sigma, t, q) - sigma * t.sqrt()
}

/// Calculate d1 for Black76 model (forward-based, no drift)
///
/// Black76 d1 calculation for rates/commodities:
/// d1 = [ln(F/K) + σ²T/2] / (σ√T)
///
/// This is equivalent to Black-Scholes with r=q=0.
/// Used for swaptions, caps/floors, and commodity options.
///
/// # Parameters
/// - `forward`: Forward price/rate
/// - `strike`: Strike price/rate
/// - `sigma`: Volatility
/// - `t`: Time to expiry in years
///
/// # Edge Cases
/// - At expiration (t ≤ 0) or zero volatility: returns appropriate limit
#[inline]
#[must_use]
pub fn d1_black76(forward: f64, strike: f64, sigma: f64, t: f64) -> f64 {
    if t <= 0.0 || sigma <= 0.0 {
        let intrinsic_sign = (forward - strike).signum();
        if intrinsic_sign > 0.0 {
            return f64::INFINITY;
        } else if intrinsic_sign < 0.0 {
            return f64::NEG_INFINITY;
        } else {
            return 0.0;
        }
    }
    let variance = sigma * sigma * t;
    ((forward / strike).ln() + 0.5 * variance) / variance.sqrt()
}

/// Calculate d2 for Black76 model
///
/// d2 = d1 - σ√T
#[inline]
#[must_use]
pub fn d2_black76(forward: f64, strike: f64, sigma: f64, t: f64) -> f64 {
    if t <= 0.0 || sigma <= 0.0 {
        return d1_black76(forward, strike, sigma, t);
    }
    d1_black76(forward, strike, sigma, t) - sigma * t.sqrt()
}
