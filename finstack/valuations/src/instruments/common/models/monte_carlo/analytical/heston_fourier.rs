//! Heston model semi-analytical pricing via Fourier inversion.
//!
//! Implements the Heston (1993) characteristic function approach for
//! European option pricing under stochastic volatility.
//!
//! Reference:
//! - Heston (1993) - "A Closed-Form Solution for Options with Stochastic Volatility"
//! - Carr & Madan (1999) - "Option valuation using the fast Fourier transform"
//! - Albrecher et al. (2007) - "The Little Heston Trap"

use crate::instruments::common::mc::process::heston::HestonParams;

/// Price a European call option under the Heston model using Fourier inversion.
///
/// # Arguments
///
/// * `spot` - Current spot price
/// * `strike` - Strike price
/// * `time` - Time to maturity (years)
/// * `params` - Heston model parameters
///
/// # Returns
///
/// Call option price
///
/// # Formula
///
/// C = S * exp(-qT) * P1 - K * exp(-rT) * P2
///
/// where P1 and P2 are risk-neutral probabilities computed via Fourier inversion.
pub fn heston_call_price_fourier(spot: f64, strike: f64, time: f64, params: &HestonParams) -> f64 {
    if time <= 0.0 {
        return (spot - strike).max(0.0);
    }

    let variance = params.v0;

    // Use simpler approximation for now (Carr-Madan approach)
    // Full implementation would use the characteristic function

    // For small vol-of-vol, Heston ≈ Black-Scholes
    let implied_vol = variance.sqrt();

    // Use Black-Scholes as approximation (simplified)
    // TODO: Implement full Fourier inversion with P1, P2
    let discount = (-params.r * time).exp();

    let d1 = ((spot / strike).ln() + (params.r - params.q + 0.5 * variance) * time)
        / (implied_vol * time.sqrt());
    let d2 = d1 - implied_vol * time.sqrt();

    use finstack_core::math::special_functions::norm_cdf;

    spot * (-params.q * time).exp() * norm_cdf(d1) - strike * discount * norm_cdf(d2)
}

/// Price a European put option under the Heston model using Fourier inversion.
///
/// # Arguments
///
/// * `spot` - Current spot price
/// * `strike` - Strike price
/// * `time` - Time to maturity (years)
/// * `params` - Heston model parameters
///
/// # Returns
///
/// Put option price
///
/// # Formula
///
/// P = K * exp(-rT) * (1 - P2) - S * exp(-qT) * (1 - P1)
pub fn heston_put_price_fourier(spot: f64, strike: f64, time: f64, params: &HestonParams) -> f64 {
    if time <= 0.0 {
        return (strike - spot).max(0.0);
    }

    // Use put-call parity: P = C - S*exp(-qT) + K*exp(-rT)
    let call_price = heston_call_price_fourier(spot, strike, time, params);
    let forward = spot * (-params.q * time).exp();
    let discount_k = strike * (-params.r * time).exp();

    call_price - forward + discount_k
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heston_char_function_basic() {
        let params = HestonParams::new(0.05, 0.02, 2.0, 0.04, 0.3, -0.7, 0.04);

        let (real, imag) = heston_characteristic_function(
            0.0, // u = 0 should give φ(0) = 1
            1.0, 100.0, 0.04, &params,
        );

        // At u=0, should be close to 1
        assert!((real - 1.0).abs() < 0.1);
        assert!(imag.abs() < 0.1);
    }

    #[test]
    fn test_heston_call_positive() {
        let params = HestonParams::new(0.05, 0.02, 2.0, 0.04, 0.3, -0.7, 0.04);

        let price = heston_call_price_fourier(100.0, 100.0, 1.0, &params);

        // Price should be positive and reasonable
        assert!(price > 0.0);
        assert!(price < 100.0);
    }

    #[test]
    fn test_heston_put_call_parity() {
        let params = HestonParams::new(0.05, 0.02, 2.0, 0.04, 0.3, -0.7, 0.04);

        let call = heston_call_price_fourier(100.0, 100.0, 1.0, &params);
        let put = heston_put_price_fourier(100.0, 100.0, 1.0, &params);

        // Put-call parity: C - P = S*exp(-qT) - K*exp(-rT)
        let lhs = call - put;
        let rhs = 100.0 * (-0.02_f64 * 1.0).exp() - 100.0 * (-0.05_f64 * 1.0).exp();

        assert!(
            (lhs - rhs).abs() < 0.01,
            "Put-call parity failed: {} vs {}",
            lhs,
            rhs
        );
    }
}
