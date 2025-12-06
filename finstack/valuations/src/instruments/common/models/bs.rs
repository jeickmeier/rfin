//! Shared Black–Scholes/Garman–Kohlhagen pricing helpers.
use crate::instruments::common::models::{d1, d2};
use crate::instruments::common::parameters::OptionType;

/// Conversion constant for per-1% greeks.
pub const ONE_PERCENT: f64 = 100.0;

/// Black–Scholes/Garman–Kohlhagen greeks (per unit, not scaled by contract size).
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

/// Black–Scholes / Garman–Kohlhagen price (per unit, no contract scaling).
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

/// Black–Scholes / Garman–Kohlhagen greeks (per unit, per-1% for vega and rhos).
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

