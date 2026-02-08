//! Discretization schemes for Schwartz-Smith two-factor commodity model.
//!
//! Uses exact solutions for both components where possible, with correlation
//! handled via Cholesky decomposition.
//!
//! # Exact Solutions
//!
//! - **X (OU)**: X_{t+Δt} = X_t e^{-κ_X Δt} + σ_X √[(1-e^{-2κ_X Δt})/(2κ_X)] Z_X
//! - **Y (ABM)**: Y_{t+Δt} = Y_t + μ_Y Δt + σ_Y √Δt Z_Y
//!
//! With correlation ρ, the shocks Z_X and Z_Y are correlated.

use super::super::process::correlation::cholesky_decomposition;
use super::super::process::schwartz_smith::SchwartzSmithProcess;
use super::super::traits::Discretization;
use finstack_core::math::linalg::CholeskyError;

/// Exact discretization for Schwartz-Smith process.
///
/// Uses analytical solutions for both X (OU) and Y (arithmetic Brownian motion)
/// with correlation handled via Cholesky decomposition.
#[derive(Debug, Clone)]
pub struct ExactSchwartzSmith {
    /// Precomputed Cholesky factor for correlation [1, ρ; ρ, 1]
    cholesky_factor: [f64; 4], // 2x2 matrix stored as [L_00, L_01, L_10, L_11]
}

impl ExactSchwartzSmith {
    /// Create a new exact Schwartz-Smith discretization.
    ///
    /// # Arguments
    ///
    /// * `rho` - Correlation between X and Y Brownian motions
    pub fn new(rho: f64) -> finstack_core::Result<Self> {
        // Build 2x2 correlation matrix: [[1.0, rho], [rho, 1.0]]
        let corr_matrix = vec![1.0, rho, rho, 1.0];
        let chol = cholesky_decomposition(&corr_matrix, 2).map_err(|e| match e {
            CholeskyError::NotPositiveDefinite { .. } | CholeskyError::Singular { .. } => {
                finstack_core::Error::Input(finstack_core::InputError::Invalid)
            }
            CholeskyError::DimensionMismatch { .. } => {
                finstack_core::Error::Input(finstack_core::InputError::DimensionMismatch)
            }
            _ => finstack_core::Error::Input(finstack_core::InputError::Invalid),
        })?;

        // Store as array for efficiency
        let mut chol_array = [0.0; 4];
        chol_array.copy_from_slice(&chol);

        Ok(Self {
            cholesky_factor: chol_array,
        })
    }

    /// Create from Schwartz-Smith process (convenience method).
    pub fn from_process(process: &SchwartzSmithProcess) -> finstack_core::Result<Self> {
        Self::new(process.params().rho)
    }
}

impl Discretization<SchwartzSmithProcess> for ExactSchwartzSmith {
    fn step(
        &self,
        process: &SchwartzSmithProcess,
        _t: f64,
        dt: f64,
        x: &mut [f64],
        z: &[f64],
        _work: &mut [f64],
    ) {
        let params = process.params();
        let kappa_x = params.kappa_x;
        let sigma_x = params.sigma_x;
        let mu_y = params.mu_y;
        let sigma_y = params.sigma_y;

        // Apply correlation to independent shocks
        // z_corr = L * z_indep where L is 2x2 Cholesky factor
        let mut z_corr = [0.0; 2];
        z_corr[0] = self.cholesky_factor[0] * z[0] + self.cholesky_factor[1] * z[1];
        z_corr[1] = self.cholesky_factor[2] * z[0] + self.cholesky_factor[3] * z[1];

        // Exact solution for X (OU process)
        // X_{t+Δt} = X_t e^{-κ_X Δt} + σ_X √[(1-e^{-2κ_X Δt})/(2κ_X)] Z_X
        let exp_kappa_dt = (-kappa_x * dt).exp();
        let x_mean = x[0] * exp_kappa_dt;

        let x_std = if (kappa_x * dt).abs() < 1e-8 {
            // Taylor expansion for small κ_X Δt
            sigma_x * dt.sqrt() * (1.0 - kappa_x * dt / 3.0)
        } else {
            sigma_x * ((1.0 - (-2.0 * kappa_x * dt).exp()) / (2.0 * kappa_x)).sqrt()
        };

        x[0] = x_mean + x_std * z_corr[0];

        // Exact solution for Y (arithmetic Brownian motion)
        // Y_{t+Δt} = Y_t + μ_Y Δt + σ_Y √Δt Z_Y
        x[1] = x[1] + mu_y * dt + sigma_y * dt.sqrt() * z_corr[1];
    }

    fn work_size(&self, _process: &SchwartzSmithProcess) -> usize {
        0 // No workspace needed (correlation applied inline)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::super::super::process::schwartz_smith::{SchwartzSmithParams, SchwartzSmithProcess};
    use super::*;

    #[test]
    fn test_exact_schwartz_smith_creation() {
        let _params = SchwartzSmithParams::new(2.0, 0.30, 0.02, 0.15, -0.5);
        let disc = ExactSchwartzSmith::new(-0.5).expect("should succeed");

        assert_eq!(disc.cholesky_factor.len(), 4);
    }

    #[test]
    fn test_exact_schwartz_smith_step() {
        let params = SchwartzSmithParams::new(2.0, 0.30, 0.02, 0.15, -0.5);
        let process = SchwartzSmithProcess::new(params, 0.0, 4.5);
        let disc = ExactSchwartzSmith::from_process(&process).expect("should succeed");

        let mut x = [0.0, 4.5];
        let z = [0.0, 0.0]; // No shock
        let mut work = vec![];

        disc.step(&process, 0.0, 1.0, &mut x, &z, &mut work);

        // With z=0, X should decay: X(1) = 0 * exp(-2) = 0
        assert!((x[0] - 0.0).abs() < 1e-10);
        // Y should drift: Y(1) = 4.5 + 0.02 * 1 = 4.52
        assert!((x[1] - 4.52).abs() < 1e-10);
    }

    #[test]
    fn test_exact_schwartz_smith_spot_computation() {
        let params = SchwartzSmithParams::new(2.0, 0.30, 0.02, 0.15, -0.5);
        let process = SchwartzSmithProcess::new(params, 0.0, 4.5);
        let disc = ExactSchwartzSmith::from_process(&process).expect("should succeed");

        let mut x = [0.0, 4.5];
        let z = [0.0, 0.0];
        let mut work = vec![];

        disc.step(&process, 0.0, 1.0, &mut x, &z, &mut work);

        let spot = process.spot_from_state(&x);
        // S = exp(X + Y) = exp(0 + 4.52) ≈ 91.8
        assert!(spot > 90.0 && spot < 92.0);
    }
}
