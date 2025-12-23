//! Black/Black–Scholes common math helpers shared across instruments.
//!
//! This module provides the fundamental d1/d2 calculations used throughout
//! option pricing. All functions are inlined for performance in hot paths.
//!
//! # Performance Notes
//!
//! For hot paths that need both d1 and d2, use [`d1_d2`] or [`d1_d2_black76`]
//! to avoid redundant computation. The combined functions compute shared
//! intermediate values (ln, sqrt) only once.

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
///
/// # Performance
/// If you need both d1 and d2, use [`d1_d2`] instead to avoid redundant work.
#[inline]
#[must_use]
pub fn d1(spot: f64, strike: f64, r: f64, sigma: f64, t: f64, q: f64) -> f64 {
    d1_d2(spot, strike, r, sigma, t, q).0
}

/// Calculate d2 for Black–Scholes (general form with dividends/carry)
///
/// d2 = d1 - σ√T
///
/// # Performance
/// If you need both d1 and d2, use [`d1_d2`] instead to avoid redundant work.
#[inline]
#[must_use]
pub fn d2(spot: f64, strike: f64, r: f64, sigma: f64, t: f64, q: f64) -> f64 {
    d1_d2(spot, strike, r, sigma, t, q).1
}

/// Calculate both d1 and d2 for Black–Scholes in a single pass.
///
/// This is the preferred function for hot paths (e.g., Greeks calculations)
/// where both values are needed. Computes shared intermediate values only once.
///
/// # Returns
/// Tuple of (d1, d2)
///
/// # Example
/// ```rust
/// use finstack_valuations::instruments::common::models::volatility::black::d1_d2;
///
/// let (d1, d2) = d1_d2(100.0, 100.0, 0.05, 0.20, 1.0, 0.02);
/// assert!(d1 > d2); // d1 is always >= d2
/// ```
#[inline]
#[must_use]
pub fn d1_d2(spot: f64, strike: f64, r: f64, sigma: f64, t: f64, q: f64) -> (f64, f64) {
    // Handle edge cases with proper limiting behavior
    if t <= 0.0 || sigma <= 0.0 {
        // At expiration or zero vol: d1/d2 → ±∞ based on moneyness
        // This ensures correct delta behavior (0 or 1 for OTM/ITM)
        let intrinsic_sign = (spot - strike).signum();
        let limit = if intrinsic_sign > 0.0 {
            f64::INFINITY // ITM call → delta = 1
        } else if intrinsic_sign < 0.0 {
            f64::NEG_INFINITY // OTM call → delta = 0
        } else {
            0.0 // ATM → delta = 0.5
        };
        return (limit, limit);
    }

    // Compute shared intermediate values once
    let sqrt_t = t.sqrt();
    let sigma_sqrt_t = sigma * sqrt_t;
    let d1 = ((spot / strike).ln() + (r - q + 0.5 * sigma * sigma) * t) / sigma_sqrt_t;
    let d2 = d1 - sigma_sqrt_t;
    (d1, d2)
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
///
/// # Performance
/// If you need both d1 and d2, use [`d1_d2_black76`] instead.
#[inline]
#[must_use]
pub fn d1_black76(forward: f64, strike: f64, sigma: f64, t: f64) -> f64 {
    d1_d2_black76(forward, strike, sigma, t).0
}

/// Calculate d2 for Black76 model
///
/// d2 = d1 - σ√T
///
/// # Performance
/// If you need both d1 and d2, use [`d1_d2_black76`] instead.
#[inline]
#[must_use]
pub fn d2_black76(forward: f64, strike: f64, sigma: f64, t: f64) -> f64 {
    d1_d2_black76(forward, strike, sigma, t).1
}

/// Calculate both d1 and d2 for Black76 model in a single pass.
///
/// This is the preferred function for hot paths where both values are needed.
///
/// # Returns
/// Tuple of (d1, d2)
///
/// # Example
/// ```rust
/// use finstack_valuations::instruments::common::models::volatility::black::d1_d2_black76;
///
/// let (d1, d2) = d1_d2_black76(0.05, 0.05, 0.20, 1.0);
/// assert!(d1 > d2); // d1 is always >= d2
/// ```
#[inline]
#[must_use]
pub fn d1_d2_black76(forward: f64, strike: f64, sigma: f64, t: f64) -> (f64, f64) {
    if t <= 0.0 || sigma <= 0.0 {
        let intrinsic_sign = (forward - strike).signum();
        let limit = if intrinsic_sign > 0.0 {
            f64::INFINITY
        } else if intrinsic_sign < 0.0 {
            f64::NEG_INFINITY
        } else {
            0.0
        };
        return (limit, limit);
    }

    // Compute shared intermediate values once
    let sqrt_t = t.sqrt();
    let sigma_sqrt_t = sigma * sqrt_t;
    let variance = sigma_sqrt_t * sigma_sqrt_t; // sigma^2 * t
    let d1 = ((forward / strike).ln() + 0.5 * variance) / sigma_sqrt_t;
    let d2 = d1 - sigma_sqrt_t;
    (d1, d2)
}
