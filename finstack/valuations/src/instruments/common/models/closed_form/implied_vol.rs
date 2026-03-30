//! Implied volatility solvers for vanilla option models.
//!
//! Provides robust, monotonic bisection-based solvers for implied volatility under:
//! - Black–Scholes / Garman–Kohlhagen (spot-based, with `r` and `q`)
//! - Black-76 (forward-based, with discount factor `df`)
//!
//! These are intended as shared utilities used by instrument-specific `implied_vol` methods
//! (e.g., equity and FX options) to avoid duplicated solvers and inconsistent edge handling.

use finstack_core::Result;

use crate::instruments::common_impl::models::closed_form::greeks::bs_vega;
use crate::instruments::common_impl::models::closed_form::vanilla::bs_price;
use crate::instruments::common_impl::parameters::OptionType;

/// Error returned when implied volatility cannot be bracketed (target price may exceed arbitrage bounds).
const UNBRACKETED_MSG: &str =
    "Cannot bracket implied volatility: price may exceed arbitrage bounds";

/// Error returned when implied volatility inputs contain non-finite values.
const NON_FINITE_MSG: &str = "Implied volatility solver received non-finite input parameters";

/// Minimum volatility (annualized) used for bracketing.
const MIN_VOL: f64 = 1e-8;
/// Maximum volatility (annualized) allowed during bracketing.
const MAX_VOL: f64 = 10.0;
/// Default absolute tolerance on price during solve (per-unit price).
const PRICE_TOL: f64 = 1e-10;
/// Default maximum solver iterations.
const MAX_ITER: usize = 200;
/// Maximum Newton-Raphson iterations before falling back to bisection.
const MAX_NEWTON_ITER: usize = 15;
/// Minimum vega for Newton step to be accepted (avoid division by near-zero).
const MIN_VEGA: f64 = 1e-15;

/// Solve for Black–Scholes / Garman–Kohlhagen implied volatility.
///
/// Finds \(\sigma\) such that `bs_price(spot, strike, r, q, sigma, t, option_type) == target_price`.
///
/// - `target_price` is the **per-unit** option price (not contract-scaled).
/// - Returns `Ok(0.0)` when `t <= 0` (expired; volatility is moot).
/// - Returns `Err` for non-finite inputs, non-positive `spot`/`strike`/`target_price`,
///   or when the target cannot be bracketed.
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
    if !spot.is_finite()
        || !strike.is_finite()
        || !r.is_finite()
        || !q.is_finite()
        || !t.is_finite()
        || !target_price.is_finite()
    {
        return Err(finstack_core::Error::Validation(NON_FINITE_MSG.into()));
    }
    if t <= 0.0 {
        return Ok(0.0);
    }
    if target_price <= 0.0 || spot <= 0.0 || strike <= 0.0 {
        return Err(finstack_core::Error::Validation(
            "implied vol requires positive spot, strike, and target_price".into(),
        ));
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

    // Newton-Raphson with bisection fallback.
    // bs_vega returns dPrice/dSigma scaled by 0.01, so multiply by 100 for raw vega.
    let raw_vega_at = |sigma: f64| -> f64 { bs_vega(spot, strike, t, r, q, sigma) * 100.0 };

    let mut mid = 0.5 * (lo + hi);

    // Phase 1: Newton-Raphson (quadratic convergence near root)
    for _ in 0..MAX_NEWTON_ITER {
        let f_mid = price_at(mid) - target_price;
        if f_mid.abs() < PRICE_TOL {
            return Ok(mid);
        }
        let vega = raw_vega_at(mid);
        if vega.abs() < MIN_VEGA {
            break; // vega too small; fall through to bisection
        }
        let step = f_mid / vega;
        let candidate = mid - step;
        if candidate <= lo || candidate >= hi || !candidate.is_finite() {
            break; // Newton step out of bracket; fall through to bisection
        }
        // Update bracket
        if f_mid > 0.0 {
            hi = mid;
        } else {
            lo = mid;
        }
        mid = candidate;
    }

    // Phase 2: Bisection fallback (guaranteed convergence)
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
/// - Returns `Ok(0.0)` when `t <= 0` (expired; volatility is moot).
/// - Returns `Err` for non-finite inputs, non-positive `forward`/`strike`/`df`/`target_price`,
///   or when the target cannot be bracketed.
pub fn black76_implied_vol(
    forward: f64,
    strike: f64,
    df: f64,
    t: f64,
    option_type: OptionType,
    target_price: f64,
) -> Result<f64> {
    if !forward.is_finite()
        || !strike.is_finite()
        || !df.is_finite()
        || !t.is_finite()
        || !target_price.is_finite()
    {
        return Err(finstack_core::Error::Validation(NON_FINITE_MSG.into()));
    }
    if t <= 0.0 {
        return Ok(0.0);
    }
    if target_price <= 0.0 || forward <= 0.0 || strike <= 0.0 || df <= 0.0 {
        return Err(finstack_core::Error::Validation(
            "implied vol requires positive forward, strike, df, and target_price".into(),
        ));
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

    // Newton-Raphson with bisection fallback.
    // For Black-76: vega = df * d(bs_price(F,K,0,0,sigma,t))/d(sigma)
    let raw_vega_at =
        |sigma: f64| -> f64 { df * bs_vega(forward, strike, t, 0.0, 0.0, sigma) * 100.0 };

    let mut mid = 0.5 * (lo + hi);

    for _ in 0..MAX_NEWTON_ITER {
        let f_mid = price_at(mid) - target_price;
        if f_mid.abs() < PRICE_TOL {
            return Ok(mid);
        }
        let vega = raw_vega_at(mid);
        if vega.abs() < MIN_VEGA {
            break;
        }
        let candidate = mid - f_mid / vega;
        if candidate <= lo || candidate >= hi || !candidate.is_finite() {
            break;
        }
        if f_mid > 0.0 {
            hi = mid;
        } else {
            lo = mid;
        }
        mid = candidate;
    }

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
