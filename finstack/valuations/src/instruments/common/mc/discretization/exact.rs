//! Exact discretization schemes for processes with analytical solutions.
//!
//! These schemes produce numerically exact transitions (up to floating-point precision)
//! without discretization error.

use super::super::process::gbm::GbmProcess;
use super::super::traits::{Discretization, StochasticProcess};

/// Exact discretization for Geometric Brownian Motion.
///
/// Uses the analytical log-normal solution:
///
/// ```text
/// S_{t+Δt} = S_t exp((r - q - ½σ²)Δt + σ√Δt Z)
/// ```
///
/// where Z ~ N(0,1).
///
/// This is numerically exact and has no discretization error.
#[derive(Clone, Debug, Default)]
pub struct ExactGbm;

impl ExactGbm {
    /// Create a new exact GBM discretization.
    pub fn new() -> Self {
        Self
    }
}

impl Discretization<GbmProcess> for ExactGbm {
    fn step(
        &self,
        process: &GbmProcess,
        _t: f64,
        dt: f64,
        x: &mut [f64],
        z: &[f64],
        _work: &mut [f64],
    ) {
        let params = process.params();

        // Drift component: (r - q - ½σ²)Δt
        let drift = (params.r - params.q - 0.5 * params.sigma * params.sigma) * dt;

        // Diffusion component: σ√Δt Z
        let diffusion = params.sigma * dt.sqrt() * z[0];

        // Apply log-normal update: S_{t+Δt} = S_t exp(drift + diffusion)
        x[0] *= (drift + diffusion).exp();
    }

    fn work_size(&self, _process: &GbmProcess) -> usize {
        0 // No workspace needed
    }
}

/// Exact discretization for multi-factor GBM.
///
/// Each factor evolves independently (or with correlation applied upstream).
#[derive(Clone, Debug, Default)]
pub struct ExactMultiGbm;

impl ExactMultiGbm {
    /// Create a new exact multi-GBM discretization.
    pub fn new() -> Self {
        Self
    }
}

impl<P> Discretization<P> for ExactMultiGbm
where
    P: StochasticProcess,
{
    fn step(&self, process: &P, _t: f64, dt: f64, x: &mut [f64], z: &[f64], work: &mut [f64]) {
        let dim = process.dim();

        // Get drift and diffusion coefficients
        let (drift_vec, diff_vec) = work.split_at_mut(dim);

        process.drift(_t, x, drift_vec);
        process.diffusion(_t, x, diff_vec);

        // Apply exact GBM formula for each component
        // For diagonal diffusion: S_i(t+dt) = S_i(t) exp((μ_i - ½σ_i²)dt + σ_i√dt Z_i)
        for i in 0..dim {
            let mu = drift_vec[i] / x[i]; // Convert absolute drift to rate
            let sigma = diff_vec[i] / x[i]; // Convert absolute vol to rate

            let drift_term = (mu - 0.5 * sigma * sigma) * dt;
            let diffusion_term = sigma * dt.sqrt() * z[i];

            x[i] *= (drift_term + diffusion_term).exp();
        }
    }

    fn work_size(&self, process: &P) -> usize {
        2 * process.dim() // drift + diffusion vectors
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::process::gbm::{GbmParams, GbmProcess};
    use super::*;

    #[test]
    fn test_exact_gbm_step() {
        let params = GbmParams::new(0.05, 0.02, 0.2);
        let process = GbmProcess::new(params);
        let disc = ExactGbm::new();

        let mut x = vec![100.0];
        let z = vec![0.0]; // No shock
        let mut work = vec![];

        disc.step(&process, 0.0, 1.0, &mut x, &z, &mut work);

        // With z=0, should get drift-only evolution
        // S(1) = 100 * exp((0.05 - 0.02 - 0.5*0.2²)*1.0 + 0) = 100 * exp(0.01)
        let expected = 100.0 * (0.01_f64).exp();
        assert!((x[0] - expected).abs() < 1e-10);
    }

    #[test]
    fn test_exact_gbm_with_shock() {
        let params = GbmParams::new(0.05, 0.02, 0.2);
        let process = GbmProcess::new(params);
        let disc = ExactGbm::new();

        let mut x = vec![100.0];
        let z = vec![1.0]; // +1 std dev shock
        let mut work = vec![];

        disc.step(&process, 0.0, 1.0, &mut x, &z, &mut work);

        // S(1) = 100 * exp((0.05 - 0.02 - 0.5*0.2²)*1.0 + 0.2*1.0*1.0)
        //      = 100 * exp(0.01 + 0.2) = 100 * exp(0.21)
        let expected = 100.0 * 0.21_f64.exp();
        assert!((x[0] - expected).abs() < 1e-10);
    }

    #[test]
    fn test_exact_gbm_multiple_steps() {
        let params = GbmParams::new(0.05, 0.0, 0.2);
        let process = GbmProcess::new(params);
        let disc = ExactGbm::new();

        let mut x = vec![100.0];
        let z_zero = vec![0.0];
        let mut work = vec![];

        // Take 10 steps of 0.1 each
        for _ in 0..10 {
            disc.step(&process, 0.0, 0.1, &mut x, &z_zero, &mut work);
        }

        // Should equal single step of 1.0
        let mut x_single = vec![100.0];
        disc.step(&process, 0.0, 1.0, &mut x_single, &z_zero, &mut work);

        assert!((x[0] - x_single[0]).abs() < 1e-10);
    }

    #[test]
    fn test_work_size() {
        let params = GbmParams::new(0.05, 0.02, 0.2);
        let process = GbmProcess::new(params);
        let disc = ExactGbm::new();

        assert_eq!(disc.work_size(&process), 0);
    }
}
