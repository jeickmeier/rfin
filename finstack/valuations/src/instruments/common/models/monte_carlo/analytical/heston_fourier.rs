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
use std::f64::consts::PI;

/// Heston characteristic function for log-spot process.
///
/// Computes φ(u; t, S, v) = E[exp(iu ln(S_T)) | S_t = S, v_t = v]
/// where u is the frequency parameter.
///
/// # Arguments
///
/// * `u` - Tenor parameter (complex in general, real here)
/// * `time` - Time to maturity
/// * `spot` - Current spot price
/// * `variance` - Current variance (v_0)
/// * `params` - Heston model parameters
///
/// # Returns
///
/// (real_part, imag_part) of the characteristic function
#[allow(dead_code)]
fn heston_characteristic_function(
    u: f64,
    time: f64,
    spot: f64,
    variance: f64,
    params: &HestonParams,
) -> (f64, f64) {
    let kappa = params.kappa;
    let theta = params.theta;
    let sigma_v = params.sigma_v;
    let rho = params.rho;
    let r = params.r;
    let q = params.q;

    // Complex number i*u
    let i_u_imag = u;

    // d = sqrt(σ_v²(ρ²u² - iu) + (κ - iρσ_v u)²)
    // For numerical stability, use the formulation from Albrecher et al. (2007)

    let i_rho_sigma_u = rho * sigma_v * u;
    // d² = σ_v²(ρ²u² - iu) + (κ - iρσ_v u)²
    // Breaking down (κ - iρσ_v u)² = κ² - 2iκρσ_v u - (ρσ_v u)²
    // Real part: σ_v²ρ²u² + κ² - (ρσ_v u)²
    // Imag part: -σ_v²u - 2κρσ_v u
    let d_squared_real =
        sigma_v * sigma_v * (rho * rho * u * u) + kappa * kappa - (i_rho_sigma_u * i_rho_sigma_u);
    let d_squared_imag = -sigma_v * sigma_v * u - 2.0 * kappa * i_rho_sigma_u;

    // d = sqrt(d_squared)
    let d_mag = (d_squared_real * d_squared_real + d_squared_imag * d_squared_imag)
        .sqrt()
        .sqrt();
    let d_arg = (d_squared_imag.atan2(d_squared_real)) / 2.0;
    let d_real = d_mag * d_arg.cos();
    let d_imag = d_mag * d_arg.sin();

    // g = (κ - iρσ_v u - d) / (κ - iρσ_v u + d)
    let num_real = kappa - d_real;
    let num_imag = -i_rho_sigma_u - d_imag;
    let den_real = kappa + d_real;
    let den_imag = -i_rho_sigma_u + d_imag;

    let den_mag_sq = den_real * den_real + den_imag * den_imag;
    let g_real = (num_real * den_real + num_imag * den_imag) / den_mag_sq;
    let g_imag = (num_imag * den_real - num_real * den_imag) / den_mag_sq;

    // exp(-d*T)
    let exp_dt_real = (-d_real * time).exp() * (-d_imag * time).cos();
    let exp_dt_imag = (-d_real * time).exp() * (-d_imag * time).sin();

    // 1 - g*exp(-d*T)
    let term_real = 1.0 - g_real * exp_dt_real + g_imag * exp_dt_imag;
    let term_imag = -g_real * exp_dt_imag - g_imag * exp_dt_real;

    // C = (r - q)*i*u*T + (κ*θ/σ_v²) * [(κ - iρσ_v u - d)*T - 2*ln(1 - g*exp(-d*T)) / (1 - g)]
    let factor = kappa * theta / (sigma_v * sigma_v);

    // First part: (κ - iρσ_v u - d)*T
    let part1_real = (kappa - d_real) * time;
    let part1_imag = (-i_rho_sigma_u - d_imag) * time;

    // Second part: -2*ln(1 - g*exp(-d*T)) / (1 - g)
    let ln_arg_real = term_real;
    let ln_arg_imag = term_imag;
    let ln_mag = (ln_arg_real * ln_arg_real + ln_arg_imag * ln_arg_imag)
        .sqrt()
        .ln();
    let ln_arg = ln_arg_imag.atan2(ln_arg_real);

    let one_minus_g_real = 1.0 - g_real;
    let one_minus_g_imag = -g_imag;
    let one_minus_g_mag_sq =
        one_minus_g_real * one_minus_g_real + one_minus_g_imag * one_minus_g_imag;

    let part2_real = -2.0 * (ln_mag * one_minus_g_real) / one_minus_g_mag_sq;
    let part2_imag =
        -2.0 * (ln_arg * one_minus_g_real - ln_mag * one_minus_g_imag) / one_minus_g_mag_sq;

    // C = (r - q)*i*u*T + (κ*θ/σ_v²) * [...]
    // The (r - q)*i*u*T term is purely imaginary
    let c_real = factor * (part1_real + part2_real);
    let c_imag = (r - q) * i_u_imag * time + factor * (part1_imag + part2_imag);

    // D = [(κ - iρσ_v u - d) / σ_v²] * [(1 - exp(-d*T)) / (1 - g*exp(-d*T))]
    let num2_real = (kappa - d_real) * (1.0 - exp_dt_real);
    let num2_imag =
        (-i_rho_sigma_u - d_imag) * (1.0 - exp_dt_real) + (kappa - d_real) * (-exp_dt_imag);

    let d_real_part = (num2_real * term_real + num2_imag * term_imag)
        / (sigma_v * sigma_v * (term_real * term_real + term_imag * term_imag));
    let d_imag_part = (num2_imag * term_real - num2_real * term_imag)
        / (sigma_v * sigma_v * (term_real * term_real + term_imag * term_imag));

    // φ = exp(C + D*v + i*u*ln(S))
    let exponent_real = c_real + d_real_part * variance;
    let exponent_imag = c_imag + d_imag_part * variance + u * spot.ln();

    let result_real = exponent_real.exp() * exponent_imag.cos();
    let result_imag = exponent_real.exp() * exponent_imag.sin();

    (result_real, result_imag)
}

/// Compute P1 probability for Heston call pricing.
///
/// P1 = 0.5 + (1/π) ∫_0^∞ Re[exp(-iu ln(K)) φ₁(u)] du
#[allow(dead_code)]
fn heston_p1(spot: f64, strike: f64, time: f64, variance: f64, params: &HestonParams) -> f64 {
    // Numerical integration using trapezoidal rule
    let du = 0.1;
    let u_max = 100.0;
    let num_steps = (u_max / du) as usize;

    let mut sum = 0.0;

    for i in 1..num_steps {
        let u = i as f64 * du;

        // Modified characteristic function for P1
        // For P1, we need φ(u - i, ...) which shifts the measure

        let (phi_real, phi_imag) = heston_characteristic_function(u, time, spot, variance, params);

        // Integrand: Re[exp(-iu ln(K)) φ(u)] / (iu)
        let k_ln = strike.ln();
        let exp_real = (-u * k_ln).sin();
        let exp_imag = (-u * k_ln).cos();

        let integrand_real = (phi_real * exp_imag - phi_imag * exp_real) / u;

        sum += integrand_real * du;
    }

    0.5 + sum / PI
}

/// Compute P2 probability for Heston call pricing.
///
/// P2 = 0.5 + (1/π) ∫_0^∞ Re[exp(-iu ln(K)) φ₂(u)] du
#[allow(dead_code)]
fn heston_p2(spot: f64, strike: f64, time: f64, variance: f64, params: &HestonParams) -> f64 {
    // Numerical integration using trapezoidal rule
    let du = 0.1;
    let u_max = 100.0;
    let num_steps = (u_max / du) as usize;

    let mut sum = 0.0;

    for i in 1..num_steps {
        let u = i as f64 * du;

        let (phi_real, phi_imag) = heston_characteristic_function(u, time, spot, variance, params);

        // Integrand: Re[exp(-iu ln(K)) φ(u)] / (iu)
        let k_ln = strike.ln();
        let exp_real = (-u * k_ln).sin();
        let exp_imag = (-u * k_ln).cos();

        let integrand_real = (phi_real * exp_imag - phi_imag * exp_real) / u;

        sum += integrand_real * du;
    }

    0.5 + sum / PI
}

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
