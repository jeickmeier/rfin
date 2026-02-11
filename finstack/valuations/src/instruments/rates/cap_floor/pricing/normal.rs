//! Bachelier (normal) model helpers for interest rate caplets/floorlets.
//!
//! This pricing path is used when cap/floor volatility is quoted in normal terms
//! (absolute rate units), which is common in low/negative-rate environments.

use crate::instruments::common_impl::models::volatility::normal::bachelier_price;
use crate::instruments::common_impl::parameters::OptionType;
use finstack_core::currency::Currency;
use finstack_core::money::Money;

/// Inputs for normal (Bachelier) caplet/floorlet pricing.
#[derive(Debug, Clone, Copy)]
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
    /// Normal volatility (absolute rate units, annualized)
    pub volatility: f64,
    /// Time to fixing date in years
    pub time_to_fixing: f64,
    /// Accrual year fraction for the period
    pub accrual_year_fraction: f64,
    /// Currency for the cashflow
    pub currency: Currency,
}

/// Price a caplet/floorlet using Bachelier's normal model.
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

    if !forward.is_finite()
        || !strike.is_finite()
        || !sigma.is_finite()
        || !t_fix.is_finite()
        || !tau.is_finite()
        || !df.is_finite()
        || !notional.is_finite()
    {
        return Err(finstack_core::Error::Validation(
            "Normal cap/floor pricing received non-finite input".to_string(),
        ));
    }

    if tau < 0.0 || df < 0.0 || notional < 0.0 {
        return Err(finstack_core::Error::Validation(
            "Normal cap/floor pricing requires non-negative notional, discount factor, and accrual"
                .to_string(),
        ));
    }

    let annuity = notional * tau * df;
    let option_type = if is_cap {
        OptionType::Call
    } else {
        OptionType::Put
    };

    // For near-zero/invalid vol, return intrinsic to avoid unstable Bachelier numerics.
    const MIN_NORMAL_VOL: f64 = 1e-8;
    let effective_sigma = if sigma.is_finite() { sigma } else { 0.0 };
    if effective_sigma < MIN_NORMAL_VOL {
        let intrinsic = if is_cap {
            (forward - strike).max(0.0)
        } else {
            (strike - forward).max(0.0)
        };
        return Ok(Money::new(intrinsic * annuity, ccy));
    }

    let pv = bachelier_price(
        option_type,
        forward,
        strike,
        effective_sigma.max(MIN_NORMAL_VOL),
        t_fix.max(0.0),
        annuity,
    );

    if !pv.is_finite() {
        return Err(finstack_core::Error::Validation(format!(
            "Normal cap/floor pricing produced non-finite PV: forward={}, strike={}, sigma={}, t={}",
            forward, strike, sigma, t_fix
        )));
    }

    Ok(Money::new(pv, ccy))
}
