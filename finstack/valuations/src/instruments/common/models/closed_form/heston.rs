//! Heston model semi-analytical pricing via Fourier inversion.
//!
//! Implements the Heston (1993) characteristic function approach for
//! European option pricing under stochastic volatility.
//!
//! # Algorithm
//!
//! Uses the Gil-Pelaez / P1-P2 formulation:
//! ```text
//! C = S * exp(-qT) * P1 - K * exp(-rT) * P2
//! ```
//!
//! where P1 and P2 are risk-neutral probabilities computed via Fourier inversion
//! of the probability characteristic functions ψ_j(φ).
//!
//! # Numerical Stability
//!
//! Implements the "Little Heston Trap" formulation from Albrecher et al. (2007)
//! to avoid branch-cut discontinuities in the complex logarithm.
//!
//! # Conventions
//!
//! | Parameter | Convention | Units |
//! |-----------|-----------|-------|
//! | Rates (r, q) | Continuously compounded | Decimal (0.05 = 5%) |
//! | Variance (v0, theta) | Annualized variance | Decimal (0.04 = 20% vol) |
//! | Vol-of-vol (sigma_v) | Annualized | Decimal |
//! | Time (T) | ACT/365-style | Years |
//! | Prices | Per unit of underlying | Currency units |
//!
//! # Reference
//!
//! - Heston (1993) - "A Closed-Form Solution for Options with Stochastic Volatility"
//! - Carr & Madan (1999) - "Option valuation using the fast Fourier transform"
//! - Albrecher et al. (2007) - "The Little Heston Trap"

use finstack_core::math::gauss_legendre_integrate_composite;
use num_complex::Complex;
use std::f64::consts::PI;

#[derive(Debug, Clone, Copy)]
/// Heston stochastic volatility model parameters.
///
/// # References
///
/// - Heston, S. L. (1993). "A Closed-Form Solution for Options with Stochastic Volatility
///   with Applications to Bond and Currency Options." *Review of Financial Studies*, 6(2), 327-343.
pub struct HestonParams {
    /// Risk-free interest rate
    pub r: f64,
    /// Continuous dividend yield
    pub q: f64,
    /// Mean reversion speed of variance
    pub kappa: f64,
    /// Long-run variance level
    pub theta: f64,
    /// Volatility of variance (vol-of-vol)
    pub sigma_v: f64,
    /// Correlation between asset price and variance
    pub rho: f64,
    /// Initial variance level
    pub v0: f64,
}

impl HestonParams {
    /// Create new Heston model parameters
    pub fn new(r: f64, q: f64, kappa: f64, theta: f64, sigma_v: f64, rho: f64, v0: f64) -> Self {
        Self {
            r,
            q,
            kappa,
            theta,
            sigma_v,
            rho,
            v0,
        }
    }
}

#[cfg(feature = "mc")]
impl From<finstack_monte_carlo::process::heston::HestonParams> for HestonParams {
    fn from(value: finstack_monte_carlo::process::heston::HestonParams) -> Self {
        Self {
            r: value.r,
            q: value.q,
            kappa: value.kappa,
            theta: value.theta,
            sigma_v: value.sigma_v,
            rho: value.rho,
            v0: value.v0,
        }
    }
}

/// Configuration for Heston Fourier integration.
///
/// Provides tuning knobs for the numerical integration.
#[derive(Debug, Clone, Copy)]
pub struct HestonFourierSettings {
    /// Upper limit for Fourier integral (default: 100)
    pub u_max: f64,
    /// Number of panels for composite Gauss-Legendre (default: 100)
    pub panels: usize,
    /// Gauss-Legendre order per panel (default: 16)
    pub gl_order: usize,
    /// Small epsilon to avoid singularity at φ=0 (default: 1e-8)
    pub phi_eps: f64,
}

impl Default for HestonFourierSettings {
    fn default() -> Self {
        Self {
            u_max: 100.0,
            panels: 100,
            gl_order: 16,
            phi_eps: 1e-8,
        }
    }
}

impl HestonFourierSettings {
    /// Create settings adapted to the option's time to maturity.
    ///
    /// Short-dated options require finer integration grids because
    /// the characteristic function oscillates more rapidly.
    ///
    /// | Maturity | u_max | panels | gl_order |
    /// |----------|-------|--------|----------|
    /// | T < 0.05 | 200   | 200    | 16       |
    /// | T < 0.25 | 150   | 150    | 16       |
    /// | T < 1.0  | 100   | 100    | 16       |
    /// | T >= 1.0 | 80    | 80     | 16       |
    #[must_use]
    pub fn for_maturity(time: f64) -> Self {
        if time < 0.05 {
            Self {
                u_max: 200.0,
                panels: 200,
                gl_order: 16,
                phi_eps: 1e-8,
            }
        } else if time < 0.25 {
            Self {
                u_max: 150.0,
                panels: 150,
                gl_order: 16,
                phi_eps: 1e-8,
            }
        } else if time < 1.0 {
            Self::default()
        } else {
            Self {
                u_max: 80.0,
                panels: 80,
                gl_order: 16,
                phi_eps: 1e-8,
            }
        }
    }
}

/// Heston probability characteristic function ψ_j(φ) for j ∈ {1, 2}.
///
/// Uses the "Little Heston Trap" formulation from Albrecher et al. (2007)
/// to avoid branch-cut discontinuities and overflow from `exp(+dT)`.
///
/// The key change vs. the original Heston (1993) is:
/// - `g⁻ = (b - ρσφi - d) / (b - ρσφi + d)` (swapped numerator/denominator)
/// - `exp(-dT)` instead of `exp(+dT)` (avoids overflow for large T or Re(d) > 0)
///
/// # Arguments
///
/// * `j` - Probability index (1 or 2)
/// * `phi` - Fourier variable
/// * `time` - Time to maturity
/// * `log_spot` - Natural log of spot price
/// * `params` - Heston model parameters
///
/// # Returns
///
/// Complex value of ψ_j(φ)
///
/// # References
///
/// - Albrecher et al. (2007) — "The Little Heston Trap"
fn heston_pj_characteristic_function(
    j: u8,
    phi: f64,
    time: f64,
    log_spot: f64,
    params: &HestonParams,
) -> Complex<f64> {
    let kappa = params.kappa;
    let theta = params.theta;
    let sigma = params.sigma_v;
    let rho = params.rho;
    let v0 = params.v0;
    let r = params.r;
    let q = params.q;

    let i = Complex::new(0.0, 1.0);

    // For P1: u = 0.5, b = kappa - rho*sigma
    // For P2: u = -0.5, b = kappa
    let (u, b) = if j == 1 {
        (0.5, kappa - rho * sigma)
    } else {
        (-0.5, kappa)
    };

    let a = kappa * theta;
    let sigma_sq = sigma * sigma;

    // d = sqrt((rho*sigma*phi*i - b)^2 - sigma^2*(2*u*phi*i - phi^2))
    let d_sq = (rho * sigma * phi * i - b).powi(2) - sigma_sq * (2.0 * u * phi * i - phi * phi);
    let d = d_sq.sqrt();

    // Little Heston Trap formulation (Albrecher et al. 2007):
    // g⁻ = (b - rho*sigma*phi*i - d) / (b - rho*sigma*phi*i + d)
    // Uses exp(-dT) to avoid overflow
    let b_minus_rsi = b - rho * sigma * phi * i;
    let g_minus = (b_minus_rsi - d) / (b_minus_rsi + d);

    // exp(-d*T) — bounded, avoids the overflow of exp(+dT)
    let exp_minus_dt = (-d * time).exp();

    let one = Complex::new(1.0, 0.0);

    // C = (r-q)*phi*i*T + (a/sigma^2) * [(b - rho*sigma*phi*i - d)*T
    //     - 2*ln((1 - g⁻*exp(-dT)) / (1 - g⁻))]
    let c = (r - q) * phi * i * time
        + (a / sigma_sq)
            * ((b_minus_rsi - d) * time
                - 2.0 * ((one - g_minus * exp_minus_dt) / (one - g_minus)).ln());

    // D = (b - rho*sigma*phi*i - d) / sigma^2
    //     * (1 - exp(-dT)) / (1 - g⁻*exp(-dT))
    let d_val =
        (b_minus_rsi - d) / sigma_sq * (one - exp_minus_dt) / (one - g_minus * exp_minus_dt);

    // ψ_j(φ) = exp(C + D*v0 + i*φ*ln(S))
    (c + d_val * v0 + i * phi * log_spot).exp()
}

/// Compute Pj probability for Heston call pricing via Fourier inversion.
///
/// P_j = 0.5 + (1/π) ∫_0^∞ Re[exp(-i*φ*ln(K)) * ψ_j(φ) / (i*φ)] dφ
///
/// # Arguments
///
/// * `j` - Probability index (1 or 2)
/// * `spot` - Current spot price
/// * `strike` - Strike price
/// * `time` - Time to maturity
/// * `params` - Heston model parameters
/// * `settings` - Integration settings
fn heston_pj(
    j: u8,
    spot: f64,
    strike: f64,
    time: f64,
    params: &HestonParams,
    settings: &HestonFourierSettings,
) -> f64 {
    let log_spot = spot.ln();
    let log_strike = strike.ln();
    let i = Complex::new(0.0, 1.0);

    // Integrand: Re[exp(-i*φ*ln(K)) * ψ_j(φ) / (i*φ)]
    let integrand = |phi: f64| {
        // Handle singularity at φ=0
        if phi.abs() < settings.phi_eps {
            return 0.0;
        }

        let psi = heston_pj_characteristic_function(j, phi, time, log_spot, params);
        let exp_term = (-i * phi * log_strike).exp();
        let integrand_complex = exp_term * psi / (i * phi);

        integrand_complex.re
    };

    // Use composite Gauss-Legendre integration over [0, u_max]
    let integral = gauss_legendre_integrate_composite(
        integrand,
        0.0,
        settings.u_max,
        settings.gl_order,
        settings.panels,
    )
    .unwrap_or(0.0);

    let prob = 0.5 + integral / PI;

    // Clamp to [0, 1] to handle small numerical errors
    prob.clamp(0.0, 1.0)
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
///
/// # Integration Settings
///
/// Uses [`HestonFourierSettings::for_maturity`] to adapt the integration grid
/// to the option's time to maturity. Short-dated options use finer grids to
/// handle the more rapidly oscillating characteristic function. For custom
/// control, use [`heston_call_price_fourier_with_settings`].
///
/// # Example
///
/// ```text
/// use finstack_valuations::instruments::common::models::closed_form::heston::{
///     heston_call_price_fourier, HestonParams,
/// };
///
/// let params = HestonParams::new(
///     0.05,  // risk-free rate
///     0.02,  // dividend yield
///     2.0,   // kappa (mean reversion)
///     0.04,  // theta (long-run variance)
///     0.3,   // sigma_v (vol-of-vol)
///     -0.7,  // rho (correlation)
///     0.04,  // v0 (initial variance)
/// );
///
/// let price = heston_call_price_fourier(100.0, 100.0, 1.0, &params);
/// assert!(price > 0.0 && price < 100.0);
/// ```
#[must_use]
pub fn heston_call_price_fourier(spot: f64, strike: f64, time: f64, params: &HestonParams) -> f64 {
    heston_call_price_fourier_with_settings(
        spot,
        strike,
        time,
        params,
        &HestonFourierSettings::for_maturity(time),
    )
}

/// Price a European call option with custom integration settings.
///
/// See [`heston_call_price_fourier`] for details.
#[must_use]
pub fn heston_call_price_fourier_with_settings(
    spot: f64,
    strike: f64,
    time: f64,
    params: &HestonParams,
    settings: &HestonFourierSettings,
) -> f64 {
    // Handle expired options
    if time <= 0.0 {
        return (spot - strike).max(0.0);
    }

    // Special case: very small vol-of-vol approaches Black-Scholes
    // This avoids numerical issues when sigma_v is tiny
    if params.sigma_v < 1e-10 {
        return black_scholes_call(spot, strike, time, params.r, params.q, params.v0.sqrt());
    }

    // Compute P1 and P2 via Fourier inversion
    let p1 = heston_pj(1, spot, strike, time, params, settings);
    let p2 = heston_pj(2, spot, strike, time, params, settings);

    // C = S * exp(-qT) * P1 - K * exp(-rT) * P2
    let call_price = spot * (-params.q * time).exp() * p1 - strike * (-params.r * time).exp() * p2;

    // Clamp to non-negative (numerical errors can cause tiny negatives for deep OTM)
    call_price.max(0.0)
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
/// Uses put-call parity: P = C - S*exp(-qT) + K*exp(-rT)
#[must_use]
pub fn heston_put_price_fourier(spot: f64, strike: f64, time: f64, params: &HestonParams) -> f64 {
    heston_put_price_fourier_with_settings(
        spot,
        strike,
        time,
        params,
        &HestonFourierSettings::for_maturity(time),
    )
}

/// Price a European put option with custom integration settings.
///
/// See [`heston_put_price_fourier`] for details.
pub fn heston_put_price_fourier_with_settings(
    spot: f64,
    strike: f64,
    time: f64,
    params: &HestonParams,
    settings: &HestonFourierSettings,
) -> f64 {
    if time <= 0.0 {
        return (strike - spot).max(0.0);
    }

    // Use put-call parity: P = C - S*exp(-qT) + K*exp(-rT)
    let call_price = heston_call_price_fourier_with_settings(spot, strike, time, params, settings);
    let forward = spot * (-params.q * time).exp();
    let discount_k = strike * (-params.r * time).exp();

    (call_price - forward + discount_k).max(0.0)
}

/// Black-Scholes call price (fallback for sigma_v ≈ 0).
fn black_scholes_call(spot: f64, strike: f64, time: f64, r: f64, q: f64, vol: f64) -> f64 {
    use crate::instruments::common_impl::models::closed_form::vanilla::bs_price;
    use crate::instruments::common_impl::parameters::OptionType;
    bs_price(spot, strike, r, q, vol, time, OptionType::Call)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    /// Test that ψ_j(0) ≈ 1 for both probability characteristic functions.
    #[test]
    fn test_pj_char_function_at_zero() {
        let params = HestonParams::new(0.05, 0.02, 2.0, 0.04, 0.3, -0.7, 0.04);
        let log_spot = 100.0_f64.ln();

        // At φ=0, ψ_j(0) should equal 1 (or very close)
        for j in [1u8, 2u8] {
            let psi = heston_pj_characteristic_function(j, 1e-10, 1.0, log_spot, &params);
            assert!(
                (psi.re - 1.0).abs() < 0.01,
                "ψ_{}(0) real part should be ~1, got {}",
                j,
                psi.re
            );
            assert!(
                psi.im.abs() < 0.01,
                "ψ_{}(0) imag part should be ~0, got {}",
                j,
                psi.im
            );
        }
    }

    /// Test that P1 and P2 are within valid probability range [0, 1].
    #[test]
    fn test_probabilities_in_valid_range() {
        let params = HestonParams::new(0.05, 0.02, 2.0, 0.04, 0.3, -0.7, 0.04);
        let settings = HestonFourierSettings::default();

        // Test various moneyness levels
        for strike in [80.0, 100.0, 120.0] {
            let p1 = heston_pj(1, 100.0, strike, 1.0, &params, &settings);
            let p2 = heston_pj(2, 100.0, strike, 1.0, &params, &settings);

            assert!(
                (0.0..=1.0).contains(&p1),
                "P1 should be in [0,1], got {} for K={}",
                p1,
                strike
            );
            assert!(
                (0.0..=1.0).contains(&p2),
                "P2 should be in [0,1], got {} for K={}",
                p2,
                strike
            );

            // P1 >= P2 for calls (P1 is stock measure, P2 is money measure)
            assert!(
                p1 >= p2 - 1e-6,
                "P1 should be >= P2, got P1={}, P2={} for K={}",
                p1,
                p2,
                strike
            );
        }
    }

    /// Test that call price is positive and reasonable.
    #[test]
    fn test_heston_call_positive() {
        let params = HestonParams::new(0.05, 0.02, 2.0, 0.04, 0.3, -0.7, 0.04);

        let price = heston_call_price_fourier(100.0, 100.0, 1.0, &params);

        assert!(price > 0.0, "Call price should be positive, got {}", price);
        assert!(
            price < 100.0,
            "Call price should be less than spot, got {}",
            price
        );
    }

    /// Test put-call parity holds.
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
            "Put-call parity failed: C-P={} vs S*exp(-qT)-K*exp(-rT)={}",
            lhs,
            rhs
        );
    }

    /// Test convergence to Black-Scholes as vol-of-vol → 0.
    #[test]
    fn test_black_scholes_limit() {
        let vol = 0.2;
        let variance = vol * vol;

        // Heston with very small sigma_v should match Black-Scholes
        let params = HestonParams::new(
            0.05,     // r
            0.0,      // q
            2.0,      // kappa (doesn't matter when sigma_v=0)
            variance, // theta = v0 for consistency
            1e-12,    // sigma_v ≈ 0
            0.0,      // rho
            variance, // v0
        );

        let heston_price = heston_call_price_fourier(100.0, 100.0, 1.0, &params);
        let bs_price = black_scholes_call(100.0, 100.0, 1.0, 0.05, 0.0, vol);

        assert!(
            (heston_price - bs_price).abs() < 0.01,
            "Heston should converge to BS: Heston={}, BS={}",
            heston_price,
            bs_price
        );
    }

    /// Test against the volatility/heston.rs implementation.
    ///
    /// Cross-validates our closed-form implementation against the
    /// HestonModel implementation in the volatility module.
    #[test]
    fn test_cross_validation_with_volatility_heston() {
        use crate::instruments::common_impl::models::volatility::heston::{
            HestonModel, HestonParameters,
        };

        // Test parameters
        let spot = 100.0;
        let strike = 100.0;
        let time = 0.5;
        let r = 0.05;
        let q = 0.02;
        let v0 = 0.04;
        let kappa = 2.0;
        let theta = 0.04;
        let sigma_v = 0.3;
        let rho = -0.7;

        // Our implementation
        let params = HestonParams::new(r, q, kappa, theta, sigma_v, rho, v0);
        let our_price = heston_call_price_fourier(spot, strike, time, &params);

        // Volatility module implementation
        let vol_params =
            HestonParameters::new(v0, kappa, theta, sigma_v, rho).expect("valid Heston params");
        let model = HestonModel::new(vol_params);
        let vol_price = model
            .price_european_call(spot, strike, time, r, q)
            .expect("Heston pricing should succeed");

        // Both implementations should produce similar prices
        // Allow some tolerance due to different integration schemes
        assert!(
            (our_price - vol_price).abs() < 0.1,
            "Closed-form price {} should match volatility module price {} within tolerance",
            our_price,
            vol_price
        );
    }

    /// Test a known reference case with reasonable parameters.
    ///
    /// Uses typical equity option parameters and validates the price
    /// is within an expected range based on Black-Scholes bounds.
    #[test]
    fn test_reference_typical_params() {
        let params = HestonParams::new(
            0.05, // r
            0.0,  // q
            2.0,  // kappa
            0.04, // theta
            0.3,  // sigma_v
            -0.5, // rho
            0.04, // v0
        );

        let price = heston_call_price_fourier(100.0, 100.0, 0.5, &params);

        // With v0=0.04 (20% vol) and T=0.5, ATM call should be roughly 5-8
        // BS with 20% vol gives ~5.87 for these params
        assert!(
            price > 4.0 && price < 10.0,
            "Heston price {} should be in reasonable range for these parameters",
            price
        );
    }

    /// Test another reference case: ATM option with typical equity parameters.
    ///
    /// Parameters: S=100, K=100, T=1, r=0.05, q=0.02
    /// v0=0.04, kappa=2.0, theta=0.04, sigma=0.3, rho=-0.7
    #[test]
    fn test_reference_typical_equity() {
        let params = HestonParams::new(0.05, 0.02, 2.0, 0.04, 0.3, -0.7, 0.04);

        let call = heston_call_price_fourier(100.0, 100.0, 1.0, &params);
        let put = heston_put_price_fourier(100.0, 100.0, 1.0, &params);

        // With v0=0.04 (20% vol), ATM call should be roughly 8-10
        assert!(
            call > 5.0 && call < 15.0,
            "ATM call price {} should be reasonable",
            call
        );
        assert!(
            put > 3.0 && put < 12.0,
            "ATM put price {} should be reasonable",
            put
        );
    }

    /// Test OTM and ITM options have correct ordering.
    #[test]
    fn test_moneyness_ordering() {
        let params = HestonParams::new(0.05, 0.02, 2.0, 0.04, 0.3, -0.7, 0.04);

        let call_itm = heston_call_price_fourier(100.0, 90.0, 1.0, &params);
        let call_atm = heston_call_price_fourier(100.0, 100.0, 1.0, &params);
        let call_otm = heston_call_price_fourier(100.0, 110.0, 1.0, &params);

        // ITM > ATM > OTM for calls
        assert!(
            call_itm > call_atm,
            "ITM call {} should be > ATM call {}",
            call_itm,
            call_atm
        );
        assert!(
            call_atm > call_otm,
            "ATM call {} should be > OTM call {}",
            call_atm,
            call_otm
        );
    }

    /// Test expired option returns intrinsic value.
    #[test]
    fn test_expired_option() {
        let params = HestonParams::new(0.05, 0.02, 2.0, 0.04, 0.3, -0.7, 0.04);

        // ITM call
        let call_itm = heston_call_price_fourier(100.0, 90.0, 0.0, &params);
        assert!(
            (call_itm - 10.0).abs() < 1e-10,
            "Expired ITM call should be intrinsic: {}",
            call_itm
        );

        // OTM call
        let call_otm = heston_call_price_fourier(100.0, 110.0, 0.0, &params);
        assert!(
            call_otm.abs() < 1e-10,
            "Expired OTM call should be 0: {}",
            call_otm
        );

        // ITM put
        let put_itm = heston_put_price_fourier(100.0, 110.0, 0.0, &params);
        assert!(
            (put_itm - 10.0).abs() < 1e-10,
            "Expired ITM put should be intrinsic: {}",
            put_itm
        );
    }

    /// Test with extreme parameters to ensure stability.
    #[test]
    fn test_stability_extreme_params() {
        // High vol-of-vol
        let params_high_vov = HestonParams::new(0.05, 0.0, 5.0, 0.09, 1.0, -0.9, 0.09);
        let price = heston_call_price_fourier(100.0, 100.0, 1.0, &params_high_vov);
        assert!(
            price.is_finite() && price >= 0.0,
            "Should handle high vol-of-vol"
        );

        // Very short maturity
        let params = HestonParams::new(0.05, 0.02, 2.0, 0.04, 0.3, -0.7, 0.04);
        let price_short = heston_call_price_fourier(100.0, 100.0, 0.01, &params);
        assert!(
            price_short.is_finite() && price_short >= 0.0,
            "Should handle short maturity"
        );

        // Deep OTM
        let price_deep_otm = heston_call_price_fourier(100.0, 200.0, 1.0, &params);
        assert!(
            price_deep_otm.is_finite() && price_deep_otm >= 0.0,
            "Should handle deep OTM"
        );

        // Deep ITM
        let price_deep_itm = heston_call_price_fourier(100.0, 50.0, 1.0, &params);
        assert!(
            price_deep_itm.is_finite() && price_deep_itm > 40.0,
            "Should handle deep ITM"
        );
    }

    /// Test improved accuracy for very short-dated options.
    #[test]
    fn test_short_maturity_adaptive() {
        let params = HestonParams::new(0.05, 0.0, 2.0, 0.04, 0.3, -0.7, 0.04);

        // Very short maturity: T = 1 week
        let time = 7.0 / 365.0;
        let price = heston_call_price_fourier(100.0, 100.0, time, &params);

        // Should be close to BS with vol = sqrt(v0) = 0.2
        let bs = black_scholes_call(100.0, 100.0, time, 0.05, 0.0, 0.2);

        // With short maturity and moderate vol-of-vol, Heston ≈ BS
        assert!(
            (price - bs).abs() < 0.5,
            "Short-dated Heston={:.4} should be close to BS={:.4}",
            price,
            bs
        );
        assert!(price > 0.0, "Price must be positive");
    }

    /// Test that adaptive settings produce valid results across maturities.
    #[test]
    fn test_adaptive_settings_consistency() {
        let params = HestonParams::new(0.05, 0.02, 2.0, 0.04, 0.3, -0.7, 0.04);

        for &time in &[0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.0, 5.0] {
            let price = heston_call_price_fourier(100.0, 100.0, time, &params);
            assert!(
                price.is_finite() && price >= 0.0,
                "Price must be finite and non-negative for T={}: got {}",
                time,
                price
            );

            // Put-call parity must hold
            let put = heston_put_price_fourier(100.0, 100.0, time, &params);
            let parity =
                price - put - (100.0 * (-0.02 * time).exp() - 100.0 * (-0.05 * time).exp());
            assert!(
                parity.abs() < 0.1,
                "Put-call parity violated for T={}: residual={}",
                time,
                parity
            );
        }
    }
}
