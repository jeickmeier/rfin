//! Exact discretization for Hull-White 1-factor model.
//!
//! Provides analytical solution for the OU/HW1F SDE with piecewise-constant θ(t).
//!
//! # SDE
//!
//! ```text
//! dr_t = κ[θ(t) - r_t]dt + σ dW_t
//! ```
//!
//! # Exact Solution
//!
//! For piecewise-constant θ over [t, t+Δt]:
//!
//! ```text
//! r_{t+Δt} = r_t e^{-κΔt} + θ(1 - e^{-κΔt}) + σ√[(1-e^{-2κΔt})/(2κ)] Z
//! ```
//!
//! where Z ~ N(0, 1).
//!
//! This is an exact solution (no discretization error), providing the best
//! accuracy for HW1F simulation.

use super::super::process::ou::HullWhite1FProcess;
use super::super::traits::Discretization;

/// Exact discretization for Hull-White 1-factor.
///
/// Uses the analytical solution to the OU SDE, providing zero discretization
/// error for the conditional distribution.
///
/// # Advantages
///
/// - Exact (no approximation error)
/// - Unconditionally stable (any step size)
/// - Fast evaluation (no iterations)
///
/// # Formula
///
/// ```text
/// r_{t+Δt} = E[r_{t+Δt}|r_t] + Std[r_{t+Δt}|r_t] * Z
///
/// where:
///   E[r_{t+Δt}|r_t] = r_t e^{-κΔt} + θ(1 - e^{-κΔt})
///   Std[r_{t+Δt}|r_t] = σ√[(1 - e^{-2κΔt}) / (2κ)]
/// ```
#[derive(Clone, Debug, Default)]
pub struct ExactHullWhite1F;

impl ExactHullWhite1F {
    /// Create a new exact HW1F discretization.
    pub fn new() -> Self {
        Self
    }
}

impl Discretization<HullWhite1FProcess> for ExactHullWhite1F {
    fn step(
        &self,
        process: &HullWhite1FProcess,
        t: f64,
        dt: f64,
        x: &mut [f64],
        z: &[f64],
        _work: &mut [f64],
    ) {
        let params = process.params();
        let kappa = params.kappa;
        let sigma = params.sigma;
        let theta = process.theta_at_time(t);

        // Compute exact conditional mean and standard deviation
        let exp_kappa_dt = (-kappa * dt).exp();

        // Conditional mean: E[r_{t+Δt}|r_t]
        let mean = x[0] * exp_kappa_dt + theta * (1.0 - exp_kappa_dt);

        // Conditional standard deviation: Std[r_{t+Δt}|r_t]
        // For small κΔt, use Taylor expansion to avoid numerical issues
        let std_dev = if (kappa * dt).abs() < 1e-8 {
            // Taylor: √[(1 - e^{-2κΔt})/(2κ)] ≈ √Δt [1 - κΔt/3 + ...]
            sigma * dt.sqrt() * (1.0 - kappa * dt / 3.0)
        } else {
            sigma * ((1.0 - (-2.0 * kappa * dt).exp()) / (2.0 * kappa)).sqrt()
        };

        // Exact step
        x[0] = mean + std_dev * z[0];
    }

    fn work_size(&self, _process: &HullWhite1FProcess) -> usize {
        0 // No workspace needed
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::super::super::process::ou::{HullWhite1FParams, HullWhite1FProcess};
    use super::*;

    #[test]
    fn test_exact_hw1f_mean_reversion() {
        let params = HullWhite1FParams::new(0.1, 0.01, 0.03);
        let process = HullWhite1FProcess::new(params);
        let disc = ExactHullWhite1F::new();

        let t: f64 = 0.0;
        let dt: f64 = 1.0;
        let mut x = vec![0.05]; // Start above mean
        let z = vec![0.0]; // No shock
        let mut work = vec![0.0; disc.work_size(&process)];

        disc.step(&process, t, dt, &mut x, &z, &mut work);

        // Should move toward mean (0.03)
        assert!(x[0] < 0.05);
        assert!(x[0] > 0.03);

        // Check exact formula
        let expected: f64 = 0.05 * (-0.1_f64 * 1.0).exp() + 0.03 * (1.0 - (-0.1_f64 * 1.0).exp());
        assert!((x[0] - expected).abs() < 1e-10);
    }

    #[test]
    fn test_exact_hw1f_positive_shock() {
        let params = HullWhite1FParams::new(0.1, 0.01, 0.03);
        let process = HullWhite1FProcess::new(params);
        let disc = ExactHullWhite1F::new();

        let t: f64 = 0.0;
        let dt: f64 = 0.1;
        let mut x = vec![0.03]; // Start at mean
        let z = vec![2.0]; // Positive shock
        let mut work = vec![0.0; disc.work_size(&process)];

        disc.step(&process, t, dt, &mut x, &z, &mut work);

        // With positive shock, rate should increase
        assert!(x[0] > 0.03);

        // Check that volatility term is applied correctly
        let std_dev: f64 = 0.01 * ((1.0 - (-2.0_f64 * 0.1 * 0.1).exp()) / (2.0 * 0.1)).sqrt();
        let expected_move = std_dev * 2.0; // 2 standard deviations

        // Rate should be approximately mean + 2*std
        assert!((x[0] - (0.03 + expected_move)).abs() < 0.001);
    }

    #[test]
    fn test_exact_hw1f_time_dependent_theta() {
        let theta_curve = vec![0.02, 0.04];
        let theta_times = vec![0.0, 0.5];

        let params =
            HullWhite1FParams::with_time_dependent_theta(0.1, 0.01, theta_curve, theta_times);
        let process = HullWhite1FProcess::new(params);
        let disc = ExactHullWhite1F::new();

        // Step in first regime (θ = 0.02)
        let t1: f64 = 0.0;
        let dt: f64 = 0.1;
        let mut x1 = vec![0.03];
        let z = vec![0.0];
        let mut work = vec![0.0; disc.work_size(&process)];

        disc.step(&process, t1, dt, &mut x1, &z, &mut work);

        let expected1: f64 = 0.03 * (-0.1_f64 * 0.1).exp() + 0.02 * (1.0 - (-0.1_f64 * 0.1).exp());
        assert!((x1[0] - expected1).abs() < 1e-10);

        // Step in second regime (θ = 0.04)
        let t2: f64 = 0.6;
        let mut x2 = vec![0.03];
        disc.step(&process, t2, dt, &mut x2, &z, &mut work);

        let expected2: f64 = 0.03 * (-0.1_f64 * 0.1).exp() + 0.04 * (1.0 - (-0.1_f64 * 0.1).exp());
        assert!((x2[0] - expected2).abs() < 1e-10);

        // Should move toward different means
        assert!(x2[0] > x1[0]);
    }
}
