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

use crate::instruments::common::models::volatility::black::d1_d2;
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

impl BsGreeks {
    /// Validate that Greeks are within expected bounds.
    ///
    /// Returns `true` if all Greeks satisfy their theoretical constraints:
    /// - Delta: must be in [-1, 1] (calls in [0, 1], puts in [-1, 0])
    /// - Gamma: must be non-negative (≥ 0)
    /// - Vega: must be non-negative (≥ 0)
    ///
    /// Theta and rhos have no strict sign constraints (can be positive or negative
    /// depending on option moneyness and rate environment).
    #[must_use]
    pub fn is_valid(&self) -> bool {
        // Delta must be in [-1, 1]
        if !(-1.0..=1.0).contains(&self.delta) {
            return false;
        }
        // Gamma must be non-negative
        if self.gamma < 0.0 {
            return false;
        }
        // Vega must be non-negative
        if self.vega < 0.0 {
            return false;
        }
        // All values must be finite
        self.delta.is_finite()
            && self.gamma.is_finite()
            && self.vega.is_finite()
            && self.theta.is_finite()
            && self.rho_r.is_finite()
            && self.rho_q.is_finite()
    }

    /// Clamp Greeks to their valid bounds.
    ///
    /// This corrects for minor numerical precision issues near boundaries:
    /// - Delta: clamped to [-1, 1]
    /// - Gamma: clamped to [0, ∞)
    /// - Vega: clamped to [0, ∞)
    ///
    /// Theta and rhos are not clamped as they have no theoretical bounds.
    #[must_use]
    pub fn clamped(self) -> Self {
        Self {
            delta: self.delta.clamp(-1.0, 1.0),
            gamma: self.gamma.max(0.0),
            vega: self.vega.max(0.0),
            theta: self.theta,
            rho_r: self.rho_r,
            rho_q: self.rho_q,
        }
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

    // Use combined d1_d2 to avoid redundant computation
    let (d1, d2) = d1_d2(spot, strike, r, sigma, t, q);

    // Compute CDFs - use symmetry N(-x) = 1 - N(x) to reduce calls
    let cdf_d1 = finstack_core::math::norm_cdf(d1);
    let cdf_d2 = finstack_core::math::norm_cdf(d2);

    let exp_q_t = (-q * t).exp();
    let exp_r_t = (-r * t).exp();

    match option_type {
        OptionType::Call => spot * exp_q_t * cdf_d1 - strike * exp_r_t * cdf_d2,
        OptionType::Put => {
            // Use symmetry: N(-x) = 1 - N(x)
            let cdf_m_d1 = 1.0 - cdf_d1;
            let cdf_m_d2 = 1.0 - cdf_d2;
            strike * exp_r_t * cdf_m_d2 - spot * exp_q_t * cdf_m_d1
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
    // Use combined d1_d2 to compute both values in one pass (avoids duplicate ln/sqrt)
    let (d1, d2) = d1_d2(spot, strike, r, sigma, t, q);

    // Pre-compute shared exponentials
    let exp_q_t = (-q * t).exp();
    let exp_r_t = (-r * t).exp();
    let sqrt_t = t.sqrt();

    // PDF is always needed for gamma/vega/theta
    let pdf_d1 = finstack_core::math::norm_pdf(d1);

    // Compute CDFs only twice - use symmetry N(-x) = 1 - N(x) for the complements
    let cdf_d1 = finstack_core::math::norm_cdf(d1);
    let cdf_d2 = finstack_core::math::norm_cdf(d2);
    let cdf_m_d1 = 1.0 - cdf_d1; // N(-d1) = 1 - N(d1)
    let cdf_m_d2 = 1.0 - cdf_d2; // N(-d2) = 1 - N(d2)

    let delta = match option_type {
        OptionType::Call => exp_q_t * cdf_d1,
        OptionType::Put => -exp_q_t * cdf_m_d1,
    };

    // Gamma is the same for calls and puts
    let gamma = if sigma <= 0.0 || sqrt_t <= 0.0 {
        0.0
    } else {
        exp_q_t * pdf_d1 / (spot * sigma * sqrt_t)
    };

    // Vega is the same for calls and puts (per 1% vol)
    let vega = spot * exp_q_t * pdf_d1 * sqrt_t / ONE_PERCENT;

    // Theta differs by option type
    // Common term for both: -S * φ(d1) * σ * e^(-qT) / (2√T)
    let theta_common = if sqrt_t > 0.0 {
        -spot * pdf_d1 * sigma * exp_q_t / (2.0 * sqrt_t)
    } else {
        0.0
    };

    let theta = match option_type {
        OptionType::Call => {
            let term2 = q * spot * cdf_d1 * exp_q_t;
            let term3 = -r * strike * exp_r_t * cdf_d2;
            (theta_common + term2 + term3) / theta_days_per_year
        }
        OptionType::Put => {
            let term2 = -q * spot * cdf_m_d1 * exp_q_t;
            let term3 = r * strike * exp_r_t * cdf_m_d2;
            (theta_common + term2 + term3) / theta_days_per_year
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

    #[test]
    fn test_bs_greeks_is_valid() {
        // Normal ATM call should be valid
        let greeks = bs_greeks(100.0, 100.0, 0.05, 0.02, 0.20, 1.0, OptionType::Call, 365.0);
        assert!(greeks.is_valid(), "ATM call Greeks should be valid");

        // Normal ATM put should be valid
        let put_greeks = bs_greeks(100.0, 100.0, 0.05, 0.02, 0.20, 1.0, OptionType::Put, 365.0);
        assert!(put_greeks.is_valid(), "ATM put Greeks should be valid");

        // Deep ITM call should still be valid
        let deep_itm = bs_greeks(200.0, 100.0, 0.05, 0.02, 0.20, 0.01, OptionType::Call, 365.0);
        assert!(deep_itm.is_valid(), "Deep ITM call Greeks should be valid");

        // Deep OTM put should still be valid
        let deep_otm = bs_greeks(200.0, 100.0, 0.05, 0.02, 0.20, 0.01, OptionType::Put, 365.0);
        assert!(deep_otm.is_valid(), "Deep OTM put Greeks should be valid");
    }

    #[test]
    fn test_bs_greeks_clamped() {
        // Create Greeks with slightly out-of-bounds values (simulating numerical noise)
        let greeks = BsGreeks {
            delta: 1.0000001, // Slightly above 1.0
            gamma: -0.0000001, // Slightly negative
            vega: -0.0000001, // Slightly negative
            theta: -0.05,
            rho_r: 0.5,
            rho_q: -0.3,
        };

        let clamped = greeks.clamped();
        assert_eq!(clamped.delta, 1.0);
        assert_eq!(clamped.gamma, 0.0);
        assert_eq!(clamped.vega, 0.0);
        assert_eq!(clamped.theta, -0.05); // Unchanged
        assert_eq!(clamped.rho_r, 0.5); // Unchanged
        assert_eq!(clamped.rho_q, -0.3); // Unchanged
        assert!(clamped.is_valid());
    }

    #[test]
    fn test_bs_greeks_delta_bounds() {
        // Test that delta stays in [-1, 1] for extreme cases
        let cases = [
            // (spot, strike, option_type, expected_delta_sign)
            (1000.0, 100.0, OptionType::Call, 1), // Deep ITM call → delta ≈ 1
            (10.0, 100.0, OptionType::Call, 1),   // Deep OTM call → delta ≈ 0
            (1000.0, 100.0, OptionType::Put, -1), // Deep OTM put → delta ≈ 0
            (10.0, 100.0, OptionType::Put, -1),   // Deep ITM put → delta ≈ -1
        ];

        for (spot, strike, opt_type, expected_sign) in cases {
            let greeks = bs_greeks(spot, strike, 0.05, 0.02, 0.20, 1.0, opt_type, 365.0);
            assert!(
                greeks.is_valid(),
                "Greeks should be valid for spot={}, strike={}, type={:?}",
                spot,
                strike,
                opt_type
            );
            if expected_sign > 0 {
                assert!(greeks.delta >= 0.0);
            } else {
                assert!(greeks.delta <= 0.0);
            }
        }
    }
}

