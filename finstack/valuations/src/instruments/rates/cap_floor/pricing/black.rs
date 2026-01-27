//! Black model helpers for interest rate caplets/floorlets.
//!
//! Exposes pure functions for price and greeks to keep `types.rs` free of pricing logic.
//!
//! # Numerical Hardening
//!
//! This module handles edge cases that can cause numerical issues:
//! - **Zero/negative volatility**: Returns intrinsic value (same as expiry)
//! - **Zero/negative forward**: Returns error (Black model undefined)
//! - **Negative time to fixing**: Returns intrinsic value (option already fixed)
//!
//! # References
//!
//! - Black, F. (1976). "The pricing of commodity contracts."
//!   *Journal of Financial Economics*, 3(1-2), 167-179.

use crate::instruments::common::models::{d1 as bs_d1, d2 as bs_d2};
use finstack_core::currency::Currency;
use finstack_core::math::{norm_cdf, norm_pdf};
use finstack_core::money::Money;

/// Inputs for Black caplet/floorlet pricing
/// Inputs for pricing a single caplet or floorlet using Black (1976) model.
#[derive(Clone, Copy, Debug)]
pub struct CapletFloorletInputs {
    /// True for caplet, false for floorlet
    pub is_cap: bool,
    /// Notional amount
    pub notional: f64,
    /// Strike rate (as decimal)
    pub strike: f64,
    /// Forward rate (as decimal)
    pub forward: f64,
    /// Discount factor to payment date
    pub discount_factor: f64,
    /// Black volatility (annualized)
    pub volatility: f64,
    /// Time to fixing date in years
    pub time_to_fixing: f64,
    /// Accrual year fraction for the period
    pub accrual_year_fraction: f64,
    /// Currency for the cashflow
    pub currency: Currency,
}

/// Compute intrinsic value of a caplet/floorlet.
///
/// This is used when the option is at or past expiry, or when volatility is zero/negative.
#[inline]
fn intrinsic_value(inputs: &CapletFloorletInputs) -> f64 {
    let payoff = if inputs.is_cap {
        (inputs.forward - inputs.strike).max(0.0)
    } else {
        (inputs.strike - inputs.forward).max(0.0)
    };
    payoff * inputs.accrual_year_fraction * inputs.notional * inputs.discount_factor
}

/// Price a caplet/floorlet using Black's formula.
///
/// Returns PV in the instrument currency given forward, discount factor, vol, time to fixing and accrual.
///
/// # Edge Case Handling
///
/// - **`t_fix <= 0`**: Option is at or past fixing; returns intrinsic value.
/// - **`sigma <= 0 || !sigma.is_finite()`**: Volatility is invalid; returns intrinsic value.
/// - **`forward <= 0`**: Black model is undefined for non-positive forwards; returns error.
///   For negative rate environments, use a normal (Bachelier) model instead.
/// - **`strike <= 0`**: Technically valid but unusual; log warning and proceed.
///
/// # Returns
///
/// `Ok(Money)` with the caplet/floorlet PV, or `Err` if inputs are invalid.
pub fn price_caplet_floorlet(inputs: CapletFloorletInputs) -> finstack_core::Result<Money> {
    let is_cap = inputs.is_cap;
    let notional = inputs.notional;
    let strike = inputs.strike;
    let forward = inputs.forward;
    let df = inputs.discount_factor;
    let sigma = inputs.volatility;
    let t_fix = inputs.time_to_fixing;
    let tau = inputs.accrual_year_fraction;
    let ccy = inputs.currency;

    // Edge case: Option is at or past fixing -> intrinsic value
    if t_fix <= 0.0 {
        return Ok(Money::new(intrinsic_value(&inputs), ccy));
    }

    // Edge case: Zero or negative volatility -> intrinsic value
    // This handles the case where vol surface returns 0 or negative due to extrapolation
    if sigma <= 0.0 || !sigma.is_finite() {
        return Ok(Money::new(intrinsic_value(&inputs), ccy));
    }

    // Edge case: Non-positive forward rate
    // Black (1976) model requires F > 0 since it uses log(F/K)
    // For negative rate environments, the normal (Bachelier) model should be used
    if forward <= 0.0 {
        return Err(finstack_core::Error::Validation(format!(
            "Black model requires positive forward rate; got forward={:.6}. \
             For negative rate environments, use Normal (Bachelier) model.",
            forward
        )));
    }

    // Edge case: Non-positive strike - unusual but technically valid for deep ITM caps
    // The formula still works, but log warning
    if strike <= 0.0 {
        tracing::warn!(
            forward = forward,
            strike = strike,
            "Black caplet pricing with non-positive strike; result may be imprecise"
        );
    }

    let d1 = bs_d1(forward, strike, 0.0, sigma, t_fix, 0.0);
    let d2 = bs_d2(forward, strike, 0.0, sigma, t_fix, 0.0);

    // Check for NaN in d1/d2 which can happen with extreme inputs
    if !d1.is_finite() || !d2.is_finite() {
        // Fall back to intrinsic if Black formula produces NaN
        tracing::warn!(
            forward = forward,
            strike = strike,
            sigma = sigma,
            t_fix = t_fix,
            d1 = d1,
            d2 = d2,
            "Black caplet d1/d2 are non-finite; falling back to intrinsic value"
        );
        return Ok(Money::new(intrinsic_value(&inputs), ccy));
    }

    let pv = if is_cap {
        df * tau * notional * (forward * norm_cdf(d1) - strike * norm_cdf(d2))
    } else {
        df * tau * notional * (strike * norm_cdf(-d2) - forward * norm_cdf(-d1))
    };

    // Final sanity check
    if !pv.is_finite() {
        return Err(finstack_core::Error::Validation(format!(
            "Black caplet pricing produced non-finite PV: forward={}, strike={}, sigma={}, t={}",
            forward, strike, sigma, t_fix
        )));
    }

    Ok(Money::new(pv, ccy))
}

/// Black forward delta (per unit forward).
pub fn delta(is_cap: bool, strike: f64, forward: f64, sigma: f64, t_fix: f64) -> f64 {
    if t_fix <= 0.0 || sigma <= 0.0 {
        if is_cap {
            return if forward > strike { 1.0 } else { 0.0 };
        } else {
            return if forward < strike { -1.0 } else { 0.0 };
        }
    }
    let d1 = bs_d1(forward, strike, 0.0, sigma, t_fix, 0.0);
    if is_cap {
        norm_cdf(d1)
    } else {
        -norm_cdf(-d1)
    }
}

/// Black forward gamma (per unit forward).
pub fn gamma(strike: f64, forward: f64, sigma: f64, t_fix: f64) -> f64 {
    if t_fix <= 0.0 || sigma <= 0.0 || forward <= 0.0 {
        return 0.0;
    }
    let d1 = bs_d1(forward, strike, 0.0, sigma, t_fix, 0.0);
    norm_pdf(d1) / (forward * sigma * t_fix.sqrt())
}

/// Black vega per 1% vol.
pub fn vega_per_pct(strike: f64, forward: f64, sigma: f64, t_fix: f64) -> f64 {
    if t_fix <= 0.0 || forward <= 0.0 {
        return 0.0;
    }
    let d1 = if sigma > 0.0 {
        bs_d1(forward, strike, 0.0, sigma, t_fix, 0.0)
    } else {
        0.0
    };
    forward * norm_pdf(d1) * t_fix.sqrt() / 100.0
}
