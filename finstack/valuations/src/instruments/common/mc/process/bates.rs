//! Bates model (Heston stochastic volatility + Merton jumps).
//!
//! Combines stochastic volatility (Heston) with Poisson jumps (Merton)
//! for modeling equity derivatives with:
//! - Stochastic volatility smile
//! - Fat tails from jumps
//! - Volatility clustering
//!
//! # Bates SDE
//!
//! ```text
//! dS_t/S_t = (r - q - λk)dt + √v_t dW_t^S + (J-1)dN_t
//! dv_t = κ(θ - v_t)dt + σ_v√v_t dW_t^v
//! ```
//!
//! where:
//! - v_t = stochastic variance (CIR process)
//! - Corr(W^S, W^v) = ρ
//! - λ = jump intensity
//! - J ~ LogNormal(μ_J, σ_J)

use super::super::traits::StochasticProcess;
use super::heston::{HestonParams, HestonProcess};
use super::jump_diffusion::MertonJumpParams;

/// Bates model parameters (Heston + jumps).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BatesParams {
    /// Heston parameters (spot dynamics + variance)
    pub heston: HestonParams,
    /// Jump parameters (intensity, distribution)
    pub jump: MertonJumpParams,
}

impl BatesParams {
    /// Create new Bates parameters.
    pub fn new(heston: HestonParams, jump: MertonJumpParams) -> Self {
        // Ensure consistency: r, q, and base sigma should align
        assert!(
            (heston.r - jump.gbm.r).abs() < 1e-12,
            "Risk-free rate must match between Heston and jump params"
        );
        assert!(
            (heston.q - jump.gbm.q).abs() < 1e-12,
            "Dividend yield must match between Heston and jump params"
        );

        Self { heston, jump }
    }

    /// Compensated drift for risk-neutral measure.
    pub fn compensated_drift(&self) -> f64 {
        self.heston.r - self.heston.q - self.jump.lambda * self.jump.jump_compensation()
    }
}

/// Bates process (Heston + Merton jumps).
///
/// State dimension: 2 (spot S, variance v)
/// Factor dimension: 3+ (diffusion for S, diffusion for v, Poisson/jump shocks)
///
/// # Usage
///
/// Requires `BatesDiscretization` that combines:
/// - QE scheme for variance (from Heston)
/// - Jump-augmented scheme for spot
/// - Correlation between S and v
#[derive(Debug, Clone)]
pub struct BatesProcess {
    params: BatesParams,
}

impl BatesProcess {
    /// Create a new Bates process.
    pub fn new(params: BatesParams) -> Self {
        Self { params }
    }

    /// Get parameters.
    pub fn params(&self) -> &BatesParams {
        &self.params
    }

    /// Get Heston component.
    pub fn heston(&self) -> HestonProcess {
        HestonProcess::new(self.params.heston.clone())
    }
}

impl StochasticProcess for BatesProcess {
    fn dim(&self) -> usize {
        2 // Spot and variance
    }

    fn num_factors(&self) -> usize {
        3 // S diffusion, v diffusion, Poisson/jumps
    }

    fn drift(&self, _t: f64, x: &[f64], out: &mut [f64]) {
        let s = x[0];
        let v = x[1].max(0.0);

        // Spot drift: (r - q - λk) S
        out[0] = self.params.compensated_drift() * s;

        // Variance drift: κ(θ - v)
        out[1] = self.params.heston.kappa * (self.params.heston.theta - v);
    }

    fn diffusion(&self, _t: f64, x: &[f64], out: &mut [f64]) {
        let s = x[0];
        let v = x[1].max(0.0);

        // Spot diffusion: √v S (stochastic vol, no jump term here)
        out[0] = v.sqrt() * s;

        // Variance diffusion: σ_v √v
        out[1] = self.params.heston.sigma_v * v.sqrt();
    }

    fn is_diagonal(&self) -> bool {
        // Non-diagonal due to spot-variance correlation (rho), consistent with Heston.
        // Process-specific discretizations (QE-Heston) handle the correlation internally.
        false
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_bates_params() {
        let heston = HestonParams::new(0.05, 0.02, 0.5, 0.04, 0.3, -0.7, 0.04);
        let jump = MertonJumpParams::new(0.05, 0.02, 0.0, 1.0, -0.05, 0.1);

        let bates = BatesParams::new(heston, jump);

        assert_eq!(bates.heston.r, 0.05);
        assert_eq!(bates.jump.lambda, 1.0);
    }

    #[test]
    fn test_bates_compensated_drift() {
        let heston = HestonParams::new(0.05, 0.02, 0.5, 0.04, 0.3, -0.7, 0.04);
        let jump = MertonJumpParams::new(0.05, 0.02, 0.0, 2.0, 0.0, 0.05);

        let bates = BatesParams::new(heston, jump);

        let drift = bates.compensated_drift();

        // Should be r - q - λk
        let expected = 0.05 - 0.02 - bates.jump.lambda * bates.jump.jump_compensation();
        assert!((drift - expected).abs() < 1e-10);
    }

    #[test]
    fn test_bates_process_drift() {
        let heston = HestonParams::new(0.05, 0.02, 0.5, 0.04, 0.3, -0.7, 0.04);
        let jump = MertonJumpParams::new(0.05, 0.02, 0.0, 1.0, -0.02, 0.08);
        let bates_params = BatesParams::new(heston, jump);

        let process = BatesProcess::new(bates_params);

        let x = vec![100.0, 0.04]; // S=100, v=0.04
        let mut drift = vec![0.0, 0.0];

        process.drift(0.0, &x, &mut drift);

        // Spot drift should be compensated
        let expected_spot_drift = process.params().compensated_drift() * 100.0;
        assert!((drift[0] - expected_spot_drift).abs() < 1e-6);

        // Variance drift
        let expected_var_drift = 0.5 * (0.04 - 0.04);
        assert_eq!(drift[1], expected_var_drift);
    }

    #[test]
    fn test_bates_process_diffusion() {
        let heston = HestonParams::new(0.05, 0.02, 0.5, 0.04, 0.3, -0.7, 0.04);
        let jump = MertonJumpParams::new(0.05, 0.02, 0.0, 1.0, 0.0, 0.1);
        let bates_params = BatesParams::new(heston, jump);

        let process = BatesProcess::new(bates_params);

        let x = vec![100.0, 0.04]; // S=100, v=0.04
        let mut diffusion = vec![0.0, 0.0];

        process.diffusion(0.0, &x, &mut diffusion);

        // Spot diffusion: √v * S = √0.04 * 100 = 0.2 * 100 = 20
        assert_eq!(diffusion[0], 0.04_f64.sqrt() * 100.0);

        // Variance diffusion: σ_v * √v = 0.3 * 0.2 = 0.06
        assert_eq!(diffusion[1], 0.3 * 0.04_f64.sqrt());
    }
}
