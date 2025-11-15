//! Black-Scholes-Merton analytical Greeks with full derivations.
//!
//! Provides closed-form formulas for option sensitivities (Greeks) derived from
//! the Black-Scholes-Merton model. These analytical formulas serve as:
//! 1. **Production risk metrics** for vanilla options
//! 2. **Validation benchmarks** for numerical Greek computations
//!
//! # Mathematical Foundation
//!
//! The Black-Scholes-Merton model assumes:
//! - Geometric Brownian motion for the underlying asset
//! - Constant volatility σ
//! - Constant risk-free rate r
//! - Continuous dividend yield q
//! - No arbitrage and frictionless markets
//!
//! ## Pricing Formulas
//!
//! **European Call:**
//! ```text
//! C = S·e^(-qT)·N(d₁) - K·e^(-rT)·N(d₂)
//! ```
//!
//! **European Put:**
//! ```text
//! P = K·e^(-rT)·N(-d₂) - S·e^(-qT)·N(-d₁)
//! ```
//!
//! where:
//! ```text
//! d₁ = [ln(S/K) + (r - q + σ²/2)T] / (σ√T)
//! d₂ = d₁ - σ√T
//! ```
//!
//! and N(·) is the cumulative standard normal distribution.
//!
//! # Greeks Definitions
//!
//! Greeks measure the sensitivity of option prices to various parameters:
//!
//! - **Delta (Δ)**: ∂V/∂S - Sensitivity to spot price changes
//! - **Gamma (Γ)**: ∂²V/∂S² - Rate of change of delta
//! - **Vega (ν)**: ∂V/∂σ - Sensitivity to volatility changes
//! - **Theta (Θ)**: ∂V/∂t - Time decay (negative of ∂V/∂T)
//! - **Rho (ρ)**: ∂V/∂r - Sensitivity to interest rate changes
//!
//! # References
//!
//! - Black, F., & Scholes, M. (1973). "The Pricing of Options and Corporate Liabilities."
//!   *Journal of Political Economy*, 81(3), 637-654.
//! - Merton, R. C. (1973). "Theory of Rational Option Pricing."
//!   *Bell Journal of Economics and Management Science*, 4(1), 141-183.
//! - Hull, J. C. (2018). *Options, Futures, and Other Derivatives* (10th ed.).
//!   Pearson. Chapter 19: The Greek Letters.
//! - Haug, E. G. (2007). *The Complete Guide to Option Pricing Formulas* (2nd ed.).
//!   McGraw-Hill. Chapter 1.
//!
//! # Examples
//!
//! ## Computing All Greeks
//!
//! ```rust
//! use finstack_valuations::instruments::common::models::closed_form::greeks::{
//!     bs_call_greeks, CallGreeks
//! };
//!
//! let spot = 100.0;
//! let strike = 100.0;
//! let time = 1.0;        // 1 year to expiration
//! let rate = 0.05;       // 5% risk-free rate
//! let div_yield = 0.02;  // 2% dividend yield
//! let vol = 0.20;        // 20% annual volatility
//!
//! let greeks: CallGreeks = bs_call_greeks(spot, strike, time, rate, div_yield, vol);
//!
//! println!("Delta: {:.4}", greeks.delta);
//! println!("Gamma: {:.4}", greeks.gamma);
//! println!("Vega: {:.4}", greeks.vega);
//! println!("Theta: {:.4}", greeks.theta);
//! println!("Rho: {:.4}", greeks.rho);
//! ```
//!
//! ## Individual Greek Calculations
//!
//! ```rust
//! use finstack_valuations::instruments::common::models::closed_form::greeks::{
//!     bs_call_delta, bs_gamma, bs_vega
//! };
//!
//! let spot = 100.0;
//! let strike = 100.0;
//! let time = 0.25;       // 3 months
//! let rate = 0.05;
//! let div_yield = 0.0;   // No dividends
//! let vol = 0.30;
//!
//! // Delta: probability of exercise (risk-neutral)
//! let delta = bs_call_delta(spot, strike, time, rate, div_yield, vol);
//! assert!((delta - 0.5).abs() < 0.1); // ATM call delta near 0.5
//!
//! // Gamma: curvature of delta
//! let gamma = bs_gamma(spot, strike, time, rate, div_yield, vol);
//! assert!(gamma > 0.0); // Always positive
//!
//! // Vega: volatility sensitivity (per 1% vol change)
//! let vega = bs_vega(spot, strike, time, rate, div_yield, vol);
//! assert!(vega > 0.0); // Always positive for both calls and puts
//! ```

use finstack_core::math::special_functions::{norm_cdf, norm_pdf};
// Reuse shared Black/BSM helpers to avoid duplication; wrap to preserve local signature ordering
use crate::instruments::common::models::volatility::black::{d1 as d1_shared, d2 as d2_shared};

/// Compute d₁ parameter for Black-Scholes-Merton formula.
///
/// Wrapper around shared helpers to preserve (spot, strike, time, rate, div_yield, vol)
/// argument order expected by callers in this module.
fn bs_d1(spot: f64, strike: f64, time: f64, rate: f64, div_yield: f64, vol: f64) -> f64 {
    // shared signature: d1(spot, strike, r, sigma, t, q)
    d1_shared(spot, strike, rate, vol, time, div_yield)
}

/// Compute d₂ parameter for Black-Scholes-Merton formula.
fn bs_d2(spot: f64, strike: f64, time: f64, rate: f64, div_yield: f64, vol: f64) -> f64 {
    d2_shared(spot, strike, rate, vol, time, div_yield)
}

/// Black-Scholes call option delta.
///
/// Delta measures the sensitivity of the option price to changes in the
/// underlying asset price. For a call option, delta ranges from 0 to 1.
///
/// # Formula
///
/// ```text
/// Δ_call = e^(-qT) · N(d₁)
/// ```
///
/// where q is the continuous dividend yield and N(·) is the cumulative
/// standard normal distribution.
///
/// # Interpretation
///
/// - **Hedge ratio**: Number of shares to hold to delta-hedge the option
/// - **Probability proxy**: Approximates risk-neutral probability of expiring ITM
/// - **Range**: [0, 1] with ATM call delta typically near 0.5
///
/// # Arguments
///
/// * `spot` - Current spot price S
/// * `strike` - Strike price K
/// * `time` - Time to expiration T (in years)
/// * `rate` - Risk-free rate r (continuously compounded)
/// * `div_yield` - Dividend yield q (continuously compounded)
/// * `vol` - Volatility σ (annualized)
///
/// # Returns
///
/// Call delta value. Returns 1.0 if deeply ITM at expiration, 0.0 if OTM.
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::instruments::common::models::closed_form::greeks::bs_call_delta;
///
/// let spot = 100.0;
/// let strike = 100.0;    // ATM
/// let time = 1.0;
/// let rate = 0.05;
/// let div_yield = 0.02;
/// let vol = 0.20;
///
/// let delta = bs_call_delta(spot, strike, time, rate, div_yield, vol);
/// // ATM call delta typically near 0.5
/// assert!((delta - 0.5).abs() < 0.1);
/// ```
pub fn bs_call_delta(
    spot: f64,
    strike: f64,
    time: f64,
    rate: f64,
    div_yield: f64,
    vol: f64,
) -> f64 {
    if time <= 0.0 {
        return if spot > strike { 1.0 } else { 0.0 };
    }

    let d1 = bs_d1(spot, strike, time, rate, div_yield, vol);
    (-div_yield * time).exp() * norm_cdf(d1)
}

/// Black-Scholes put option delta.
///
/// Delta measures the sensitivity of the option price to changes in the
/// underlying asset price. For a put option, delta ranges from -1 to 0.
///
/// # Formula
///
/// ```text
/// Δ_put = -e^(-qT) · N(-d₁) = e^(-qT) · [N(d₁) - 1]
/// ```
///
/// # Interpretation
///
/// - **Hedge ratio**: Negative delta means short position in underlying for hedge
/// - **Range**: [-1, 0] with ATM put delta typically near -0.5
/// - **Put-call parity**: Δ_put = Δ_call - e^(-qT)
///
/// # Arguments
///
/// * `spot` - Current spot price S
/// * `strike` - Strike price K
/// * `time` - Time to expiration T (in years)
/// * `rate` - Risk-free rate r (continuously compounded)
/// * `div_yield` - Dividend yield q (continuously compounded)
/// * `vol` - Volatility σ (annualized)
///
/// # Returns
///
/// Put delta value. Returns -1.0 if deeply ITM at expiration, 0.0 if OTM.
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::instruments::common::models::closed_form::greeks::bs_put_delta;
///
/// let spot = 100.0;
/// let strike = 100.0;    // ATM
/// let time = 1.0;
/// let rate = 0.05;
/// let div_yield = 0.02;
/// let vol = 0.20;
///
/// let delta = bs_put_delta(spot, strike, time, rate, div_yield, vol);
/// // ATM put delta typically near -0.5 (accounting for dividend yield adjustment)
/// assert!(delta < 0.0 && delta > -1.0);
/// ```
pub fn bs_put_delta(spot: f64, strike: f64, time: f64, rate: f64, div_yield: f64, vol: f64) -> f64 {
    if time <= 0.0 {
        return if spot < strike { -1.0 } else { 0.0 };
    }

    let d1 = bs_d1(spot, strike, time, rate, div_yield, vol);
    -(-div_yield * time).exp() * norm_cdf(-d1)
}

/// Black-Scholes gamma (same for both calls and puts).
///
/// Gamma measures the rate of change of delta with respect to the underlying
/// asset price. It represents the curvature of the option value function.
///
/// # Formula
///
/// ```text
/// Γ = e^(-qT) · φ(d₁) / (S · σ · √T)
/// ```
///
/// where φ(·) is the standard normal probability density function.
///
/// # Interpretation
///
/// - **Delta hedging cost**: High gamma means frequent rebalancing needed
/// - **Convexity**: Measures how quickly delta changes
/// - **Always positive**: Both calls and puts have positive gamma
/// - **Peaks at ATM**: Maximum gamma occurs when spot ≈ strike
///
/// # Arguments
///
/// * `spot` - Current spot price S
/// * `strike` - Strike price K
/// * `time` - Time to expiration T (in years)
/// * `rate` - Risk-free rate r (continuously compounded)
/// * `div_yield` - Dividend yield q (continuously compounded)
/// * `vol` - Volatility σ (annualized)
///
/// # Returns
///
/// Gamma value. Returns 0.0 at expiration or for zero volatility.
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::instruments::common::models::closed_form::greeks::bs_gamma;
///
/// let spot = 100.0;
/// let strike = 100.0;    // ATM has highest gamma
/// let time = 0.25;       // 3 months
/// let rate = 0.05;
/// let div_yield = 0.0;
/// let vol = 0.20;
///
/// let gamma = bs_gamma(spot, strike, time, rate, div_yield, vol);
/// assert!(gamma > 0.0); // Always positive
///
/// // OTM option has lower gamma
/// let gamma_otm = bs_gamma(spot, 110.0, time, rate, div_yield, vol);
/// assert!(gamma > gamma_otm);
/// ```
pub fn bs_gamma(spot: f64, strike: f64, time: f64, rate: f64, div_yield: f64, vol: f64) -> f64 {
    if time <= 0.0 || spot <= 0.0 || vol <= 0.0 {
        return 0.0;
    }

    let d1 = bs_d1(spot, strike, time, rate, div_yield, vol);
    (-div_yield * time).exp() * norm_pdf(d1) / (spot * vol * time.sqrt())
}

/// Black-Scholes vega (same for both calls and puts).
///
/// Vega measures the sensitivity of the option price to changes in implied
/// volatility. Not technically a "Greek" letter, but universally called vega.
///
/// # Formula
///
/// ```text
/// ν = S · e^(-qT) · √T · φ(d₁)
/// ```
///
/// # Interpretation
///
/// - **Volatility exposure**: Change in option value per 1% change in volatility
/// - **Always positive**: Long options have positive vega (benefit from vol increases)
/// - **Time decay**: Vega decreases as expiration approaches
/// - **Peaks at ATM**: Maximum vega when spot ≈ strike
///
/// # Convention
///
/// Vega is typically quoted per 1% (0.01) change in volatility. Some systems
/// quote per 1bp (0.0001) change, so verify conventions.
///
/// # Arguments
///
/// * `spot` - Current spot price S
/// * `strike` - Strike price K
/// * `time` - Time to expiration T (in years)
/// * `rate` - Risk-free rate r (continuously compounded)
/// * `div_yield` - Dividend yield q (continuously compounded)
/// * `vol` - Volatility σ (annualized)
///
/// # Returns
///
/// Vega value (per 1-unit change in volatility). Returns 0.0 at expiration.
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::instruments::common::models::closed_form::greeks::bs_vega;
///
/// let spot = 100.0;
/// let strike = 100.0;
/// let time = 1.0;
/// let rate = 0.05;
/// let div_yield = 0.0;
/// let vol = 0.20;
///
/// let vega = bs_vega(spot, strike, time, rate, div_yield, vol);
/// assert!(vega > 0.0); // Always positive for long options
///
/// // Vega decreases as expiration approaches
/// let vega_short = bs_vega(spot, strike, 0.25, rate, div_yield, vol);
/// assert!(vega > vega_short);
/// ```
pub fn bs_vega(spot: f64, strike: f64, time: f64, rate: f64, div_yield: f64, vol: f64) -> f64 {
    if time <= 0.0 {
        return 0.0;
    }

    let d1 = bs_d1(spot, strike, time, rate, div_yield, vol);
    spot * (-div_yield * time).exp() * time.sqrt() * norm_pdf(d1)
}

/// Black-Scholes call theta.
///
/// Θ_call = -S * φ(d1) * σ / (2√T) * exp(-qT) - r*K*exp(-rT)*N(d2) + q*S*exp(-qT)*N(d1)
pub fn bs_call_theta(
    spot: f64,
    strike: f64,
    time: f64,
    rate: f64,
    div_yield: f64,
    vol: f64,
) -> f64 {
    if time <= 0.0 {
        return 0.0;
    }

    let d1 = bs_d1(spot, strike, time, rate, div_yield, vol);
    let d2 = bs_d2(spot, strike, time, rate, div_yield, vol);

    let term1 = -spot * norm_pdf(d1) * vol * (-div_yield * time).exp() / (2.0 * time.sqrt());
    let term2 = -rate * strike * (-rate * time).exp() * norm_cdf(d2);
    let term3 = div_yield * spot * (-div_yield * time).exp() * norm_cdf(d1);

    term1 + term2 + term3
}

/// Black-Scholes put theta.
///
/// Θ_put = -S * φ(d1) * σ / (2√T) * exp(-qT) + r*K*exp(-rT)*N(-d2) - q*S*exp(-qT)*N(-d1)
pub fn bs_put_theta(spot: f64, strike: f64, time: f64, rate: f64, div_yield: f64, vol: f64) -> f64 {
    if time <= 0.0 {
        return 0.0;
    }

    let d1 = bs_d1(spot, strike, time, rate, div_yield, vol);
    let d2 = bs_d2(spot, strike, time, rate, div_yield, vol);

    let term1 = -spot * norm_pdf(d1) * vol * (-div_yield * time).exp() / (2.0 * time.sqrt());
    let term2 = rate * strike * (-rate * time).exp() * norm_cdf(-d2);
    let term3 = -div_yield * spot * (-div_yield * time).exp() * norm_cdf(-d1);

    term1 + term2 + term3
}

/// Black-Scholes call rho.
///
/// ρ_call = K * T * exp(-rT) * N(d2)
pub fn bs_call_rho(spot: f64, strike: f64, time: f64, rate: f64, div_yield: f64, vol: f64) -> f64 {
    if time <= 0.0 {
        return 0.0;
    }

    let d2 = bs_d2(spot, strike, time, rate, div_yield, vol);
    strike * time * (-rate * time).exp() * norm_cdf(d2)
}

/// Black-Scholes put rho.
///
/// ρ_put = -K * T * exp(-rT) * N(-d2)
pub fn bs_put_rho(spot: f64, strike: f64, time: f64, rate: f64, div_yield: f64, vol: f64) -> f64 {
    if time <= 0.0 {
        return 0.0;
    }

    let d2 = bs_d2(spot, strike, time, rate, div_yield, vol);
    -strike * time * (-rate * time).exp() * norm_cdf(-d2)
}

/// Convenience wrapper for all call Greeks.
pub struct CallGreeks {
    /// Delta: sensitivity to underlying price (∂C/∂S)
    pub delta: f64,
    /// Gamma: rate of change of delta (∂²C/∂S²)
    pub gamma: f64,
    /// Vega: sensitivity to volatility (∂C/∂σ), per 1% vol
    pub vega: f64,
    /// Theta: time decay (∂C/∂t), per day
    pub theta: f64,
    /// Rho: sensitivity to interest rate (∂C/∂r), per 1% rate
    pub rho: f64,
}

/// Convenience wrapper for all put Greeks.
pub struct PutGreeks {
    /// Delta: sensitivity to underlying price (∂P/∂S)
    pub delta: f64,
    /// Gamma: rate of change of delta (∂²P/∂S²)
    pub gamma: f64,
    /// Vega: sensitivity to volatility (∂P/∂σ), per 1% vol
    pub vega: f64,
    /// Theta: time decay (∂P/∂t), per day
    pub theta: f64,
    /// Rho: sensitivity to interest rate (∂P/∂r), per 1% rate
    pub rho: f64,
}

/// Compute all call Greeks at once.
pub fn bs_call_greeks(
    spot: f64,
    strike: f64,
    time: f64,
    rate: f64,
    div_yield: f64,
    vol: f64,
) -> CallGreeks {
    CallGreeks {
        delta: bs_call_delta(spot, strike, time, rate, div_yield, vol),
        gamma: bs_gamma(spot, strike, time, rate, div_yield, vol),
        vega: bs_vega(spot, strike, time, rate, div_yield, vol),
        theta: bs_call_theta(spot, strike, time, rate, div_yield, vol),
        rho: bs_call_rho(spot, strike, time, rate, div_yield, vol),
    }
}

/// Compute all put Greeks at once.
pub fn bs_put_greeks(
    spot: f64,
    strike: f64,
    time: f64,
    rate: f64,
    div_yield: f64,
    vol: f64,
) -> PutGreeks {
    PutGreeks {
        delta: bs_put_delta(spot, strike, time, rate, div_yield, vol),
        gamma: bs_gamma(spot, strike, time, rate, div_yield, vol),
        vega: bs_vega(spot, strike, time, rate, div_yield, vol),
        theta: bs_put_theta(spot, strike, time, rate, div_yield, vol),
        rho: bs_put_rho(spot, strike, time, rate, div_yield, vol),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_call_delta_atm() {
        let delta = bs_call_delta(100.0, 100.0, 1.0, 0.05, 0.02, 0.2);
        // ATM delta should be around 0.5-0.6
        assert!(delta > 0.4 && delta < 0.7);
    }

    #[test]
    fn test_put_delta_atm() {
        let delta = bs_put_delta(100.0, 100.0, 1.0, 0.05, 0.02, 0.2);
        // ATM put delta should be negative, around -0.4 to -0.5
        assert!(delta < 0.0 && delta > -0.7);
    }

    #[test]
    fn test_gamma_positive() {
        let gamma = bs_gamma(100.0, 100.0, 1.0, 0.05, 0.02, 0.2);
        // Gamma should always be positive
        assert!(gamma > 0.0);
    }

    #[test]
    fn test_vega_positive() {
        let vega = bs_vega(100.0, 100.0, 1.0, 0.05, 0.02, 0.2);
        // Vega should always be positive
        assert!(vega > 0.0);
    }

    #[test]
    fn test_put_call_delta_parity() {
        let call_delta = bs_call_delta(100.0, 100.0, 1.0, 0.05, 0.02, 0.2);
        let put_delta = bs_put_delta(100.0, 100.0, 1.0, 0.05, 0.02, 0.2);

        // Delta parity: Δ_call - Δ_put = exp(-qT)
        let lhs = call_delta - put_delta;
        let rhs = (-0.02_f64 * 1.0).exp();

        assert!(
            (lhs - rhs).abs() < 0.001,
            "Delta parity failed: {} vs {}",
            lhs,
            rhs
        );
    }
}
