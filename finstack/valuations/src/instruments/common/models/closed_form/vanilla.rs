//! Black–Scholes/Garman–Kohlhagen vanilla option pricing and Greeks.
//!
//! This module provides closed-form pricing and Greeks for European vanilla options
//! using the Black-Scholes-Merton (equity) or Garman-Kohlhagen (FX) framework.
//!
//! # Features
//!
//! - **`bs_price`**: Computes the fair value of a European call or put
//! - **`bs_greeks`**: Computes all first-order Greeks (delta, gamma, vega, theta, rho_r, rho_q)
//! - **`BsGreeks`**: Struct holding per-unit Greeks with both domestic and foreign rho
//!
//! # Model
//!
//! The pricing formula uses continuous compounding with dividend yield (or foreign rate for FX):
//! ```text
//! Call = S·e^(-qT)·N(d₁) - K·e^(-rT)·N(d₂)
//! Put  = K·e^(-rT)·N(-d₂) - S·e^(-qT)·N(-d₁)
//! ```
//!
//! where:
//! - `r` is the domestic (risk-free) rate
//! - `q` is the dividend yield (or foreign rate for FX options)
//!
//! # References
//!
//! - Black, F., & Scholes, M. (1973). "The Pricing of Options and Corporate Liabilities."
//! - Garman, M. B., & Kohlhagen, S. W. (1983). "Foreign Currency Option Values."

use crate::instruments::common::models::volatility::black::{d1, d2};
use crate::instruments::common::parameters::OptionType;
use std::fmt;

/// Conversion constant for per-1% Greeks.
pub const ONE_PERCENT: f64 = 100.0;

/// Black–Scholes/Garman–Kohlhagen Greeks (per unit, not scaled by contract size).
///
/// This struct is suitable for both equity options (with dividend yield) and
/// FX options (with foreign rate), as it includes both `rho_r` (domestic) and
/// `rho_q` (foreign/dividend) sensitivities.
#[derive(Clone, Copy, Debug, Default)]
pub struct BsGreeks {
    /// Delta sensitivity per unit.
    pub delta: f64,
    /// Gamma sensitivity per unit.
    pub gamma: f64,
    /// Vega per 1% volatility move.
    pub vega: f64,
    /// Theta per day (scaled by provided day-count basis).
    pub theta: f64,
    /// Rho to the domestic/risk-free rate per 1%.
    pub rho_r: f64,
    /// Rho to the foreign/dividend yield per 1%.
    pub rho_q: f64,
}

impl fmt::Display for BsGreeks {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Δ={:.4} Γ={:.6} V={:.4} Θ={:.4} ρr={:.4} ρq={:.4}",
            self.delta, self.gamma, self.vega, self.theta, self.rho_r, self.rho_q
        )
    }
}

/// Black–Scholes / Garman–Kohlhagen price (per unit, no contract scaling).
///
/// # Arguments
///
/// * `spot` - Current spot price S
/// * `strike` - Strike price K
/// * `r` - Domestic (risk-free) rate, continuously compounded
/// * `q` - Dividend yield or foreign rate, continuously compounded
/// * `sigma` - Volatility σ (annualized)
/// * `t` - Time to expiration T (in years)
/// * `option_type` - Call or Put
///
/// # Returns
///
/// Option price per unit of the underlying. At expiration (t ≤ 0), returns intrinsic value.
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::instruments::common::models::closed_form::vanilla::bs_price;
/// use finstack_valuations::instruments::common::parameters::OptionType;
///
/// let price = bs_price(100.0, 100.0, 0.05, 0.02, 0.20, 1.0, OptionType::Call);
/// assert!(price > 0.0);
/// ```
#[must_use]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn bs_price(
    spot: f64,
    strike: f64,
    r: f64,
    q: f64,
    sigma: f64,
    t: f64,
    option_type: OptionType,
) -> f64 {
    if t <= 0.0 {
        return match option_type {
            OptionType::Call => (spot - strike).max(0.0),
            OptionType::Put => (strike - spot).max(0.0),
        };
    }
    let d1 = d1(spot, strike, r, sigma, t, q);
    let d2 = d2(spot, strike, r, sigma, t, q);
    match option_type {
        OptionType::Call => {
            spot * (-q * t).exp() * finstack_core::math::norm_cdf(d1)
                - strike * (-r * t).exp() * finstack_core::math::norm_cdf(d2)
        }
        OptionType::Put => {
            strike * (-r * t).exp() * finstack_core::math::norm_cdf(-d2)
                - spot * (-q * t).exp() * finstack_core::math::norm_cdf(-d1)
        }
    }
}

/// Black–Scholes / Garman–Kohlhagen Greeks (per unit, per-1% for vega and rhos).
///
/// Computes all first-order sensitivities for European vanilla options.
///
/// # Arguments
///
/// * `spot` - Current spot price S
/// * `strike` - Strike price K
/// * `r` - Domestic (risk-free) rate, continuously compounded
/// * `q` - Dividend yield or foreign rate, continuously compounded
/// * `sigma` - Volatility σ (annualized)
/// * `t` - Time to expiration T (in years)
/// * `option_type` - Call or Put
/// * `theta_days_per_year` - Day-count basis for theta (e.g., 365.0 for ACT/365)
///
/// # Returns
///
/// [`BsGreeks`] struct with:
/// - `delta`: ∂V/∂S (per unit)
/// - `gamma`: ∂²V/∂S² (per unit)
/// - `vega`: ∂V/∂σ per 1% vol change
/// - `theta`: ∂V/∂t per day
/// - `rho_r`: ∂V/∂r per 1% domestic rate change
/// - `rho_q`: ∂V/∂q per 1% foreign/dividend rate change
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::instruments::common::models::closed_form::vanilla::{bs_greeks, BsGreeks};
/// use finstack_valuations::instruments::common::parameters::OptionType;
///
/// let greeks = bs_greeks(100.0, 100.0, 0.05, 0.02, 0.20, 1.0, OptionType::Call, 365.0);
/// assert!(greeks.delta > 0.0 && greeks.delta < 1.0); // Call delta in (0, 1)
/// assert!(greeks.gamma > 0.0); // Gamma always positive
/// assert!(greeks.vega > 0.0);  // Vega always positive
/// ```
#[must_use]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn bs_greeks(
    spot: f64,
    strike: f64,
    r: f64,
    q: f64,
    sigma: f64,
    t: f64,
    option_type: OptionType,
    theta_days_per_year: f64,
) -> BsGreeks {
    let d1 = d1(spot, strike, r, sigma, t, q);
    let d2 = d2(spot, strike, r, sigma, t, q);
    let exp_q_t = (-q * t).exp();
    let exp_r_t = (-r * t).exp();
    let sqrt_t = t.sqrt();
    let pdf_d1 = finstack_core::math::norm_pdf(d1);
    let cdf_d1 = finstack_core::math::norm_cdf(d1);
    let cdf_m_d1 = finstack_core::math::norm_cdf(-d1);
    let cdf_d2 = finstack_core::math::norm_cdf(d2);
    let cdf_m_d2 = finstack_core::math::norm_cdf(-d2);

    let delta = match option_type {
        OptionType::Call => exp_q_t * cdf_d1,
        OptionType::Put => -exp_q_t * cdf_m_d1,
    };
    let gamma = if sigma <= 0.0 {
        0.0
    } else {
        exp_q_t * pdf_d1 / (spot * sigma * sqrt_t)
    };
    let vega = spot * exp_q_t * pdf_d1 * sqrt_t / ONE_PERCENT; // per 1% vol
    let theta = match option_type {
        OptionType::Call => {
            let term1 = -spot * pdf_d1 * sigma * exp_q_t / (2.0 * sqrt_t);
            let term2 = q * spot * cdf_d1 * exp_q_t;
            let term3 = -r * strike * exp_r_t * cdf_d2;
            (term1 + term2 + term3) / theta_days_per_year
        }
        OptionType::Put => {
            let term1 = -spot * pdf_d1 * sigma * exp_q_t / (2.0 * sqrt_t);
            let term2 = -q * spot * cdf_m_d1 * exp_q_t;
            let term3 = r * strike * exp_r_t * cdf_m_d2;
            (term1 + term2 + term3) / theta_days_per_year
        }
    };
    let rho_r = match option_type {
        OptionType::Call => strike * t * exp_r_t * cdf_d2 / ONE_PERCENT,
        OptionType::Put => -strike * t * exp_r_t * cdf_m_d2 / ONE_PERCENT,
    };
    let rho_q = match option_type {
        OptionType::Call => -spot * t * exp_q_t * cdf_d1 / ONE_PERCENT,
        OptionType::Put => spot * t * exp_q_t * cdf_m_d1 / ONE_PERCENT,
    };

    BsGreeks {
        delta,
        gamma,
        vega,
        theta,
        rho_r,
        rho_q,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bs_price_call_atm() {
        let price = bs_price(100.0, 100.0, 0.05, 0.02, 0.20, 1.0, OptionType::Call);
        // ATM call with these params should be around 9-10
        assert!(price > 8.0 && price < 12.0, "price = {}", price);
    }

    #[test]
    fn test_bs_price_put_atm() {
        let price = bs_price(100.0, 100.0, 0.05, 0.02, 0.20, 1.0, OptionType::Put);
        // Put-call parity check
        let call = bs_price(100.0, 100.0, 0.05, 0.02, 0.20, 1.0, OptionType::Call);
        let parity = call - price - 100.0 * (-0.02_f64).exp() + 100.0 * (-0.05_f64).exp();
        assert!(parity.abs() < 1e-10, "Put-call parity violated: {}", parity);
    }

    #[test]
    fn test_bs_price_expired() {
        // ITM call at expiration
        assert!((bs_price(110.0, 100.0, 0.05, 0.0, 0.2, 0.0, OptionType::Call) - 10.0).abs() < 1e-10);
        // OTM call at expiration
        assert!(bs_price(90.0, 100.0, 0.05, 0.0, 0.2, 0.0, OptionType::Call).abs() < 1e-10);
        // ITM put at expiration
        assert!((bs_price(90.0, 100.0, 0.05, 0.0, 0.2, 0.0, OptionType::Put) - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_bs_greeks_call() {
        let greeks = bs_greeks(100.0, 100.0, 0.05, 0.02, 0.20, 1.0, OptionType::Call, 365.0);
        // ATM call delta should be around 0.5-0.6
        assert!(greeks.delta > 0.4 && greeks.delta < 0.7, "delta = {}", greeks.delta);
        // Gamma always positive
        assert!(greeks.gamma > 0.0);
        // Vega always positive
        assert!(greeks.vega > 0.0);
    }

    #[test]
    fn test_bs_greeks_put() {
        let greeks = bs_greeks(100.0, 100.0, 0.05, 0.02, 0.20, 1.0, OptionType::Put, 365.0);
        // ATM put delta should be negative, around -0.4 to -0.5
        assert!(greeks.delta < 0.0 && greeks.delta > -0.7, "delta = {}", greeks.delta);
        // Gamma same for calls and puts
        let call_greeks = bs_greeks(100.0, 100.0, 0.05, 0.02, 0.20, 1.0, OptionType::Call, 365.0);
        assert!((greeks.gamma - call_greeks.gamma).abs() < 1e-10);
    }

    #[test]
    fn test_bs_greeks_display() {
        let greeks = bs_greeks(100.0, 100.0, 0.05, 0.02, 0.20, 1.0, OptionType::Call, 365.0);
        let s = format!("{}", greeks);
        assert!(s.contains("Δ="));
        assert!(s.contains("Γ="));
        assert!(s.contains("V="));
    }
}

