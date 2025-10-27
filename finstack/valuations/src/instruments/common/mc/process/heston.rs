//! Heston stochastic volatility model.
//!
//! The Heston model describes equity price dynamics with stochastic volatility:
//!
//! ```text
//! dS_t = (r - q) S_t dt + √v_t S_t dW_1(t)
//! dv_t = κ(θ - v_t) dt + σ_v √v_t dW_2(t)
//! ```
//!
//! where:
//! - S = spot price
//! - v = variance (volatility squared)
//! - κ = mean reversion speed
//! - θ = long-term variance
//! - σ_v = volatility of variance
//! - ρ = correlation between W_1 and W_2
//!
//! Reference: Heston (1993) - "A Closed-Form Solution for Options with Stochastic Volatility"

use super::super::traits::StochasticProcess;

/// Heston model parameters.
#[derive(Clone, Debug)]
pub struct HestonParams {
    /// Risk-free rate
    pub r: f64,
    /// Dividend/foreign rate
    pub q: f64,
    /// Mean reversion speed
    pub kappa: f64,
    /// Long-term variance
    pub theta: f64,
    /// Volatility of variance
    pub sigma_v: f64,
    /// Correlation between asset and variance
    pub rho: f64,
    /// Initial variance
    pub v0: f64,
}

impl HestonParams {
    /// Create new Heston parameters.
    ///
    /// # Arguments
    ///
    /// * `r` - Risk-free rate
    /// * `q` - Dividend yield
    /// * `kappa` - Mean reversion speed (> 0)
    /// * `theta` - Long-term variance (> 0)
    /// * `sigma_v` - Vol-of-vol (> 0)
    /// * `rho` - Correlation in [-1, 1]
    /// * `v0` - Initial variance (> 0)
    pub fn new(r: f64, q: f64, kappa: f64, theta: f64, sigma_v: f64, rho: f64, v0: f64) -> Self {
        assert!(kappa > 0.0, "kappa must be positive");
        assert!(theta > 0.0, "theta must be positive");
        assert!(sigma_v > 0.0, "sigma_v must be positive");
        assert!((-1.0..=1.0).contains(&rho), "rho must be in [-1, 1]");
        assert!(v0 > 0.0, "v0 must be positive");

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

    /// Check Feller condition: 2κθ > σ_v²
    ///
    /// When satisfied, the variance process stays strictly positive.
    pub fn satisfies_feller(&self) -> bool {
        2.0 * self.kappa * self.theta > self.sigma_v * self.sigma_v
    }
}

/// Heston stochastic volatility process.
///
/// State: [S, v] (spot and variance)
/// Factors: 2 correlated Brownian motions
#[derive(Clone, Debug)]
pub struct HestonProcess {
    params: HestonParams,
}

impl HestonProcess {
    /// Create a new Heston process.
    pub fn new(params: HestonParams) -> Self {
        Self { params }
    }

    /// Create with explicit parameters.
    pub fn with_params(
        r: f64,
        q: f64,
        kappa: f64,
        theta: f64,
        sigma_v: f64,
        rho: f64,
        v0: f64,
    ) -> Self {
        Self::new(HestonParams::new(r, q, kappa, theta, sigma_v, rho, v0))
    }

    /// Get parameters.
    pub fn params(&self) -> &HestonParams {
        &self.params
    }
}

impl StochasticProcess for HestonProcess {
    fn dim(&self) -> usize {
        2 // S and v
    }

    fn num_factors(&self) -> usize {
        2 // Two Brownian motions
    }

    fn drift(&self, _t: f64, x: &[f64], out: &mut [f64]) {
        let s = x[0];
        let v = x[1];

        // dS/dt = (r - q) S
        out[0] = (self.params.r - self.params.q) * s;

        // dv/dt = κ(θ - v)
        out[1] = self.params.kappa * (self.params.theta - v);
    }

    fn diffusion(&self, _t: f64, x: &[f64], out: &mut [f64]) {
        let s = x[0];
        let v = x[1].max(0.0); // Ensure non-negative for sqrt

        // Diffusion for S: √v S
        out[0] = v.sqrt() * s;

        // Diffusion for v: σ_v √v
        out[1] = self.params.sigma_v * v.sqrt();
    }

    fn is_diagonal(&self) -> bool {
        // Heston has correlation, so not diagonal
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heston_params() {
        let params = HestonParams::new(
            0.05,  // r
            0.02,  // q
            2.0,   // kappa
            0.04,  // theta
            0.3,   // sigma_v
            -0.5,  // rho
            0.04,  // v0
        );

        assert_eq!(params.kappa, 2.0);
        assert!(params.satisfies_feller());
    }

    #[test]
    fn test_feller_condition() {
        let params_feller = HestonParams::new(0.05, 0.02, 2.0, 0.04, 0.2, -0.5, 0.04);
        assert!(params_feller.satisfies_feller());

        let params_no_feller = HestonParams::new(0.05, 0.02, 0.5, 0.04, 0.5, -0.5, 0.04);
        assert!(!params_no_feller.satisfies_feller());
    }

    #[test]
    fn test_heston_drift_diffusion() {
        let heston = HestonProcess::with_params(0.05, 0.02, 2.0, 0.04, 0.3, -0.5, 0.04);

        let x = vec![100.0, 0.04];
        let mut drift = vec![0.0; 2];
        let mut diffusion = vec![0.0; 2];

        heston.drift(0.0, &x, &mut drift);
        heston.diffusion(0.0, &x, &mut diffusion);

        // S drift: (r-q)S = 0.03 * 100 = 3.0
        assert!((drift[0] - 3.0).abs() < 1e-10);

        // v drift: κ(θ-v) = 2.0 * (0.04 - 0.04) = 0
        assert!((drift[1] - 0.0).abs() < 1e-10);

        // S diffusion: √v S = √0.04 * 100 = 0.2 * 100 = 20
        assert!((diffusion[0] - 20.0).abs() < 1e-10);

        // v diffusion: σ_v √v = 0.3 * 0.2 = 0.06
        assert!((diffusion[1] - 0.06).abs() < 1e-10);
    }

    #[test]
    #[should_panic]
    fn test_invalid_params_negative_kappa() {
        HestonParams::new(0.05, 0.02, -1.0, 0.04, 0.3, -0.5, 0.04);
    }

    #[test]
    #[should_panic]
    fn test_invalid_params_rho_out_of_range() {
        HestonParams::new(0.05, 0.02, 2.0, 0.04, 0.3, 1.5, 0.04);
    }
}
