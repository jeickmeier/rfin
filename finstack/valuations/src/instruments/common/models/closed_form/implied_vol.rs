//! Implied volatility solvers for vanilla option models.
//!
//! Provides robust, monotonic bisection-based solvers for implied volatility under:
//! - Black–Scholes / Garman–Kohlhagen (spot-based, with `r` and `q`)
//! - Black-76 (forward-based, with discount factor `df`)
//!
//! These are intended as shared utilities used by instrument-specific `implied_vol` methods
//! (e.g., equity and FX options) to avoid duplicated solvers and inconsistent edge handling.

use crate::instruments::common_impl::models::closed_form::vanilla::bs_price;
use crate::instruments::common_impl::parameters::OptionType;

/// Minimum volatility (annualized) used for bracketing.
const MIN_VOL: f64 = 1e-8;
/// Maximum volatility (annualized) allowed during bracketing.
const MAX_VOL: f64 = 10.0;
/// Default absolute tolerance on price during solve (per-unit price).
const PRICE_TOL: f64 = 1e-10;
/// Default maximum bisection iterations.
const MAX_ITER: usize = 200;

/// Solve for Black–Scholes / Garman–Kohlhagen implied volatility.
///
/// Finds \(\sigma\) such that `bs_price(spot, strike, r, q, sigma, t, option_type) == target_price`.
///
/// - `target_price` is the **per-unit** option price (not contract-scaled).
/// - Returns `0.0` when `t <= 0`, `target_price <= 0`, or when the target cannot be bracketed.
#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn bs_implied_vol(
    spot: f64,
    strike: f64,
    r: f64,
    q: f64,
    t: f64,
    option_type: OptionType,
    target_price: f64,
) -> f64 {
    if !spot.is_finite() || !strike.is_finite() || !r.is_finite() || !q.is_finite() {
        return 0.0;
    }
    if t <= 0.0 || target_price <= 0.0 || spot <= 0.0 || strike <= 0.0 {
        return 0.0;
    }

    // Intrinsic lower bound (per unit) for continuous compounding.
    let intrinsic = match option_type {
        OptionType::Call => (spot * (-q * t).exp() - strike * (-r * t).exp()).max(0.0),
        OptionType::Put => (strike * (-r * t).exp() - spot * (-q * t).exp()).max(0.0),
    };
    if target_price <= intrinsic {
        return 0.0;
    }

    let price_at = |sigma: f64| -> f64 { bs_price(spot, strike, r, q, sigma, t, option_type) };

    // Bracket the solution (monotone increasing in sigma for vanilla options).
    let mut lo = MIN_VOL;
    let mut hi = 0.3_f64.max(MIN_VOL);
    let f_lo = price_at(lo) - target_price;
    let mut f_hi = price_at(hi) - target_price;

    // Expand hi until we cross the target, or give up.
    let mut tries = 0usize;
    while f_hi < 0.0 && hi < MAX_VOL && tries < 50 {
        hi = (hi * 1.5).min(MAX_VOL);
        f_hi = price_at(hi) - target_price;
        tries += 1;
    }
    // If we still can't reach the target, return 0 (unbracketed).
    if f_hi < 0.0 || !f_hi.is_finite() {
        return 0.0;
    }
    // If lower end is already above the target (shouldn't happen unless target < intrinsic),
    // return 0 for safety.
    if f_lo > 0.0 || !f_lo.is_finite() {
        return 0.0;
    }

    // Bisection.
    let mut mid = 0.5 * (lo + hi);
    for _ in 0..MAX_ITER {
        mid = 0.5 * (lo + hi);
        let f_mid = price_at(mid) - target_price;
        if !f_mid.is_finite() {
            return 0.0;
        }
        if f_mid.abs() < PRICE_TOL || (hi - lo) < 1e-12 {
            return mid;
        }
        if f_mid > 0.0 {
            hi = mid;
        } else {
            lo = mid;
        }
    }

    mid
}

/// Solve for Black-76 implied volatility (forward-based).
///
/// Finds \(\sigma\) such that:
/// `df * bs_price(forward, strike, 0, 0, sigma, t, option_type) == target_price`.
///
/// - `target_price` is the **per-unit** option price (not contract-scaled).
/// - Returns `0.0` when `t <= 0`, `target_price <= 0`, or when the target cannot be bracketed.
#[must_use]
pub fn black76_implied_vol(
    forward: f64,
    strike: f64,
    df: f64,
    t: f64,
    option_type: OptionType,
    target_price: f64,
) -> f64 {
    if !forward.is_finite() || !strike.is_finite() || !df.is_finite() {
        return 0.0;
    }
    if t <= 0.0 || target_price <= 0.0 || forward <= 0.0 || strike <= 0.0 || df <= 0.0 {
        return 0.0;
    }

    let intrinsic = match option_type {
        OptionType::Call => (forward - strike).max(0.0) * df,
        OptionType::Put => (strike - forward).max(0.0) * df,
    };
    if target_price <= intrinsic {
        return 0.0;
    }

    let price_at =
        |sigma: f64| -> f64 { df * bs_price(forward, strike, 0.0, 0.0, sigma, t, option_type) };

    let mut lo = MIN_VOL;
    let mut hi = 0.3_f64.max(MIN_VOL);
    let f_lo = price_at(lo) - target_price;
    let mut f_hi = price_at(hi) - target_price;

    let mut tries = 0usize;
    while f_hi < 0.0 && hi < MAX_VOL && tries < 50 {
        hi = (hi * 1.5).min(MAX_VOL);
        f_hi = price_at(hi) - target_price;
        tries += 1;
    }
    if f_hi < 0.0 || !f_hi.is_finite() {
        return 0.0;
    }
    if f_lo > 0.0 || !f_lo.is_finite() {
        return 0.0;
    }

    let mut mid = 0.5 * (lo + hi);
    for _ in 0..MAX_ITER {
        mid = 0.5 * (lo + hi);
        let f_mid = price_at(mid) - target_price;
        if !f_mid.is_finite() {
            return 0.0;
        }
        if f_mid.abs() < PRICE_TOL || (hi - lo) < 1e-12 {
            return mid;
        }
        if f_mid > 0.0 {
            hi = mid;
        } else {
            lo = mid;
        }
    }

    mid
}
