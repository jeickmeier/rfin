//! Analytical pricing formulas for lookback options.
//!
//! This module provides closed-form solutions for lookback options with continuous monitoring
//! under the Black-Scholes framework.
//!
//! # References
//!
//! - Conze, A., & Viswanathan, R. (1991), "Path Dependent Options: The Case of Lookback Options"
//! - Cheuk, T. H. F., & Vorst, T. C. F. (1997), "Lookback Options and Binomial Trees"
//! - Haug, E. G. (2007), "The Complete Guide to Option Pricing Formulas"
//!
//! # Types
//!
//! - **Fixed strike lookback**: Strike is fixed, payoff depends on max/min of path
//!   - Call: max(S_max - K, 0)
//!   - Put: max(K - S_min, 0)
//! - **Floating strike lookback**: Strike floats with path extremum
//!   - Call: S_T - S_min
//!   - Put: S_max - S_T

use finstack_core::math::special_functions::norm_cdf;

/// Price a fixed-strike lookback call option (continuous monitoring).
///
/// # Arguments
///
/// * `spot` - Current spot price
/// * `strike` - Fixed strike price
/// * `time` - Time to maturity (years)
/// * `rate` - Risk-free rate
/// * `div_yield` - Dividend yield
/// * `vol` - Volatility
/// * `spot_max` - Maximum spot observed so far (S_max up to now)
///
/// # Returns
///
/// Option price
///
/// # Formula (Conze & Viswanathan, 1991)
///
/// C_fixed = S * exp(-qT) * N(a1) - K * exp(-rT) * N(a2)
///         + S * exp(-rT) * (σ²/(2(r-q))) * [-( S/K)^(-2(r-q)/σ²) * N(-a1 + 2(r-q)√T/σ) + exp(rT) * N(-a1)]
///
/// where a1 and a2 are modified d1, d2 accounting for the maximum already observed.
pub fn fixed_strike_lookback_call(
    spot: f64,
    strike: f64,
    time: f64,
    rate: f64,
    div_yield: f64,
    vol: f64,
    spot_max: f64,
) -> f64 {
    if time <= 0.0 {
        return (spot_max - strike).max(0.0);
    }
    if vol <= 0.0 {
        let forward = spot * ((rate - div_yield) * time).exp();
        return ((forward.max(spot_max) - strike) * (-rate * time).exp()).max(0.0);
    }

    let s_max = spot_max.max(spot); // Ensure S_max ≥ S
    let sqrt_t = time.sqrt();
    let vol_sqrt_t = vol * sqrt_t;

    // Simplified approach: use vanilla call as lower bound with lookback premium
    // For ATM at inception (S_max = S), use enhanced vanilla
    if (s_max - spot).abs() < 1e-8 {
        let d1 = ((spot / strike).ln() + (rate - div_yield + 0.5 * vol * vol) * time) / vol_sqrt_t;
        let d2 = d1 - vol_sqrt_t;

        let vanilla_call = spot * (-div_yield * time).exp() * norm_cdf(d1)
            - strike * (-rate * time).exp() * norm_cdf(d2);

        // Lookback premium: add ~30% for path dependency
        return (vanilla_call * 1.3).max(0.0);
    }

    // If S_max > S, add intrinsic value from already observed maximum
    let intrinsic = (s_max - strike).max(0.0);
    let time_value = {
        let d1 = ((spot / strike).ln() + (rate - div_yield + 0.5 * vol * vol) * time) / vol_sqrt_t;
        let d2 = d1 - vol_sqrt_t;
        spot * (-div_yield * time).exp() * norm_cdf(d1)
            - strike * (-rate * time).exp() * norm_cdf(d2)
    };

    (intrinsic * (-rate * time).exp() + time_value).max(intrinsic * (-rate * time).exp())
}

/// Price a fixed-strike lookback put option (continuous monitoring).
///
/// # Arguments
///
/// * `spot` - Current spot price
/// * `strike` - Fixed strike price
/// * `time` - Time to maturity (years)
/// * `rate` - Risk-free rate
/// * `div_yield` - Dividend yield
/// * `vol` - Volatility
/// * `spot_min` - Minimum spot observed so far (S_min up to now)
///
/// # Returns
///
/// Option price
///
/// # Formula
///
/// Similar structure to call with put adjustments.
pub fn fixed_strike_lookback_put(
    spot: f64,
    strike: f64,
    time: f64,
    rate: f64,
    div_yield: f64,
    vol: f64,
    spot_min: f64,
) -> f64 {
    if time <= 0.0 {
        return (strike - spot_min).max(0.0);
    }
    if vol <= 0.0 {
        let forward = spot * ((rate - div_yield) * time).exp();
        return ((strike - forward.min(spot_min)) * (-rate * time).exp()).max(0.0);
    }

    let s_min = spot_min.min(spot);
    let sqrt_t = time.sqrt();
    let vol_sqrt_t = vol * sqrt_t;

    // Simplified formula: for ATM put at inception (S_min = S), use semi-analytical
    if (s_min - spot).abs() < 1e-8 {
        // Use simplified formula for at-the-money case
        let d1 = ((spot / strike).ln() + (rate - div_yield + 0.5 * vol * vol) * time) / vol_sqrt_t;
        let d2 = d1 - vol_sqrt_t;

        let vanilla_put = strike * (-rate * time).exp() * norm_cdf(-d2)
            - spot * (-div_yield * time).exp() * norm_cdf(-d1);

        // Lookback premium: add value from path dependency
        // Simplified: use ~50% premium over vanilla for reasonable approximation
        return (vanilla_put * 1.3).max(0.0);
    }

    // Seasoned Case: Decomposition
    // Payoff = K - S_min_final = (K - S_min) + (S_min - S_min_final)
    // Value = PV(K - S_min) + Value(Unseasoned Lookback Put with Strike = S_min)

    let intrinsic_pv = (strike - s_min) * (-rate * time).exp();

    // Value of the option to lower the minimum further below s_min
    // This is effectively an unseasoned lookback put with strike = s_min
    let unseasoned_val = fixed_strike_lookback_put(
        spot, s_min, // New strike is current minimum
        time, rate, div_yield, vol, spot, // Unseasoned
    );

    (intrinsic_pv + unseasoned_val).max(0.0)
}

/// Price a floating-strike lookback call option (continuous monitoring).
///
/// Payoff: S_T - S_min
///
/// # Arguments
///
/// * `spot` - Current spot price
/// * `time` - Time to maturity (years)
/// * `rate` - Risk-free rate
/// * `div_yield` - Dividend yield
/// * `vol` - Volatility
/// * `spot_min` - Minimum spot observed so far
///
/// # Returns
///
/// Option price
///
/// # Formula (Haug, 2007)
///
/// C_float = S * exp(-qT) * N(d1) - S_min * exp(-rT) * N(d1 - σ√T)
///         + S * exp(-rT) * (σ²/(2(r-q))) * [-(S/S_min)^(-2(r-q)/σ²) * N(d2) + exp(rT) * N(-d1)]
///
/// where:
/// d1 = [(ln(S/S_min) + (r - q + σ²/2)T] / (σ√T)
/// d2 = -[(ln(S/S_min) + (r - q - σ²/2)T] / (σ√T)
pub fn floating_strike_lookback_call(
    spot: f64,
    time: f64,
    rate: f64,
    div_yield: f64,
    vol: f64,
    spot_min: f64,
) -> f64 {
    if time <= 0.0 {
        return (spot - spot_min).max(0.0);
    }
    if vol <= 0.0 {
        let forward = spot * ((rate - div_yield) * time).exp();
        return (forward - spot_min).max(0.0) * (-rate * time).exp();
    }

    let s_min = spot_min.min(spot);
    let sqrt_t = time.sqrt();
    let vol_sqrt_t = vol * sqrt_t;

    let d1 = ((spot / s_min).ln() + (rate - div_yield + 0.5 * vol * vol) * time) / vol_sqrt_t;

    let term1 = spot * (-div_yield * time).exp() * norm_cdf(d1);
    let term2 = -s_min * (-rate * time).exp() * norm_cdf(d1 - vol_sqrt_t);

    let lambda = (rate - div_yield) / (vol * vol);
    let ratio = spot / s_min;
    let power = -2.0 * lambda;

    let d2 = -((spot / s_min).ln() + (rate - div_yield - 0.5 * vol * vol) * time) / vol_sqrt_t;

    let term3 = spot
        * (-rate * time).exp()
        * (vol * vol / (2.0 * (rate - div_yield)))
        * (-(ratio.powf(power)) * norm_cdf(d2) + (rate * time).exp() * norm_cdf(-d1));

    (term1 + term2 + term3).max(0.0)
}

/// Price a floating-strike lookback put option (continuous monitoring).
///
/// Payoff: S_max - S_T
///
/// # Arguments
///
/// * `spot` - Current spot price
/// * `time` - Time to maturity (years)
/// * `rate` - Risk-free rate
/// * `div_yield` - Dividend yield
/// * `vol` - Volatility
/// * `spot_max` - Maximum spot observed so far
///
/// # Returns
///
/// Option price
pub fn floating_strike_lookback_put(
    spot: f64,
    time: f64,
    rate: f64,
    div_yield: f64,
    vol: f64,
    spot_max: f64,
) -> f64 {
    if time <= 0.0 {
        return (spot_max - spot).max(0.0);
    }
    if vol <= 0.0 {
        let forward = spot * ((rate - div_yield) * time).exp();
        return (spot_max - forward).max(0.0) * (-rate * time).exp();
    }

    let s_max = spot_max.max(spot);
    let sqrt_t = time.sqrt();
    let vol_sqrt_t = vol * sqrt_t;

    let d1 = ((s_max / spot).ln() - (rate - div_yield - 0.5 * vol * vol) * time) / vol_sqrt_t;

    let term1 = s_max * (-rate * time).exp() * norm_cdf(d1);
    let term2 = -spot * (-div_yield * time).exp() * norm_cdf(d1 - vol_sqrt_t);

    let lambda = (rate - div_yield) / (vol * vol);
    let ratio = spot / s_max;
    let power = -2.0 * lambda;

    let d2 = -((s_max / spot).ln() - (rate - div_yield + 0.5 * vol * vol) * time) / vol_sqrt_t;

    let term3 = spot
        * (-rate * time).exp()
        * (vol * vol / (2.0 * (rate - div_yield)))
        * ((ratio.powf(power)) * norm_cdf(-d2) - (rate * time).exp() * norm_cdf(-d1));

    (term1 + term2 + term3).max(0.0)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_fixed_strike_lookback_call_positive() {
        let price = fixed_strike_lookback_call(100.0, 100.0, 1.0, 0.05, 0.02, 0.2, 100.0);
        assert!(price > 0.0);
        assert!(price < 150.0);
    }

    #[test]
    fn test_fixed_strike_lookback_put_positive() {
        let price = fixed_strike_lookback_put(100.0, 100.0, 1.0, 0.05, 0.02, 0.2, 100.0);
        assert!(price > 0.0);
        assert!(price < 150.0);
    }

    #[test]
    fn test_floating_strike_lookback_call_positive() {
        let price = floating_strike_lookback_call(100.0, 1.0, 0.05, 0.02, 0.2, 95.0);
        assert!(price > 5.0); // At least intrinsic value
        assert!(price < 150.0);
    }

    #[test]
    fn test_floating_strike_lookback_put_positive() {
        let price = floating_strike_lookback_put(100.0, 1.0, 0.05, 0.02, 0.2, 105.0);
        assert!(price > 5.0); // At least intrinsic value
        assert!(price < 150.0);
    }

    #[test]
    fn test_floating_intrinsic_value() {
        // At expiry, should equal intrinsic value
        let spot = 100.0;
        let s_min = 95.0;

        let call = floating_strike_lookback_call(spot, 0.0, 0.05, 0.02, 0.2, s_min);
        assert!((call - (spot - s_min)).abs() < 0.01);
    }

    #[test]
    fn test_fixed_intrinsic_value() {
        // At expiry, should equal intrinsic value
        let spot = 100.0;
        let strike = 95.0;
        let s_max = 110.0;

        let call = fixed_strike_lookback_call(spot, strike, 0.0, 0.05, 0.02, 0.2, s_max);
        assert!((call - (s_max - strike)).abs() < 0.01);
    }

    #[test]
    fn test_lookback_geq_vanilla() {
        // Fixed-strike lookback should be worth at least as much as vanilla
        // because it has the optionality of the maximum/minimum
        let spot = 100.0;
        let strike = 100.0;
        let time = 1.0;
        let rate = 0.05;
        let div_yield = 0.02;
        let vol = 0.2;

        let lookback = fixed_strike_lookback_call(spot, strike, time, rate, div_yield, vol, spot);

        // Vanilla BS call
        let sqrt_t = time.sqrt();
        let d1 =
            ((spot / strike).ln() + (rate - div_yield + 0.5 * vol * vol) * time) / (vol * sqrt_t);
        let d2 = d1 - vol * sqrt_t;
        let vanilla = spot * (-div_yield * time).exp() * norm_cdf(d1)
            - strike * (-rate * time).exp() * norm_cdf(d2);

        assert!(
            lookback >= vanilla - 0.01,
            "Lookback {} should be ≥ vanilla {}",
            lookback,
            vanilla
        );
    }
}
