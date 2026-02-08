//! Analytical formulas for barrier options with continuous monitoring.
//!
//! Provides closed-form pricing formulas for European barrier options that
//! knock in or out when the underlying asset price crosses a barrier level.
//! These formulas assume continuous barrier monitoring and serve as:
//! 1. **Validation benchmarks** for discrete barrier adjustments
//! 2. **Production pricing** when continuous monitoring is appropriate
//!
//! # Barrier Option Types
//!
//! - **Up-and-In**: Activated when spot rises above barrier (S > H)
//! - **Up-and-Out**: Deactivated when spot rises above barrier
//! - **Down-and-In**: Activated when spot falls below barrier (S < H)
//! - **Down-and-Out**: Deactivated when spot falls below barrier
//!
//! # Mathematical Foundation
//!
//! Barrier options are priced using the **reflection principle** applied to
//! geometric Brownian motion. The key insight is that barrier crossing
//! probabilities can be computed analytically using mirror image arguments.
//!
//! ## General Formula (Reiner-Rubinstein 1991)
//!
//! The price decomposes into combinations of vanilla options and
//! barrier-adjusted terms involving powers of (H/S) where H is the barrier.
//!
//! For a down-and-out call:
//! ```text
//! C_do = C_vanilla - C_knock_in
//!      = S·e^(-qT)·N(x) - K·e^(-rT)·N(x - σ√T)
//!        - (H/S)^(2λ) · [H·e^(-qT)·N(y) - K·e^(-rT)·N(y - σ√T)]
//! ```
//!
//! where λ = (r - q + σ²/2) / σ² and x, y are appropriately defined d-parameters.
//!
//! # Discrete Monitoring Corrections
//!
//! Real-world barriers are monitored discretely (e.g., daily closes), not continuously.
//! Continuous barrier formulas **underestimate** discrete barrier option values.
//!
//! Common corrections:
//! - **Broadie-Glasserman-Kou (1997)**: Adjust barrier by factor exp(±0.5826σ√Δt)
//! - **Gobet (2000)**: Higher-order correction using Brownian bridge
//! - **Rule of thumb**: H_adj = H · exp(±0.5826σ√Δt) where Δt is monitoring frequency
//!
//! # Academic References
//!
//! ## Primary Sources
//!
//! - Reiner, E., & Rubinstein, M. (1991). "Breaking Down the Barriers."
//!   *Risk Magazine*, 4(8), 28-35.
//!   (Canonical formulas for all 8 barrier option types)
//!
//! - Merton, R. C. (1973). "Theory of Rational Option Pricing."
//!   *Bell Journal of Economics and Management Science*, 4(1), 141-183.
//!   (Foundational work including barrier option theory)
//!
//! ## Discrete Monitoring Corrections
//!
//! - Broadie, M., Glasserman, P., & Kou, S. G. (1997). "A Continuity Correction
//!   for Discrete Barrier Options." *Mathematical Finance*, 7(4), 325-349.
//!
//! - Gobet, E. (2000). "Weak Approximation of Killed Diffusion Using Euler Schemes."
//!   *Stochastic Processes and their Applications*, 87(2), 167-197.
//!
//! - Fusai, G., & Recchioni, M. C. (2007). "Analysis of Quadrature Methods for
//!   Pricing Discrete Barrier Options." *Journal of Economic Dynamics and Control*,
//!   31(3), 826-860.
//!
//! ## Reference Texts
//!
//! - Haug, E. G. (2007). *The Complete Guide to Option Pricing Formulas* (2nd ed.).
//!   McGraw-Hill. Chapter 4: Barrier Options.
//!
//! - Wilmott, P., Howison, S., & Dewynne, J. (1995). *The Mathematics of Financial
//!   Derivatives*. Cambridge University Press. Chapter 8.
//!
//! # Implementation Notes
//!
//! - Formulas are numerically stable for typical parameter ranges
//! - Edge cases handled: zero time, barrier already crossed, extreme strikes
//! - Rebates paid at expiry are supported via `barrier_rebate_continuous`
//! - For discrete monitoring in production, apply Broadie-Glasserman-Kou correction
//!
//! # Examples
//!
//! ## Down-and-Out Call
//!
//! ```rust,ignore
//! use finstack_valuations::instruments::common::models::closed_form::barrier::down_out_call;
//!
//! let spot = 100.0;
//! let strike = 100.0;
//! let barrier = 90.0;    // Barrier below current spot
//! let time = 1.0;
//! let rate = 0.05;
//! let div_yield = 0.02;
//! let vol = 0.20;
//!
//! let price = down_out_call(spot, strike, barrier, time, rate, div_yield, vol);
//!
//! // Price should be less than vanilla call (knockout feature reduces value)
//! assert!(price >= 0.0);
//! ```
//!
//! ## Down-and-In Call
//!
//! ```rust,ignore
//! use finstack_valuations::instruments::common::models::closed_form::barrier::down_in_call;
//!
//! let spot = 100.0;
//! let strike = 100.0;
//! let barrier = 90.0;    // Barrier below current spot
//! let time = 0.5;
//! let rate = 0.05;
//! let div_yield = 0.0;
//! let vol = 0.25;
//!
//! // Option only activates if spot falls to 90
//! let price = down_in_call(spot, strike, barrier, time, rate, div_yield, vol);
//! assert!(price >= 0.0);
//! ```
//!
//! # See Also
//!
//! - [`BarrierType`] for barrier option classification
//! - [`BarrierParams`] for parameter grouping
//! - Monte Carlo barrier pricing for discrete monitoring and exotic payoffs

use finstack_core::math::special_functions::norm_cdf;

/// Parameters for barrier option pricing.
#[derive(Clone, Copy, Debug)]
pub struct BarrierParams {
    /// Current underlying spot price
    pub spot: f64,
    /// Strike price
    pub strike: f64,
    /// Barrier level
    pub barrier: f64,
    /// Time to expiry in years
    pub time: f64,
    /// Risk-free interest rate (annualized)
    pub rate: f64,
    /// Continuous dividend yield (annualized)
    pub div_yield: f64,
    /// Volatility (annualized)
    pub vol: f64,
}

impl BarrierParams {
    /// Create new barrier parameters
    pub fn new(
        spot: f64,
        strike: f64,
        barrier: f64,
        time: f64,
        rate: f64,
        div_yield: f64,
        vol: f64,
    ) -> Self {
        Self {
            spot,
            strike,
            barrier,
            time,
            rate,
            div_yield,
            vol,
        }
    }
}

/// Barrier option type.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BarrierType {
    /// Up-and-in barrier option (activates when spot rises above barrier)
    UpIn,
    /// Up-and-out barrier option (deactivates when spot rises above barrier)
    UpOut,
    /// Down-and-in barrier option (activates when spot falls below barrier)
    DownIn,
    /// Down-and-out barrier option (deactivates when spot falls below barrier)
    DownOut,
}

/// Helper function for barrier pricing.
#[allow(clippy::too_many_arguments)]
fn barrier_helper(
    spot: f64,
    strike: f64,
    barrier: f64,
    time: f64,
    rate: f64,
    div_yield: f64,
    vol: f64,
    eta: f64, // 1 for call, -1 for put
    phi: f64, // 1 for up, -1 for down
) -> f64 {
    if time <= 0.0 || vol <= 0.0 {
        return 0.0;
    }

    let mu = (rate - div_yield - 0.5 * vol * vol) / (vol * vol);

    let x = (spot / strike).ln() / (vol * time.sqrt()) + (1.0 + mu) * vol * time.sqrt();
    let x1 = (spot / barrier).ln() / (vol * time.sqrt()) + (1.0 + mu) * vol * time.sqrt();
    let y = (barrier * barrier / (spot * strike)).ln() / (vol * time.sqrt())
        + (1.0 + mu) * vol * time.sqrt();
    let y1 = (barrier / spot).ln() / (vol * time.sqrt()) + (1.0 + mu) * vol * time.sqrt();

    let discount = (-rate * time).exp();
    let forward_discount = (-div_yield * time).exp();

    // Standard vanilla components
    let a = phi * spot * forward_discount * norm_cdf(phi * x)
        - phi * strike * discount * norm_cdf(phi * (x - vol * time.sqrt()));

    // Barrier-adjusted components
    let b = phi * spot * forward_discount * norm_cdf(phi * x1)
        - phi * strike * discount * norm_cdf(phi * (x1 - vol * time.sqrt()));

    let c =
        phi * spot * forward_discount * (barrier / spot).powf(2.0 * (mu + 1.0)) * norm_cdf(eta * y)
            - phi
                * strike
                * discount
                * (barrier / spot).powf(2.0 * mu)
                * norm_cdf(eta * (y - vol * time.sqrt()));

    let d = phi
        * spot
        * forward_discount
        * (barrier / spot).powf(2.0 * (mu + 1.0))
        * norm_cdf(eta * y1)
        - phi
            * strike
            * discount
            * (barrier / spot).powf(2.0 * mu)
            * norm_cdf(eta * (y1 - vol * time.sqrt()));

    let is_call = eta > 0.0;

    // Combine based on barrier type
    if spot > barrier {
        // Up barrier
        if is_call {
            // Call
            a - b + c - d
        } else {
            // Put
            b - c + d
        }
    } else {
        // Down barrier
        if is_call {
            // Call
            b - c + d
        } else {
            // Put
            a - b + c - d
        }
    }
}

/// Calculate probability of hitting the barrier before T (Touch Probability).
///
/// Returns P(min S < H) for down barrier, or P(max S > H) for up barrier.
pub fn barrier_touch_probability(
    spot: f64,
    barrier: f64,
    time: f64,
    rate: f64,
    div_yield: f64,
    vol: f64,
    is_up: bool,
) -> f64 {
    if time <= 0.0 {
        return if is_up {
            if spot >= barrier {
                1.0
            } else {
                0.0
            }
        } else if spot <= barrier {
            1.0
        } else {
            0.0
        };
    }

    let sigma_sqrt_t = vol * time.sqrt();
    let nu = rate - div_yield - 0.5 * vol * vol;

    if is_up {
        if spot >= barrier {
            return 1.0;
        }
        // Up barrier (H > S)
        // P(max S > H)
        let x = (barrier / spot).ln(); // Positive
        let d1 = (-x + nu * time) / sigma_sqrt_t;
        let d2 = (-x - nu * time) / sigma_sqrt_t;
        // (H/S)^(2*nu/sigma^2)
        let power_term = (barrier / spot).powf(2.0 * nu / (vol * vol));

        norm_cdf(d1) + power_term * norm_cdf(d2)
    } else {
        if spot <= barrier {
            return 1.0;
        }
        // Down barrier (H < S)
        // P(min S < H)
        let log_h_s = (barrier / spot).ln(); // Negative
        let d1 = (log_h_s - nu * time) / sigma_sqrt_t;
        let d2 = (log_h_s + nu * time) / sigma_sqrt_t;
        let power_term = (barrier / spot).powf(2.0 * nu / (vol * vol));

        norm_cdf(d1) + power_term * norm_cdf(d2)
    }
}

/// Calculate value of a rebate paid at maturity.
///
/// - For Knock-Out: Paid if barrier is hit (Hit Rebate).
/// - For Knock-In: Paid if barrier is NOT hit (No-Hit Rebate).
#[allow(clippy::too_many_arguments)]
pub fn barrier_rebate_continuous(
    spot: f64,
    barrier: f64,
    rebate: f64,
    time: f64,
    rate: f64,
    div_yield: f64,
    vol: f64,
    barrier_type: BarrierType,
) -> f64 {
    let is_up = matches!(barrier_type, BarrierType::UpIn | BarrierType::UpOut);
    let p_hit = barrier_touch_probability(spot, barrier, time, rate, div_yield, vol, is_up);

    let df = (-rate * time).exp();

    match barrier_type {
        BarrierType::UpIn | BarrierType::DownIn => {
            // Knock-In: Option activates if Hit.
            // Rebate paid if it fails to activate (Not Hit).
            rebate * df * (1.0 - p_hit)
        }
        BarrierType::UpOut | BarrierType::DownOut => {
            // Knock-Out: Option deactivates if Hit.
            // Rebate paid if it deactivates (Hit).
            rebate * df * p_hit
        }
    }
}

/// Price a continuous up-and-out call.
pub fn up_out_call(
    spot: f64,
    strike: f64,
    barrier: f64,
    time: f64,
    rate: f64,
    div_yield: f64,
    vol: f64,
) -> f64 {
    if spot >= barrier {
        return 0.0; // Already knocked out
    }

    // Up-and-out = Vanilla - Up-and-in
    let vanilla = {
        let d1 = ((spot / strike).ln() + (rate - div_yield + 0.5 * vol * vol) * time)
            / (vol * time.sqrt());
        let d2 = d1 - vol * time.sqrt();
        spot * (-div_yield * time).exp() * norm_cdf(d1)
            - strike * (-rate * time).exp() * norm_cdf(d2)
    };

    let up_in = barrier_helper(spot, strike, barrier, time, rate, div_yield, vol, 1.0, 1.0);

    vanilla - up_in
}

/// Price a continuous up-and-in call.
pub fn up_in_call(
    spot: f64,
    strike: f64,
    barrier: f64,
    time: f64,
    rate: f64,
    div_yield: f64,
    vol: f64,
) -> f64 {
    if spot >= barrier {
        // Already knocked in, price as vanilla
        let d1 = ((spot / strike).ln() + (rate - div_yield + 0.5 * vol * vol) * time)
            / (vol * time.sqrt());
        let d2 = d1 - vol * time.sqrt();
        return spot * (-div_yield * time).exp() * norm_cdf(d1)
            - strike * (-rate * time).exp() * norm_cdf(d2);
    }

    barrier_helper(spot, strike, barrier, time, rate, div_yield, vol, 1.0, 1.0)
}

/// Price a continuous down-and-out call.
pub fn down_out_call(
    spot: f64,
    strike: f64,
    barrier: f64,
    time: f64,
    rate: f64,
    div_yield: f64,
    vol: f64,
) -> f64 {
    if spot <= barrier {
        return 0.0; // Already knocked out
    }

    let vanilla = {
        let d1 = ((spot / strike).ln() + (rate - div_yield + 0.5 * vol * vol) * time)
            / (vol * time.sqrt());
        let d2 = d1 - vol * time.sqrt();
        spot * (-div_yield * time).exp() * norm_cdf(d1)
            - strike * (-rate * time).exp() * norm_cdf(d2)
    };

    let down_in = barrier_helper(spot, strike, barrier, time, rate, div_yield, vol, 1.0, -1.0);

    vanilla - down_in
}

/// Price a continuous down-and-in call.
pub fn down_in_call(
    spot: f64,
    strike: f64,
    barrier: f64,
    time: f64,
    rate: f64,
    div_yield: f64,
    vol: f64,
) -> f64 {
    if spot <= barrier {
        // Already knocked in, price as vanilla
        let d1 = ((spot / strike).ln() + (rate - div_yield + 0.5 * vol * vol) * time)
            / (vol * time.sqrt());
        let d2 = d1 - vol * time.sqrt();
        return spot * (-div_yield * time).exp() * norm_cdf(d1)
            - strike * (-rate * time).exp() * norm_cdf(d2);
    }

    barrier_helper(spot, strike, barrier, time, rate, div_yield, vol, 1.0, -1.0)
}

/// Generic barrier call price dispatcher.
#[allow(clippy::too_many_arguments)]
pub fn barrier_call_continuous(
    spot: f64,
    strike: f64,
    barrier: f64,
    time: f64,
    rate: f64,
    div_yield: f64,
    vol: f64,
    barrier_type: BarrierType,
) -> f64 {
    match barrier_type {
        BarrierType::UpIn => up_in_call(spot, strike, barrier, time, rate, div_yield, vol),
        BarrierType::UpOut => up_out_call(spot, strike, barrier, time, rate, div_yield, vol),
        BarrierType::DownIn => down_in_call(spot, strike, barrier, time, rate, div_yield, vol),
        BarrierType::DownOut => down_out_call(spot, strike, barrier, time, rate, div_yield, vol),
    }
}

/// Price a continuous down-and-in put.
pub fn down_in_put(
    spot: f64,
    strike: f64,
    barrier: f64,
    time: f64,
    rate: f64,
    div_yield: f64,
    vol: f64,
) -> f64 {
    if spot <= barrier {
        // Already knocked in, price as vanilla put
        let d1 = ((spot / strike).ln() + (rate - div_yield + 0.5 * vol * vol) * time)
            / (vol * time.sqrt());
        let d2 = d1 - vol * time.sqrt();
        return strike * (-rate * time).exp() * norm_cdf(-d2)
            - spot * (-div_yield * time).exp() * norm_cdf(-d1);
    }

    barrier_helper(
        spot, strike, barrier, time, rate, div_yield, vol, -1.0, -1.0,
    )
}

/// Price a continuous down-and-out put.
pub fn down_out_put(
    spot: f64,
    strike: f64,
    barrier: f64,
    time: f64,
    rate: f64,
    div_yield: f64,
    vol: f64,
) -> f64 {
    if spot <= barrier {
        return 0.0; // Already knocked out
    }

    // Vanilla put
    let vanilla = {
        let d1 = ((spot / strike).ln() + (rate - div_yield + 0.5 * vol * vol) * time)
            / (vol * time.sqrt());
        let d2 = d1 - vol * time.sqrt();
        strike * (-rate * time).exp() * norm_cdf(-d2)
            - spot * (-div_yield * time).exp() * norm_cdf(-d1)
    };

    let down_in = barrier_helper(
        spot, strike, barrier, time, rate, div_yield, vol, -1.0, -1.0,
    );

    vanilla - down_in
}

/// Price a continuous up-and-in put.
pub fn up_in_put(
    spot: f64,
    strike: f64,
    barrier: f64,
    time: f64,
    rate: f64,
    div_yield: f64,
    vol: f64,
) -> f64 {
    if spot >= barrier {
        // Already knocked in, price as vanilla put
        let d1 = ((spot / strike).ln() + (rate - div_yield + 0.5 * vol * vol) * time)
            / (vol * time.sqrt());
        let d2 = d1 - vol * time.sqrt();
        return strike * (-rate * time).exp() * norm_cdf(-d2)
            - spot * (-div_yield * time).exp() * norm_cdf(-d1);
    }

    barrier_helper(spot, strike, barrier, time, rate, div_yield, vol, -1.0, 1.0)
}

/// Price a continuous up-and-out put.
pub fn up_out_put(
    spot: f64,
    strike: f64,
    barrier: f64,
    time: f64,
    rate: f64,
    div_yield: f64,
    vol: f64,
) -> f64 {
    if spot >= barrier {
        return 0.0; // Already knocked out
    }

    // Vanilla put
    let vanilla = {
        let d1 = ((spot / strike).ln() + (rate - div_yield + 0.5 * vol * vol) * time)
            / (vol * time.sqrt());
        let d2 = d1 - vol * time.sqrt();
        strike * (-rate * time).exp() * norm_cdf(-d2)
            - spot * (-div_yield * time).exp() * norm_cdf(-d1)
    };

    let up_in = barrier_helper(spot, strike, barrier, time, rate, div_yield, vol, -1.0, 1.0);

    vanilla - up_in
}

/// Generic barrier put price dispatcher.
#[allow(clippy::too_many_arguments)]
pub fn barrier_put_continuous(
    spot: f64,
    strike: f64,
    barrier: f64,
    time: f64,
    rate: f64,
    div_yield: f64,
    vol: f64,
    barrier_type: BarrierType,
) -> f64 {
    match barrier_type {
        BarrierType::UpIn => up_in_put(spot, strike, barrier, time, rate, div_yield, vol),
        BarrierType::UpOut => up_out_put(spot, strike, barrier, time, rate, div_yield, vol),
        BarrierType::DownIn => down_in_put(spot, strike, barrier, time, rate, div_yield, vol),
        BarrierType::DownOut => down_out_put(spot, strike, barrier, time, rate, div_yield, vol),
    }
}

// ==================== DF-FIRST WRAPPERS ====================
//
// These wrappers take the discount factor directly instead of a rate, ensuring
// that r_eff and time are on consistent bases (no day-count mismatches).
// Use these when DF is sourced from date-based curve lookups.

/// Price a continuous barrier call with explicit discount factor (DF-first API).
///
/// This is the preferred entry point when `df` is known directly (e.g., from
/// date-based curve lookup). Derives `r_eff = -ln(df)/t` internally.
///
/// See [`barrier_call_continuous`] for formula details.
#[allow(clippy::too_many_arguments)]
pub fn barrier_call_continuous_df(
    spot: f64,
    strike: f64,
    barrier: f64,
    time: f64,
    df: f64,
    div_yield: f64,
    vol: f64,
    barrier_type: BarrierType,
) -> f64 {
    // Derive rate from DF for internal calculations
    let rate = if time > 0.0 && df > 0.0 {
        -df.ln() / time
    } else {
        0.0
    };
    barrier_call_continuous(
        spot,
        strike,
        barrier,
        time,
        rate,
        div_yield,
        vol,
        barrier_type,
    )
}

/// Price a continuous barrier put with explicit discount factor (DF-first API).
///
/// This is the preferred entry point when `df` is known directly (e.g., from
/// date-based curve lookup). Derives `r_eff = -ln(df)/t` internally.
///
/// See [`barrier_put_continuous`] for formula details.
#[allow(clippy::too_many_arguments)]
pub fn barrier_put_continuous_df(
    spot: f64,
    strike: f64,
    barrier: f64,
    time: f64,
    df: f64,
    div_yield: f64,
    vol: f64,
    barrier_type: BarrierType,
) -> f64 {
    // Derive rate from DF for internal calculations
    let rate = if time > 0.0 && df > 0.0 {
        -df.ln() / time
    } else {
        0.0
    };
    barrier_put_continuous(
        spot,
        strike,
        barrier,
        time,
        rate,
        div_yield,
        vol,
        barrier_type,
    )
}

/// Price a barrier rebate with explicit discount factor (DF-first API).
///
/// This is the preferred entry point when `df` is known directly (e.g., from
/// date-based curve lookup). Derives `r_eff = -ln(df)/t` internally.
///
/// See [`barrier_rebate_continuous`] for formula details.
#[allow(clippy::too_many_arguments)]
pub fn barrier_rebate_continuous_df(
    spot: f64,
    barrier: f64,
    rebate: f64,
    time: f64,
    df: f64,
    div_yield: f64,
    vol: f64,
    barrier_type: BarrierType,
) -> f64 {
    // Derive rate from DF for internal calculations
    let rate = if time > 0.0 && df > 0.0 {
        -df.ln() / time
    } else {
        0.0
    };
    barrier_rebate_continuous(
        spot,
        barrier,
        rebate,
        time,
        rate,
        div_yield,
        vol,
        barrier_type,
    )
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_barrier_in_plus_out_equals_vanilla() {
        let spot = 100.0;
        let strike = 100.0;
        let barrier = 120.0;
        let time = 1.0;
        let rate = 0.05;
        let div_yield = 0.02;
        let vol = 0.2;

        let up_in = up_in_call(spot, strike, barrier, time, rate, div_yield, vol);
        let up_out = up_out_call(spot, strike, barrier, time, rate, div_yield, vol);

        // Vanilla call price
        let d1 = ((spot / strike).ln() + (rate - div_yield + 0.5 * vol * vol) * time)
            / (vol * time.sqrt());
        let d2 = d1 - vol * time.sqrt();
        let vanilla = spot * (-div_yield * time).exp() * norm_cdf(d1)
            - strike * (-rate * time).exp() * norm_cdf(d2);

        let sum = up_in + up_out;

        assert!(
            (sum - vanilla).abs() < 0.01,
            "Barrier parity failed: {} vs {}",
            sum,
            vanilla
        );
    }

    #[test]
    fn test_barrier_put_in_plus_out_equals_vanilla() {
        let spot = 100.0;
        let strike = 100.0;
        let barrier = 80.0;
        let time = 1.0;
        let rate = 0.05;
        let div_yield = 0.02;
        let vol = 0.2;

        let down_in = barrier_put_continuous(
            spot,
            strike,
            barrier,
            time,
            rate,
            div_yield,
            vol,
            BarrierType::DownIn,
        );
        let down_out = barrier_put_continuous(
            spot,
            strike,
            barrier,
            time,
            rate,
            div_yield,
            vol,
            BarrierType::DownOut,
        );

        let d1 = ((spot / strike).ln() + (rate - div_yield + 0.5 * vol * vol) * time)
            / (vol * time.sqrt());
        let d2 = d1 - vol * time.sqrt();
        let vanilla = strike * (-rate * time).exp() * norm_cdf(-d2)
            - spot * (-div_yield * time).exp() * norm_cdf(-d1);

        let sum = down_in + down_out;

        assert!(
            (sum - vanilla).abs() < 0.01,
            "Barrier parity failed: {} vs {}",
            sum,
            vanilla
        );
    }

    #[test]
    fn test_up_out_call_knocked_out() {
        let price = up_out_call(125.0, 100.0, 120.0, 1.0, 0.05, 0.02, 0.2);
        assert_eq!(price, 0.0, "Already above barrier should be zero");
    }

    #[test]
    fn test_down_out_call_knocked_out() {
        let price = down_out_call(75.0, 100.0, 80.0, 1.0, 0.05, 0.02, 0.2);
        assert_eq!(price, 0.0, "Already below barrier should be zero");
    }

    #[test]
    fn test_barrier_prices_non_negative() {
        let spot = 100.0;
        let strike = 100.0;
        let barrier_up = 120.0;
        let barrier_down = 80.0;
        let time = 1.0;
        let rate = 0.05;
        let div_yield = 0.02;
        let vol = 0.2;

        assert!(up_in_call(spot, strike, barrier_up, time, rate, div_yield, vol) >= 0.0);
        assert!(up_out_call(spot, strike, barrier_up, time, rate, div_yield, vol) >= 0.0);
        assert!(down_in_call(spot, strike, barrier_down, time, rate, div_yield, vol) >= 0.0);
        assert!(down_out_call(spot, strike, barrier_down, time, rate, div_yield, vol) >= 0.0);
    }

    // ==================== DF-WRAPPER TESTS ====================

    #[test]
    fn test_df_wrapper_consistency_barrier_call() {
        let spot = 100.0_f64;
        let strike = 100.0_f64;
        let barrier = 120.0_f64;
        let time = 1.0_f64;
        let rate = 0.05_f64;
        let div_yield = 0.02_f64;
        let vol = 0.2_f64;
        let df = (-rate * time).exp();

        for barrier_type in [
            BarrierType::UpIn,
            BarrierType::UpOut,
            BarrierType::DownIn,
            BarrierType::DownOut,
        ] {
            let b = if matches!(barrier_type, BarrierType::DownIn | BarrierType::DownOut) {
                80.0 // Use down barrier for down types
            } else {
                barrier
            };

            let price_rate =
                barrier_call_continuous(spot, strike, b, time, rate, div_yield, vol, barrier_type);
            let price_df =
                barrier_call_continuous_df(spot, strike, b, time, df, div_yield, vol, barrier_type);

            assert!(
                (price_rate - price_df).abs() < 1e-10,
                "{:?}: rate-based {} vs df-based {}",
                barrier_type,
                price_rate,
                price_df
            );
        }
    }

    #[test]
    fn test_df_wrapper_consistency_barrier_put() {
        let spot = 100.0_f64;
        let strike = 100.0_f64;
        let barrier = 120.0_f64;
        let time = 1.0_f64;
        let rate = 0.05_f64;
        let div_yield = 0.02_f64;
        let vol = 0.2_f64;
        let df = (-rate * time).exp();

        for barrier_type in [
            BarrierType::UpIn,
            BarrierType::UpOut,
            BarrierType::DownIn,
            BarrierType::DownOut,
        ] {
            let b = if matches!(barrier_type, BarrierType::DownIn | BarrierType::DownOut) {
                80.0
            } else {
                barrier
            };

            let price_rate =
                barrier_put_continuous(spot, strike, b, time, rate, div_yield, vol, barrier_type);
            let price_df =
                barrier_put_continuous_df(spot, strike, b, time, df, div_yield, vol, barrier_type);

            assert!(
                (price_rate - price_df).abs() < 1e-10,
                "{:?}: rate-based {} vs df-based {}",
                barrier_type,
                price_rate,
                price_df
            );
        }
    }

    #[test]
    fn test_df_wrapper_consistency_rebate() {
        let spot = 100.0_f64;
        let barrier = 120.0_f64;
        let rebate = 5.0_f64;
        let time = 1.0_f64;
        let rate = 0.05_f64;
        let div_yield = 0.02_f64;
        let vol = 0.2_f64;
        let df = (-rate * time).exp();

        for barrier_type in [
            BarrierType::UpIn,
            BarrierType::UpOut,
            BarrierType::DownIn,
            BarrierType::DownOut,
        ] {
            let b = if matches!(barrier_type, BarrierType::DownIn | BarrierType::DownOut) {
                80.0
            } else {
                barrier
            };

            let price_rate = barrier_rebate_continuous(
                spot,
                b,
                rebate,
                time,
                rate,
                div_yield,
                vol,
                barrier_type,
            );
            let price_df = barrier_rebate_continuous_df(
                spot,
                b,
                rebate,
                time,
                df,
                div_yield,
                vol,
                barrier_type,
            );

            assert!(
                (price_rate - price_df).abs() < 1e-10,
                "{:?}: rate-based {} vs df-based {}",
                barrier_type,
                price_rate,
                price_df
            );
        }
    }
}
