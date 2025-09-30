//! Black model helpers for interest rate caplets/floorlets.
//!
//! Exposes pure functions for price and greeks to keep `types.rs` free of pricing logic.

use crate::instruments::common::models::{d1 as bs_d1, d2 as bs_d2};
use finstack_core::currency::Currency;
use finstack_core::math::{norm_cdf, norm_pdf};
use finstack_core::money::Money;

/// Inputs for Black caplet/floorlet pricing
#[derive(Clone, Copy, Debug)]
pub struct CapletFloorletInputs {
    pub is_cap: bool,
    pub notional: f64,
    pub strike: f64,
    pub forward: f64,
    pub discount_factor: f64,
    pub volatility: f64,
    pub time_to_fixing: f64,
    pub accrual_year_fraction: f64,
    pub currency: Currency,
}

/// Price a caplet/floorlet using Black's formula.
///
/// Returns PV in the instrument currency given forward, discount factor, vol, time to fixing and accrual.
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

    if t_fix <= 0.0 {
        let payoff = if is_cap {
            (forward - strike).max(0.0)
        } else {
            (strike - forward).max(0.0)
        };
        return Ok(Money::new(payoff * tau * notional * df, ccy));
    }

    let d1 = bs_d1(forward, strike, 0.0, sigma, t_fix, 0.0);
    let d2 = bs_d2(forward, strike, 0.0, sigma, t_fix, 0.0);

    let pv = if is_cap {
        df * tau * notional * (forward * norm_cdf(d1) - strike * norm_cdf(d2))
    } else {
        df * tau * notional * (strike * norm_cdf(-d2) - forward * norm_cdf(-d1))
    };
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
