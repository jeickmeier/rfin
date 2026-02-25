//! Implied volatility solvers for vanilla option models.
//!
//! Provides robust, monotonic bisection-based solvers for implied volatility under:
//! - Black–Scholes / Garman–Kohlhagen (spot-based, with `r` and `q`)
//! - Black-76 (forward-based, with discount factor `df`)
//!
//! These are intended as shared utilities used by instrument-specific `implied_vol` methods
//! (e.g., equity and FX options) to avoid duplicated solvers and inconsistent edge handling.

use finstack_core::Result;

use crate::instruments::common_impl::models::closed_form::vanilla::bs_price;
use crate::instruments::common_impl::parameters::OptionType;

/// Error returned when implied volatility cannot be bracketed (target price may exceed arbitrage bounds).
const UNBRACKETED_MSG: &str =
    "Cannot bracket implied volatility: price may exceed arbitrage bounds";

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
/// - Returns `Ok(0.0)` when `t <= 0` or `target_price <= 0` (expired or degenerate).
/// - Returns `Err` when the target cannot be bracketed (price may exceed arbitrage bounds).
#[allow(clippy::too_many_arguments)]
pub fn bs_implied_vol(
    spot: f64,
    strike: f64,
    r: f64,
    q: f64,
    t: f64,
    option_type: OptionType,
    target_price: f64,
) -> Result<f64> {
    if !spot.is_finite() || !strike.is_finite() || !r.is_finite() || !q.is_finite() {
        return Ok(0.0);
    }
    if t <= 0.0 || target_price <= 0.0 || spot <= 0.0 || strike <= 0.0 {
        return Ok(0.0);
    }

    // Intrinsic lower bound (per unit) for continuous compounding.
    let intrinsic = match option_type {
        OptionType::Call => (spot * (-q * t).exp() - strike * (-r * t).exp()).max(0.0),
        OptionType::Put => (strike * (-r * t).exp() - spot * (-q * t).exp()).max(0.0),
    };
    if target_price <= intrinsic {
        return Err(finstack_core::Error::Validation(UNBRACKETED_MSG.into()));
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
    if f_hi < 0.0 || !f_hi.is_finite() {
        return Err(finstack_core::Error::Validation(UNBRACKETED_MSG.into()));
    }
    if f_lo > 0.0 || !f_lo.is_finite() {
        return Err(finstack_core::Error::Validation(UNBRACKETED_MSG.into()));
    }

    // Bisection.
    let mut mid = 0.5 * (lo + hi);
    for _ in 0..MAX_ITER {
        mid = 0.5 * (lo + hi);
        let f_mid = price_at(mid) - target_price;
        if !f_mid.is_finite() {
            return Err(finstack_core::Error::Validation(UNBRACKETED_MSG.into()));
        }
        if f_mid.abs() < PRICE_TOL || (hi - lo) < 1e-12 {
            return Ok(mid);
        }
        if f_mid > 0.0 {
            hi = mid;
        } else {
            lo = mid;
        }
    }

    Ok(mid)
}

/// Solve for Black-76 implied volatility (forward-based).
///
/// Finds \(\sigma\) such that:
/// `df * bs_price(forward, strike, 0, 0, sigma, t, option_type) == target_price`.
///
/// - `target_price` is the **per-unit** option price (not contract-scaled).
/// - Returns `Ok(0.0)` when `t <= 0` or `target_price <= 0` (expired or degenerate).
/// - Returns `Err` when the target cannot be bracketed (price may exceed arbitrage bounds).
pub fn black76_implied_vol(
    forward: f64,
    strike: f64,
    df: f64,
    t: f64,
    option_type: OptionType,
    target_price: f64,
) -> Result<f64> {
    if !forward.is_finite() || !strike.is_finite() || !df.is_finite() {
        return Ok(0.0);
    }
    if t <= 0.0 || target_price <= 0.0 || forward <= 0.0 || strike <= 0.0 || df <= 0.0 {
        return Ok(0.0);
    }

    let intrinsic = match option_type {
        OptionType::Call => (forward - strike).max(0.0) * df,
        OptionType::Put => (strike - forward).max(0.0) * df,
    };
    if target_price <= intrinsic {
        return Err(finstack_core::Error::Validation(UNBRACKETED_MSG.into()));
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
        return Err(finstack_core::Error::Validation(UNBRACKETED_MSG.into()));
    }
    if f_lo > 0.0 || !f_lo.is_finite() {
        return Err(finstack_core::Error::Validation(UNBRACKETED_MSG.into()));
    }

    let mut mid = 0.5 * (lo + hi);
    for _ in 0..MAX_ITER {
        mid = 0.5 * (lo + hi);
        let f_mid = price_at(mid) - target_price;
        if !f_mid.is_finite() {
            return Err(finstack_core::Error::Validation(UNBRACKETED_MSG.into()));
        }
        if f_mid.abs() < PRICE_TOL || (hi - lo) < 1e-12 {
            return Ok(mid);
        }
        if f_mid > 0.0 {
            hi = mid;
        } else {
            lo = mid;
        }
    }

    Ok(mid)
}
