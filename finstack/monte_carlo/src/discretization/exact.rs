//! Exact discretization schemes for processes with analytical solutions.
//!
//! These schemes produce numerically exact transitions (up to floating-point precision)
//! without discretization error.

use super::super::process::correlation::{cholesky_correlation, validate_correlation_matrix};
use super::super::process::gbm::{GbmProcess, MultiGbmProcess};
use super::super::traits::{Discretization, StochasticProcess};
use finstack_core::math::linalg::CholeskyError;

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
#[derive(Debug, Clone, Default)]
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
#[derive(Debug, Clone, Default)]
pub struct ExactMultiGbm;

impl ExactMultiGbm {
    /// Create a new exact multi-GBM discretization.
    pub fn new() -> Self {
        Self
    }
}

fn recover_rate_coefficients<P: StochasticProcess>(
    process: &P,
    t: f64,
    x: &[f64],
    drift: f64,
    diffusion: f64,
    index: usize,
) -> (f64, f64) {
    if !x[index].is_subnormal() {
        return (drift / x[index], diffusion / x[index]);
    }

    let mut probe_state = x.to_vec();
    probe_state[index] = if x[index].is_sign_negative() {
        -1.0
    } else {
        1.0
    };
    let mut probe_drift = vec![0.0; x.len()];
    let mut probe_diffusion = vec![0.0; x.len()];
    process.drift(t, &probe_state, &mut probe_drift);
    process.diffusion(t, &probe_state, &mut probe_diffusion);

    (probe_drift[index], probe_diffusion[index])
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
            // Zero is an absorbing boundary for multiplicative GBM dynamics, so
            // skip the rate reconstruction that would otherwise evaluate 0 / 0.
            if x[i] == 0.0 {
                continue;
            }

            let (mu, sigma) =
                recover_rate_coefficients(process, _t, x, drift_vec[i], diff_vec[i], i);

            let drift_term = (mu - 0.5 * sigma * sigma) * dt;
            let diffusion_term = sigma * dt.sqrt() * z[i];

            x[i] *= (drift_term + diffusion_term).exp();
        }
    }

    fn work_size(&self, process: &P) -> usize {
        2 * process.dim() // drift + diffusion vectors
    }
}

/// Exact discretization for multi-factor GBM with correlation.
///
/// Handles correlated Brownian motions by applying pivoted Cholesky decomposition
/// to transform independent shocks into correlated ones before applying
/// the exact GBM formula.
///
/// # Algorithm
///
/// 1. Generate independent shocks Z ~ N(0,1) for each asset
/// 2. Apply Cholesky factor: Z_corr = L * Z_indep (in original asset order)
/// 3. Apply exact GBM formula: S_i(t+dt) = S_i(t) exp((μ_i - ½σ_i²)dt + σ_i√dt Z_corr_i)
#[derive(Debug, Clone)]
pub struct ExactMultiGbmCorrelated {
    /// Pivoted Cholesky factor of correlation matrix in original asset ordering.
    cholesky_factor: finstack_core::math::linalg::CorrelationFactor,
    /// Dimension (number of assets)
    dim: usize,
}

impl ExactMultiGbmCorrelated {
    /// Create a new exact multi-GBM discretization with correlation.
    ///
    /// # Arguments
    ///
    /// * `correlation_matrix` - Correlation matrix (n x n, row-major, must be positive semi-definite)
    /// * `dim` - Number of assets
    ///
    /// # Errors
    ///
    /// Returns error if correlation matrix is indefinite or has wrong size.
    pub fn new(correlation_matrix: &[f64], dim: usize) -> finstack_core::Result<Self> {
        validate_correlation_matrix(correlation_matrix, dim)
            .map_err(|_| finstack_core::Error::Input(finstack_core::InputError::Invalid))?;
        let cholesky_factor =
            cholesky_correlation(correlation_matrix, dim).map_err(|e| match e {
                CholeskyError::NotPositiveDefinite { .. } => {
                    finstack_core::Error::Input(finstack_core::InputError::Invalid)
                }
                CholeskyError::DimensionMismatch { .. } => {
                    finstack_core::Error::Input(finstack_core::InputError::DimensionMismatch)
                }
                _ => finstack_core::Error::Input(finstack_core::InputError::Invalid),
            })?;
        Ok(Self {
            cholesky_factor,
            dim,
        })
    }

    /// Create from a MultiGbmProcess (convenience method).
    ///
    /// Returns `None` if the process has no correlation (use `ExactMultiGbm` instead).
    pub fn from_process(process: &MultiGbmProcess) -> finstack_core::Result<Option<Self>> {
        if let Some(corr) = process.correlation() {
            Ok(Some(Self::new(corr, process.dim())?))
        } else {
            Ok(None)
        }
    }
}

impl Discretization<MultiGbmProcess> for ExactMultiGbmCorrelated {
    fn step(
        &self,
        process: &MultiGbmProcess,
        _t: f64,
        dt: f64,
        x: &mut [f64],
        z: &[f64],
        work: &mut [f64],
    ) {
        let dim = process.dim();
        assert_eq!(dim, self.dim, "Process dimension must match discretization");

        // Split work buffer: [drift_vec | diff_vec | z_corr]
        let (drift_vec, rest) = work.split_at_mut(dim);
        let (diff_vec, z_corr) = rest.split_at_mut(dim);

        // Get drift and diffusion coefficients
        process.drift(_t, x, drift_vec);
        process.diffusion(_t, x, diff_vec);

        // Apply Cholesky factor to get correlated shocks in original asset order.
        // Dimensions are guaranteed by construction: cholesky_factor is dim×dim and
        // work_size() allocates 3*dim so z_corr has length dim.
        self.cholesky_factor.apply(z, z_corr);

        // Apply exact GBM formula for each component using correlated shocks
        // S_i(t+dt) = S_i(t) exp((μ_i - ½σ_i²)dt + σ_i√dt Z_corr_i)
        for i in 0..dim {
            // Zero is an absorbing boundary for multiplicative GBM dynamics, so
            // skip the rate reconstruction that would otherwise evaluate 0 / 0.
            if x[i] == 0.0 {
                continue;
            }

            let (mu, sigma) =
                recover_rate_coefficients(process, _t, x, drift_vec[i], diff_vec[i], i);

            let drift_term = (mu - 0.5 * sigma * sigma) * dt;
            let diffusion_term = sigma * dt.sqrt() * z_corr[i];

            x[i] *= (drift_term + diffusion_term).exp();
        }
    }

    fn work_size(&self, process: &MultiGbmProcess) -> usize {
        // drift + diffusion + correlated shocks
        3 * process.dim()
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::super::super::process::gbm::{GbmParams, GbmProcess, MultiGbmProcess};
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

    #[test]
    fn test_exact_multi_gbm_preserves_zero_and_evolves_small_positive_state() {
        let params = vec![
            GbmParams::new(0.05, 0.02, 0.2),
            GbmParams::new(0.05, 0.03, 0.3),
        ];
        let process = MultiGbmProcess::new(params, None);
        let disc = ExactMultiGbm::new();

        let small_positive = 1e-300;
        let mut x = vec![0.0, small_positive];
        let z = vec![0.0, 0.0];
        let mut work = vec![0.0; disc.work_size(&process)];

        disc.step(&process, 0.0, 1.0, &mut x, &z, &mut work);

        let expected = small_positive * (-0.025_f64).exp();
        assert_eq!(x[0], 0.0);
        assert!(x[0].is_finite());
        assert!(x[1].is_finite());
        assert!((x[1] - expected).abs() < 1e-312);
    }

    #[test]
    fn test_exact_multi_gbm_smallest_subnormal_can_round_to_zero() {
        let params = vec![
            GbmParams::new(0.05, 0.02, 0.2),
            GbmParams::new(0.05, 0.03, 0.3),
        ];
        let process = MultiGbmProcess::new(params, None);
        let disc = ExactMultiGbm::new();

        let smallest_subnormal = f64::from_bits(1);
        let mut x = vec![0.0, smallest_subnormal];
        let z = vec![0.0, -3.0];
        let mut work = vec![0.0; disc.work_size(&process)];

        disc.step(&process, 0.0, 1.0, &mut x, &z, &mut work);

        assert_eq!(x[0], 0.0);
        assert!(x[0].is_finite());
        assert_eq!(x[1], 0.0);
        assert!(x[1].is_finite());
    }

    #[test]
    fn test_exact_multi_gbm_recovers_mixed_underflow_coefficients() {
        let params = vec![
            GbmParams::new(0.05, 0.02, 0.2),
            GbmParams::new(0.05, 0.03, 1.5),
        ];
        let process = MultiGbmProcess::new(params, None);
        let disc = ExactMultiGbm::new();

        let smallest_subnormal = f64::from_bits(1);
        let mut x = vec![0.0, smallest_subnormal];
        let z = vec![0.0, 2.0];
        let mut work = vec![0.0; disc.work_size(&process)];

        disc.step(&process, 0.0, 1.0, &mut x, &z, &mut work);

        let expected = smallest_subnormal * 1.925_f64.exp();
        assert_eq!(x[0], 0.0);
        assert_eq!(x[1], expected);
        assert!(x[1] > smallest_subnormal);
    }

    #[test]
    fn test_exact_multi_gbm_correlated_creation() {
        let params = vec![
            GbmParams::new(0.05, 0.02, 0.2),
            GbmParams::new(0.05, 0.03, 0.3),
        ];
        // Correlation matrix: [[1.0, 0.5], [0.5, 1.0]]
        let corr = vec![1.0, 0.5, 0.5, 1.0];
        let multi_gbm = MultiGbmProcess::new(params, Some(corr.clone()));

        let disc = ExactMultiGbmCorrelated::new(&corr, 2).expect("should succeed");
        assert_eq!(disc.dim, 2);
        assert_eq!(disc.cholesky_factor.factor_matrix().len(), 4);

        // Test from_process convenience method
        let disc_from_process =
            ExactMultiGbmCorrelated::from_process(&multi_gbm).expect("should succeed");
        assert!(disc_from_process.is_some());
    }

    #[test]
    fn test_exact_multi_gbm_correlated_step() {
        let params = vec![
            GbmParams::new(0.05, 0.02, 0.2),
            GbmParams::new(0.05, 0.03, 0.3),
        ];
        // Correlation matrix: [[1.0, 0.5], [0.5, 1.0]]
        let corr = vec![1.0, 0.5, 0.5, 1.0];
        let multi_gbm = MultiGbmProcess::new(params, Some(corr.clone()));

        let disc = ExactMultiGbmCorrelated::new(&corr, 2).expect("should succeed");

        let mut x = vec![100.0, 200.0];
        let z = vec![0.0, 0.0]; // No shock - should get drift-only evolution
        let mut work = vec![0.0; disc.work_size(&multi_gbm)];

        disc.step(&multi_gbm, 0.0, 1.0, &mut x, &z, &mut work);

        // With z=0, should get drift-only evolution
        // S_1(1) = 100 * exp((0.05 - 0.02 - 0.5*0.2²)*1.0) = 100 * exp(0.01)
        // S_2(1) = 200 * exp((0.05 - 0.03 - 0.5*0.3²)*1.0) = 200 * exp(-0.025)
        let expected_1 = 100.0 * 0.01_f64.exp();
        let expected_2 = 200.0 * (-0.025_f64).exp();
        assert!((x[0] - expected_1).abs() < 1e-10);
        assert!((x[1] - expected_2).abs() < 1e-10);
    }

    #[test]
    fn test_exact_multi_gbm_correlated_preserves_zero_and_evolves_small_positive_state() {
        let params = vec![
            GbmParams::new(0.05, 0.02, 0.2),
            GbmParams::new(0.05, 0.03, 0.3),
        ];
        let corr = vec![1.0, 0.5, 0.5, 1.0];
        let process = MultiGbmProcess::new(params, Some(corr.clone()));
        let disc = ExactMultiGbmCorrelated::new(&corr, 2).expect("should succeed");

        let small_positive = 1e-300;
        let mut x = vec![0.0, small_positive];
        let z = vec![0.0, 0.0];
        let mut work = vec![0.0; disc.work_size(&process)];

        disc.step(&process, 0.0, 1.0, &mut x, &z, &mut work);

        let expected = small_positive * (-0.025_f64).exp();
        assert_eq!(x[0], 0.0);
        assert!(x[0].is_finite());
        assert!(x[1].is_finite());
        assert!((x[1] - expected).abs() < 1e-312);
    }

    #[test]
    fn test_exact_multi_gbm_correlated_smallest_subnormal_can_round_to_zero() {
        let params = vec![
            GbmParams::new(0.05, 0.02, 0.2),
            GbmParams::new(0.05, 0.03, 0.3),
        ];
        let corr = vec![1.0, 0.0, 0.0, 1.0];
        let process = MultiGbmProcess::new(params, Some(corr.clone()));
        let disc = ExactMultiGbmCorrelated::new(&corr, 2).expect("should succeed");

        let smallest_subnormal = f64::from_bits(1);
        let mut x = vec![0.0, smallest_subnormal];
        let z = vec![0.0, -3.0];
        let mut work = vec![0.0; disc.work_size(&process)];

        disc.step(&process, 0.0, 1.0, &mut x, &z, &mut work);

        assert_eq!(x[0], 0.0);
        assert!(x[0].is_finite());
        assert_eq!(x[1], 0.0);
        assert!(x[1].is_finite());
    }

    #[test]
    fn test_exact_multi_gbm_correlated_recovers_mixed_underflow_coefficients() {
        let params = vec![
            GbmParams::new(0.05, 0.02, 0.2),
            GbmParams::new(0.05, 0.03, 1.5),
        ];
        let corr = vec![1.0, 0.0, 0.0, 1.0];
        let process = MultiGbmProcess::new(params, Some(corr.clone()));
        let disc = ExactMultiGbmCorrelated::new(&corr, 2).expect("should succeed");

        let smallest_subnormal = f64::from_bits(1);
        let mut x = vec![0.0, smallest_subnormal];
        let z = vec![0.0, 2.0];
        let mut work = vec![0.0; disc.work_size(&process)];

        disc.step(&process, 0.0, 1.0, &mut x, &z, &mut work);

        let expected = smallest_subnormal * 1.925_f64.exp();
        assert_eq!(x[0], 0.0);
        assert_eq!(x[1], expected);
        assert!(x[1] > smallest_subnormal);
    }

    #[test]
    fn test_exact_multi_gbm_correlated_with_shocks() {
        let params = vec![
            GbmParams::new(0.05, 0.02, 0.2),
            GbmParams::new(0.05, 0.03, 0.3),
        ];
        // Correlation matrix: [[1.0, 0.5], [0.5, 1.0]]
        let corr = vec![1.0, 0.5, 0.5, 1.0];
        let multi_gbm = MultiGbmProcess::new(params, Some(corr.clone()));

        let disc = ExactMultiGbmCorrelated::new(&corr, 2).expect("should succeed");

        let mut x = vec![100.0, 200.0];
        // Use a positive common-factor shock. This is robust to pivoted factorization,
        // which preserves covariance but can rotate the latent factor basis.
        let z = vec![1.0, 1.0];
        let mut work = vec![0.0; disc.work_size(&multi_gbm)];

        disc.step(&multi_gbm, 0.0, 1.0, &mut x, &z, &mut work);

        // Positive latent shocks should lift both assets above their drift-only levels.
        let drift_only_1 = 100.0 * 0.01_f64.exp();
        let drift_only_2 = 200.0 * (-0.025_f64).exp();
        assert!(x[0] > drift_only_1);
        assert!(x[1] > drift_only_2);
    }

    #[test]
    fn test_exact_multi_gbm_correlated_work_size() {
        let params = vec![
            GbmParams::new(0.05, 0.02, 0.2),
            GbmParams::new(0.05, 0.03, 0.3),
        ];
        let corr = vec![1.0, 0.5, 0.5, 1.0];
        let multi_gbm = MultiGbmProcess::new(params, Some(corr.clone()));

        let disc = ExactMultiGbmCorrelated::new(&corr, 2).expect("should succeed");
        assert_eq!(disc.work_size(&multi_gbm), 6); // 3 * 2 = 6 (drift + diffusion + z_corr)
    }

    #[test]
    fn test_exact_multi_gbm_correlated_from_process_no_correlation() {
        let params = vec![
            GbmParams::new(0.05, 0.02, 0.2),
            GbmParams::new(0.05, 0.03, 0.3),
        ];
        let multi_gbm = MultiGbmProcess::new(params, None); // No correlation

        let disc_from_process =
            ExactMultiGbmCorrelated::from_process(&multi_gbm).expect("should succeed");
        assert!(disc_from_process.is_none()); // Should return None for uncorrelated process
    }

    #[test]
    fn test_exact_multi_gbm_correlated_rejects_invalid_correlation_matrix() {
        let invalid = vec![1.0, 0.2, 0.4, 1.0];
        assert!(ExactMultiGbmCorrelated::new(&invalid, 2).is_err());
    }
}
