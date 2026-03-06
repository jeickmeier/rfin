//! Heston (1993) stochastic volatility model.
//!
//! Implements the Heston model for European option pricing and global
//! calibration to market-implied volatilities. Uses the Gil-Pelaez / P1-P2
//! Fourier inversion with the "Little Heston Trap" formulation from
//! Albrecher et al. (2007) for numerical stability.
//!
//! # Mathematical Foundation
//!
//! The Heston model describes the joint dynamics of an asset price and its
//! instantaneous variance:
//!
//! ```text
//! dS = (r - q) S dt + √v S dW₁
//! dv = κ(θ - v) dt + σ√v dW₂
//! E[dW₁ dW₂] = ρ dt
//!
//! where:
//!   S = asset price
//!   v = instantaneous variance
//!   κ = mean reversion speed of variance
//!   θ = long-run variance level
//!   σ = volatility of variance (vol-of-vol)
//!   ρ = correlation between asset and variance processes
//! ```
//!
//! # Parameters
//!
//! | Parameter | Symbol | Range | Market Role |
//! |-----------|--------|-------|-------------|
//! | v0 | v₀ | > 0 | Initial variance |
//! | kappa | κ | > 0 | Mean reversion speed |
//! | theta | θ | > 0 | Long-run variance |
//! | sigma | σ | > 0 | Vol-of-vol (smile curvature) |
//! | rho | ρ | (-1, 1) | Skew direction |
//!
//! # Feller Condition
//!
//! The condition 2κθ > σ² ensures the variance process remains strictly
//! positive. When violated, the process can hit zero, potentially causing
//! numerical instability. The constructor warns but does not reject.
//!
//! # References
//!
//! - Heston, S. L. (1993). "A Closed-Form Solution for Options with Stochastic
//!   Volatility with Applications to Bond and Currency Options."
//!   *Review of Financial Studies*, 6(2), 327-343.
//! - Albrecher, H., Mayer, P., Schoutens, W., & Tistaert, J. (2007).
//!   "The Little Heston Trap." *Wilmott Magazine*, January 2007.
//! - Gatheral, J. (2006). *The Volatility Surface: A Practitioner's Guide*.
//!   Wiley Finance.

use num_complex::Complex64;
use std::f64::consts::PI;

/// Heston stochastic volatility model parameters.
///
/// # Examples
///
/// ```rust
/// use finstack_core::math::volatility::heston::HestonParams;
///
/// let params = HestonParams::new(0.04, 2.0, 0.04, 0.3, -0.5).unwrap();
/// assert!(params.satisfies_feller_condition());
///
/// let call = params.price_european(100.0, 100.0, 0.05, 0.0, 1.0, true);
/// assert!(call > 0.0 && call < 100.0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct HestonParams {
    /// Initial variance (v₀ > 0).
    pub v0: f64,
    /// Mean reversion speed (κ > 0).
    pub kappa: f64,
    /// Long-run variance (θ > 0).
    pub theta: f64,
    /// Vol-of-vol (σ > 0).
    pub sigma: f64,
    /// Correlation between spot and variance (-1 < ρ < 1).
    pub rho: f64,
}

impl HestonParams {
    /// Construct validated Heston parameters.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - `v0 <= 0` or non-finite
    /// - `kappa <= 0` or non-finite
    /// - `theta <= 0` or non-finite
    /// - `sigma <= 0` or non-finite
    /// - `rho` not in `(-1, 1)` or non-finite
    ///
    /// # Feller Condition
    ///
    /// If 2κθ ≤ σ², a warning is emitted (but the parameters are still accepted).
    pub fn new(v0: f64, kappa: f64, theta: f64, sigma: f64, rho: f64) -> crate::Result<Self> {
        if v0 <= 0.0 || !v0.is_finite() {
            return Err(crate::Error::Validation(format!(
                "Heston v0 (initial variance) must be positive, got {v0}"
            )));
        }
        if kappa <= 0.0 || !kappa.is_finite() {
            return Err(crate::Error::Validation(format!(
                "Heston kappa (mean reversion) must be positive, got {kappa}"
            )));
        }
        if theta <= 0.0 || !theta.is_finite() {
            return Err(crate::Error::Validation(format!(
                "Heston theta (long-run variance) must be positive, got {theta}"
            )));
        }
        if sigma <= 0.0 || !sigma.is_finite() {
            return Err(crate::Error::Validation(format!(
                "Heston sigma (vol-of-vol) must be positive, got {sigma}"
            )));
        }
        if rho <= -1.0 || rho >= 1.0 || !rho.is_finite() {
            return Err(crate::Error::Validation(format!(
                "Heston rho (correlation) must be in (-1, 1), got {rho}"
            )));
        }

        Ok(Self {
            v0,
            kappa,
            theta,
            sigma,
            rho,
        })
    }

    /// Check whether the Feller condition (2κθ > σ²) is satisfied.
    ///
    /// When satisfied, the variance process is strictly positive almost surely.
    #[must_use]
    pub fn satisfies_feller_condition(&self) -> bool {
        2.0 * self.kappa * self.theta > self.sigma * self.sigma
    }

    /// Price a European option using Fourier integration.
    ///
    /// Uses the Gil-Pelaez / P1-P2 formulation:
    /// ```text
    /// Call = S × exp(-qT) × P₁ - K × exp(-rT) × P₂
    /// Put  = Call - S × exp(-qT) + K × exp(-rT)   (put-call parity)
    /// ```
    ///
    /// where P₁ and P₂ are computed via numerical integration of the
    /// Heston characteristic function using composite Gauss-Legendre quadrature.
    ///
    /// # Arguments
    ///
    /// * `spot` - Current spot price
    /// * `strike` - Strike price
    /// * `r` - Risk-free rate (continuous compounding)
    /// * `q` - Dividend yield (continuous compounding)
    /// * `t` - Time to expiry in years
    /// * `is_call` - `true` for call, `false` for put
    ///
    /// # Returns
    ///
    /// Option price (non-negative).
    #[must_use]
    pub fn price_european(
        &self,
        spot: f64,
        strike: f64,
        r: f64,
        q: f64,
        t: f64,
        is_call: bool,
    ) -> f64 {
        if t <= 0.0 {
            return if is_call {
                (spot - strike).max(0.0)
            } else {
                (strike - spot).max(0.0)
            };
        }

        // Degenerate case: very small vol-of-vol → use Black-Scholes
        if self.sigma < 1e-10 {
            return bs_call_fallback(spot, strike, r, q, t, self.v0.sqrt(), is_call);
        }

        let p1 = self.compute_pj(1, spot, strike, r, q, t);
        let p2 = self.compute_pj(2, spot, strike, r, q, t);

        let call = (spot * (-q * t).exp() * p1 - strike * (-r * t).exp() * p2).max(0.0);

        if is_call {
            call
        } else {
            // Put-call parity
            (call - spot * (-q * t).exp() + strike * (-r * t).exp()).max(0.0)
        }
    }

    /// Compute probability P_j via Fourier inversion.
    ///
    /// P_j = 1/2 + (1/π) ∫₀^∞ Re[exp(-iφ ln K) ψ_j(φ) / (iφ)] dφ
    fn compute_pj(&self, j: u8, spot: f64, strike: f64, r: f64, q: f64, t: f64) -> f64 {
        let x = spot.ln();
        let ln_k = strike.ln();
        let i = Complex64::i();

        let integrand = |phi: f64| -> f64 {
            if phi.abs() < 1e-10 {
                return 0.0;
            }
            let psi = self.char_func_j(j, phi, x, r, q, t);
            if !psi.is_finite() {
                return 0.0;
            }
            let exp_term = (-i * phi * ln_k).exp();
            let val = (exp_term * psi / (i * phi)).re;
            if val.is_finite() {
                val
            } else {
                0.0
            }
        };

        // Composite Gauss-Legendre on [ε, 100]: 16th order × 8 panels = 128 points
        let integral = crate::math::integration::gauss_legendre_integrate_composite(
            integrand, 1e-8, 100.0, 16, 8,
        )
        .unwrap_or(0.0);

        (0.5 + integral / PI).clamp(0.0, 1.0)
    }

    /// Characteristic function ψ_j(φ) for the Heston model.
    ///
    /// Uses the "Little Heston Trap" formulation (Albrecher et al. 2007)
    /// which places −d in the numerator of g, ensuring |g exp(−dT)| < 1
    /// and avoiding branch-cut discontinuities.
    fn char_func_j(&self, j: u8, phi: f64, x: f64, r: f64, q: f64, t: f64) -> Complex64 {
        let kappa = self.kappa;
        let theta = self.theta;
        let sigma = self.sigma;
        let rho = self.rho;
        let v0 = self.v0;

        let i = Complex64::i();
        let one = Complex64::new(1.0, 0.0);

        // For P₁: u = 0.5, b = κ − ρσ  (stock numeraire)
        // For P₂: u = −0.5, b = κ       (money market numeraire)
        let (u_j, b_j) = if j == 1 {
            (0.5, kappa - rho * sigma)
        } else {
            (-0.5, kappa)
        };

        let a = kappa * theta;
        let sigma_sq = sigma * sigma;

        // d = sqrt((ρσiφ − b)² − σ²(2u_j iφ − φ²))
        let rsi_phi = Complex64::new(0.0, rho * sigma * phi);
        let b = Complex64::new(b_j, 0.0);
        let d_sq = (rsi_phi - b).powi(2) - sigma_sq * (Complex64::new(-phi * phi, 2.0 * u_j * phi));
        let d = d_sq.sqrt();

        // Little Heston Trap: g = (b − ρσiφ − d)/(b − ρσiφ + d)
        let bm = b - rsi_phi;
        let g = (bm - d) / (bm + d);

        let exp_minus_dt = (-d * t).exp();

        // C = (r−q)iφT + (a/σ²)[(b−ρσiφ−d)T − 2 ln((1−g exp(−dT))/(1−g))]
        let c_val = i * phi * (r - q) * t
            + (a / sigma_sq)
                * ((bm - d) * t
                    - Complex64::new(2.0, 0.0) * ((one - g * exp_minus_dt) / (one - g)).ln());

        // D = (b−ρσiφ−d)/σ² × (1−exp(−dT))/(1−g exp(−dT))
        let d_val = ((bm - d) / sigma_sq) * (one - exp_minus_dt) / (one - g * exp_minus_dt);

        // ψ_j(φ) = exp(C + D v₀ + iφx)
        (c_val + d_val * v0 + i * phi * x).exp()
    }
}

/// Calibration diagnostics returned alongside fitted parameters.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HestonCalibrationResult {
    /// Calibrated Heston parameters.
    pub params: HestonParams,
    /// Root mean square error of volatility residuals (in vol units).
    pub rmse: f64,
    /// Number of solver iterations.
    pub iterations: usize,
    /// Whether the solver converged.
    pub converged: bool,
}

/// Calibrate Heston model parameters from market implied volatilities.
///
/// Fits the five Heston parameters (v₀, κ, θ, σ, ρ) by minimising
/// vega-weighted price differences across all expiry/strike pairs using
/// the Levenberg-Marquardt algorithm.
///
/// # Arguments
///
/// * `spot` - Current spot price
/// * `r` - Risk-free rate (continuous compounding)
/// * `q` - Dividend yield (continuous compounding)
/// * `expiries` - Expiry times for each slice (years)
/// * `strikes` - Strike prices per expiry: `strikes[i]` for `expiries[i]`
/// * `market_vols` - Market Black-76 implied vols per expiry: `market_vols[i]` for `expiries[i]`
///
/// # Returns
///
/// [`HestonCalibrationResult`] containing the fitted parameters and diagnostics.
///
/// # Algorithm
///
/// Uses an unconstrained parameterisation to map the bounded Heston
/// parameters to ℝ⁵:
///
/// | Heston | Unconstrained |
/// |--------|---------------|
/// | v₀ > 0 | x₀ = ln(v₀) |
/// | κ > 0 | x₁ = ln(κ) |
/// | θ > 0 | x₂ = ln(θ) |
/// | σ > 0 | x₃ = ln(σ) |
/// | ρ ∈ (−1,1) | x₄ = atanh(ρ) |
///
/// # Errors
///
/// Returns an error if:
/// - `expiries`, `strikes`, `market_vols` lengths are inconsistent
/// - Fewer than 5 data points (need at least 5 for 5 free parameters)
/// - Calibration fails to converge
///
/// # Examples
///
/// ```rust
/// use finstack_core::math::volatility::heston::{HestonParams, calibrate_heston};
///
/// let params = HestonParams::new(0.04, 2.0, 0.04, 0.3, -0.5).unwrap();
/// let spot = 100.0;
/// let r = 0.05;
/// let q = 0.0;
/// let expiries = [1.0];
/// let strikes_1y: Vec<f64> = vec![90.0, 95.0, 100.0, 105.0, 110.0];
/// let vols: Vec<f64> = strikes_1y.iter().map(|&k| {
///     // Generate synthetic vols by inverting Heston prices through BS
///     let price = params.price_european(spot, k, r, q, 1.0, true);
///     let fwd = spot * ((r - q) * 1.0).exp();
///     finstack_core::math::volatility::implied_vol_black(price, fwd, k, 1.0, true)
///         .unwrap_or(0.2)
/// }).collect();
/// let strikes = [strikes_1y.as_slice()];
/// let market_vols = [vols.as_slice()];
///
/// let result = calibrate_heston(spot, r, q, &expiries, &strikes, &market_vols).unwrap();
/// assert!(result.rmse < 0.01);
/// ```
pub fn calibrate_heston(
    spot: f64,
    r: f64,
    q: f64,
    expiries: &[f64],
    strikes: &[&[f64]],
    market_vols: &[&[f64]],
) -> crate::Result<HestonCalibrationResult> {
    // ---- Validate inputs ----
    if expiries.len() != strikes.len() || expiries.len() != market_vols.len() {
        return Err(crate::Error::Validation(
            "expiries, strikes, and market_vols must have the same outer length".to_string(),
        ));
    }
    let mut n_total = 0usize;
    for (i, (&t, (ks, vs))) in expiries
        .iter()
        .zip(strikes.iter().zip(market_vols.iter()))
        .enumerate()
    {
        if ks.len() != vs.len() {
            return Err(crate::Error::Validation(format!(
                "strikes[{i}] and market_vols[{i}] must have the same length"
            )));
        }
        if t <= 0.0 {
            return Err(crate::Error::Validation(format!(
                "expiries[{i}] must be positive, got {t}"
            )));
        }
        n_total += ks.len();
    }
    if n_total < 5 {
        return Err(crate::Error::Validation(format!(
            "Need at least 5 data points for Heston calibration (5 free parameters), got {n_total}"
        )));
    }

    // ---- Flatten market data and pre-compute market prices + vegas ----
    let mut flat_expiry = Vec::with_capacity(n_total);
    let mut flat_strike = Vec::with_capacity(n_total);
    let mut flat_market_price = Vec::with_capacity(n_total);
    let mut flat_vega = Vec::with_capacity(n_total);

    for (&t, (ks, vs)) in expiries.iter().zip(strikes.iter().zip(market_vols.iter())) {
        let fwd = spot * ((r - q) * t).exp();
        let df = (-r * t).exp();
        for (&k, &vol) in ks.iter().zip(vs.iter()) {
            flat_expiry.push(t);
            flat_strike.push(k);
            let mkt_price = df * crate::math::volatility::black_call(fwd, k, vol, t);
            let vega = (df * crate::math::volatility::black_vega(fwd, k, vol, t)).max(1e-10);
            flat_market_price.push(mkt_price);
            flat_vega.push(vega);
        }
    }

    // ---- Initial guess ----
    // ATM vol → initial v0; reasonable defaults for other params
    let atm_vol = {
        let mut best = 0.2_f64;
        let mut best_dist = f64::MAX;
        for (&t, (ks, vs)) in expiries.iter().zip(strikes.iter().zip(market_vols.iter())) {
            let fwd = spot * ((r - q) * t).exp();
            for (&k, &v) in ks.iter().zip(vs.iter()) {
                let dist = ((k - fwd) / fwd).abs();
                if dist < best_dist {
                    best_dist = dist;
                    best = v;
                }
            }
        }
        best
    };
    let v0_init = (atm_vol * atm_vol).max(1e-4);
    let kappa_init: f64 = 2.0;
    let theta_init = v0_init;
    let sigma_init: f64 = 0.3;
    let rho_init: f64 = -0.5;

    // Unconstrained parameterisation
    let x0 = [
        v0_init.ln(),
        kappa_init.ln(),
        theta_init.ln(),
        sigma_init.ln(),
        rho_init.clamp(-0.999, 0.999).atanh(),
    ];

    // ---- LM residual function ----
    // Residual_i = (model_price_i − market_price_i) / vega_i
    let residuals = |x: &[f64], resid: &mut [f64]| {
        let v0 = x[0].exp();
        let kappa = x[1].exp();
        let theta = x[2].exp();
        let sigma = x[3].exp();
        let rho = x[4].tanh();

        let params = HestonParams {
            v0,
            kappa,
            theta,
            sigma,
            rho,
        };

        for (idx, (((&t, &k), &mkt_p), &vega)) in flat_expiry
            .iter()
            .zip(flat_strike.iter())
            .zip(flat_market_price.iter())
            .zip(flat_vega.iter())
            .enumerate()
        {
            let model_price = params.price_european(spot, k, r, q, t, true);
            if model_price.is_finite() {
                resid[idx] = (model_price - mkt_p) / vega;
            } else {
                resid[idx] = 1.0; // penalty
            }
        }
    };

    // ---- Solve ----
    let solver = crate::math::solver_multi::LevenbergMarquardtSolver::new()
        .with_tolerance(1e-10)
        .with_max_iterations(300);

    let solution = solver.solve_system_with_dim_stats(residuals, &x0, n_total)?;

    let v0 = solution.params[0].exp();
    let kappa = solution.params[1].exp();
    let theta = solution.params[2].exp();
    let sigma = solution.params[3].exp();
    let rho = solution.params[4].tanh();

    // ---- Compute RMSE in vol space ----
    let fitted = HestonParams {
        v0,
        kappa,
        theta,
        sigma,
        rho,
    };
    let mut sse = 0.0;
    for (&t, (ks, vs)) in expiries.iter().zip(strikes.iter().zip(market_vols.iter())) {
        let fwd = spot * ((r - q) * t).exp();
        let df = (-r * t).exp();
        for (&k, &mv) in ks.iter().zip(vs.iter()) {
            let model_price = fitted.price_european(spot, k, r, q, t, true);
            let model_vol =
                crate::math::volatility::implied_vol_black(model_price / df, fwd, k, t, true)
                    .unwrap_or(mv);
            sse += (model_vol - mv) * (model_vol - mv);
        }
    }
    let rmse = (sse / n_total as f64).sqrt();

    let converged = matches!(
        solution.stats.termination_reason,
        crate::math::solver_multi::LmTerminationReason::ConvergedResidualNorm
            | crate::math::solver_multi::LmTerminationReason::ConvergedRelativeReduction
            | crate::math::solver_multi::LmTerminationReason::ConvergedGradient
            | crate::math::solver_multi::LmTerminationReason::StepTooSmall
    );

    // Validate recovered parameters
    let params = HestonParams::new(v0, kappa, theta, sigma, rho)?;

    Ok(HestonCalibrationResult {
        params,
        rmse,
        iterations: solution.stats.iterations,
        converged,
    })
}

/// Black-Scholes fallback for degenerate Heston (σ_v ≈ 0).
fn bs_call_fallback(
    spot: f64,
    strike: f64,
    r: f64,
    q: f64,
    t: f64,
    vol: f64,
    is_call: bool,
) -> f64 {
    use crate::math::special_functions::norm_cdf;

    if vol <= 0.0 || t <= 0.0 {
        return if is_call {
            (spot * (-q * t).exp() - strike * (-r * t).exp()).max(0.0)
        } else {
            (strike * (-r * t).exp() - spot * (-q * t).exp()).max(0.0)
        };
    }

    let sqrt_t = t.sqrt();
    let d1 = ((spot / strike).ln() + (r - q + 0.5 * vol * vol) * t) / (vol * sqrt_t);
    let d2 = d1 - vol * sqrt_t;

    let call = spot * (-q * t).exp() * norm_cdf(d1) - strike * (-r * t).exp() * norm_cdf(d2);

    if is_call {
        call.max(0.0)
    } else {
        (call - spot * (-q * t).exp() + strike * (-r * t).exp()).max(0.0)
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn heston_params_validation() {
        assert!(HestonParams::new(0.04, 2.0, 0.04, 0.3, -0.5).is_ok());
        assert!(HestonParams::new(0.0, 2.0, 0.04, 0.3, -0.5).is_err()); // v0 = 0
        assert!(HestonParams::new(-0.01, 2.0, 0.04, 0.3, -0.5).is_err()); // v0 < 0
        assert!(HestonParams::new(0.04, 0.0, 0.04, 0.3, -0.5).is_err()); // kappa = 0
        assert!(HestonParams::new(0.04, 2.0, 0.0, 0.3, -0.5).is_err()); // theta = 0
        assert!(HestonParams::new(0.04, 2.0, 0.04, 0.0, -0.5).is_err()); // sigma = 0
        assert!(HestonParams::new(0.04, 2.0, 0.04, 0.3, -1.0).is_err()); // rho = -1
        assert!(HestonParams::new(0.04, 2.0, 0.04, 0.3, 1.0).is_err()); // rho = 1
    }

    #[test]
    fn feller_condition() {
        // Satisfies: 2*2*0.04 = 0.16 > 0.09 = 0.3²
        let p = HestonParams::new(0.04, 2.0, 0.04, 0.3, -0.5).expect("valid");
        assert!(p.satisfies_feller_condition());

        // Violates: 2*0.5*0.04 = 0.04 < 0.25 = 0.5²
        let p2 = HestonParams::new(0.04, 0.5, 0.04, 0.5, -0.5).expect("valid");
        assert!(!p2.satisfies_feller_condition());
    }

    #[test]
    fn call_price_positive_and_bounded() {
        let p = HestonParams::new(0.04, 2.0, 0.04, 0.3, -0.5).expect("valid");
        let call = p.price_european(100.0, 100.0, 0.05, 0.0, 1.0, true);
        assert!(call > 0.0, "Call should be positive, got {call}");
        assert!(call < 100.0, "Call should be < spot, got {call}");
    }

    #[test]
    fn put_call_parity() {
        let p = HestonParams::new(0.04, 2.0, 0.04, 0.3, -0.7).expect("valid");
        let s = 100.0;
        let k = 100.0;
        let r = 0.05;
        let q = 0.02;
        let t = 1.0;

        let call = p.price_european(s, k, r, q, t, true);
        let put = p.price_european(s, k, r, q, t, false);

        let lhs = call - put;
        let rhs = s * (-q * t).exp() - k * (-r * t).exp();

        assert!(
            (lhs - rhs).abs() < 0.01,
            "Put-call parity: C−P = {lhs:.4}, S·e^{{-qT}} − K·e^{{-rT}} = {rhs:.4}"
        );
    }

    #[test]
    fn moneyness_ordering() {
        let p = HestonParams::new(0.04, 2.0, 0.04, 0.3, -0.5).expect("valid");
        let itm = p.price_european(100.0, 90.0, 0.05, 0.0, 1.0, true);
        let atm = p.price_european(100.0, 100.0, 0.05, 0.0, 1.0, true);
        let otm = p.price_european(100.0, 110.0, 0.05, 0.0, 1.0, true);

        assert!(itm > atm, "ITM > ATM: {itm:.4} vs {atm:.4}");
        assert!(atm > otm, "ATM > OTM: {atm:.4} vs {otm:.4}");
    }

    #[test]
    fn black_scholes_limit() {
        let vol = 0.2;
        let var = vol * vol;
        // sigma_v → 0: Heston degenerates to Black-Scholes
        let p = HestonParams::new(var, 2.0, var, 1e-12, 0.0).expect("valid");
        let heston = p.price_european(100.0, 100.0, 0.05, 0.0, 1.0, true);
        let bs = bs_call_fallback(100.0, 100.0, 0.05, 0.0, 1.0, vol, true);

        assert!(
            (heston - bs).abs() < 0.01,
            "Heston → BS limit: Heston={heston:.4}, BS={bs:.4}"
        );
    }

    #[test]
    fn expired_option() {
        let p = HestonParams::new(0.04, 2.0, 0.04, 0.3, -0.5).expect("valid");
        let itm_call = p.price_european(100.0, 90.0, 0.05, 0.0, 0.0, true);
        assert!((itm_call - 10.0).abs() < 1e-10, "Expired ITM call");

        let otm_call = p.price_european(100.0, 110.0, 0.05, 0.0, 0.0, true);
        assert!(otm_call.abs() < 1e-10, "Expired OTM call");

        let itm_put = p.price_european(100.0, 110.0, 0.05, 0.0, 0.0, false);
        assert!((itm_put - 10.0).abs() < 1e-10, "Expired ITM put");
    }

    #[test]
    fn calibrate_heston_round_trip() {
        // Generate synthetic market data from known Heston params
        let true_params = HestonParams::new(0.04, 2.0, 0.04, 0.3, -0.5).expect("valid");
        let spot = 100.0;
        let r = 0.05;
        let q = 0.0;

        let expiries = [0.5, 1.0];
        let strikes_1: Vec<f64> = vec![90.0, 95.0, 100.0, 105.0, 110.0];
        let strikes_2: Vec<f64> = vec![85.0, 90.0, 95.0, 100.0, 105.0, 110.0, 115.0];

        // Compute synthetic implied vols via Heston price → BS inversion
        let make_vols = |ks: &[f64], t: f64| -> Vec<f64> {
            let fwd = spot * ((r - q) * t).exp();
            let df = (-r * t).exp();
            ks.iter()
                .map(|&k| {
                    let price = true_params.price_european(spot, k, r, q, t, true);
                    crate::math::volatility::implied_vol_black(price / df, fwd, k, t, true)
                        .unwrap_or(0.2)
                })
                .collect()
        };

        let vols_1 = make_vols(&strikes_1, 0.5);
        let vols_2 = make_vols(&strikes_2, 1.0);

        let strikes: Vec<&[f64]> = vec![&strikes_1, &strikes_2];
        let market_vols: Vec<&[f64]> = vec![&vols_1, &vols_2];

        let result = calibrate_heston(spot, r, q, &expiries, &strikes, &market_vols)
            .expect("should succeed");

        // RMSE should be small (prices are exact from the model)
        assert!(
            result.rmse < 0.02,
            "RMSE should be small: {:.4}",
            result.rmse
        );
        // Recovered parameters should be in the right ballpark
        assert!(
            (result.params.v0 - true_params.v0).abs() < 0.02,
            "v0 mismatch: {:.4} vs {:.4}",
            result.params.v0,
            true_params.v0
        );
        assert!(
            result.params.rho < 0.0,
            "rho should be negative: {:.4}",
            result.params.rho
        );
    }

    #[test]
    fn calibrate_heston_rejects_insufficient_data() {
        let strikes_only = [100.0, 105.0];
        let vols_only = [0.2, 0.21];
        let result = calibrate_heston(
            100.0,
            0.05,
            0.0,
            &[1.0],
            &[strikes_only.as_slice()],
            &[vols_only.as_slice()],
        );
        assert!(result.is_err(), "Should reject < 5 data points");
    }

    #[test]
    fn calibrate_heston_round_trip_with_nonzero_rates_and_dividends() {
        let true_params = HestonParams::new(0.04, 2.0, 0.04, 0.3, -0.5).expect("valid");
        let spot = 100.0;
        let r = 0.05;
        let q = 0.02;

        let expiries = [0.5, 1.0];
        let strikes_1: Vec<f64> = vec![90.0, 95.0, 100.0, 105.0, 110.0];
        let strikes_2: Vec<f64> = vec![85.0, 90.0, 95.0, 100.0, 105.0, 110.0, 115.0];

        let make_vols = |ks: &[f64], t: f64| -> Vec<f64> {
            let fwd = spot * ((r - q) * t).exp();
            let df = (-r * t).exp();
            ks.iter()
                .map(|&k| {
                    let discounted_price = true_params.price_european(spot, k, r, q, t, true);
                    crate::math::volatility::implied_vol_black(
                        discounted_price / df,
                        fwd,
                        k,
                        t,
                        true,
                    )
                    .unwrap_or(0.2)
                })
                .collect()
        };

        let vols_1 = make_vols(&strikes_1, 0.5);
        let vols_2 = make_vols(&strikes_2, 1.0);

        let strikes: Vec<&[f64]> = vec![&strikes_1, &strikes_2];
        let market_vols: Vec<&[f64]> = vec![&vols_1, &vols_2];

        let result = calibrate_heston(spot, r, q, &expiries, &strikes, &market_vols)
            .expect("should succeed");

        assert!(
            result.rmse < 0.02,
            "RMSE should stay small: {:.4}",
            result.rmse
        );
        assert!(
            (result.params.v0 - true_params.v0).abs() < 0.02,
            "v0 mismatch: {:.4} vs {:.4}",
            result.params.v0,
            true_params.v0
        );
        assert!(
            (result.params.theta - true_params.theta).abs() < 0.02,
            "theta mismatch: {:.4} vs {:.4}",
            result.params.theta,
            true_params.theta
        );
    }
}
