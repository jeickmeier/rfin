//! Predictor-corrector discretization for the LMM/BGM forward rate model.
//!
//! Implements the Glasserman (2003) predictor-corrector scheme which averages
//! drift evaluations at the current and predicted forward states to reduce
//! discretization bias. Only "alive" forwards (those with fixing date T_i > t)
//! are evolved; dead forwards are frozen at their last value.
//!
//! # Algorithm (one time step t → t + dt)
//!
//! 1. Generate K standard normals `Z_1, …, Z_K` (provided by engine via `z`).
//! 2. **Predictor**: Euler step with drift from current forwards `F(t)`.
//! 3. **Corrector**: Recompute drift at predicted forwards, average with
//!    predictor drift.
//! 4. **Floor**: Clamp `F_i(t+dt) ≥ −d_i` (displaced diffusion floor).
//!
//! # References
//!
//! - Glasserman, P. (2003). *Monte Carlo Methods in Financial Engineering*,
//!   Ch. 7, Springer.

use super::super::process::lmm::LmmProcess;
use super::super::traits::{Discretization, StochasticProcess};

/// Predictor-corrector discretization for the [`LmmProcess`].
///
/// Each time step evaluates the terminal-measure drift twice (at the current
/// and predicted states) and uses their average together with the diffusion
/// shocks applied only once.
#[derive(Debug, Clone)]
pub struct LmmPredictorCorrector;

impl LmmPredictorCorrector {
    /// Create a new predictor-corrector discretization.
    pub fn new() -> Self {
        Self
    }
}

impl Default for LmmPredictorCorrector {
    fn default() -> Self {
        Self::new()
    }
}

impl Discretization<LmmProcess> for LmmPredictorCorrector {
    fn step(
        &self,
        process: &LmmProcess,
        t: f64,
        dt: f64,
        x: &mut [f64],
        z: &[f64],
        work: &mut [f64],
    ) {
        let params = process.params();
        let n = params.num_forwards;
        let nf = params.num_factors;
        let sqrt_dt = dt.sqrt();
        let first = process.first_alive(t);

        // Work buffer layout:
        //   [0..n]         = drift at current state
        //   [n..2n]        = predicted forwards
        //   [2n..3n]       = drift at predicted state

        // --- Compute drift at current state ---
        let (drift_curr, rest) = work.split_at_mut(n);
        process.drift(t, x, drift_curr);

        // --- Compute diffusion increments and predictor step ---
        let (predicted, rest2) = rest.split_at_mut(n);

        // Copy dead forwards unchanged
        predicted[..first].copy_from_slice(&x[..first]);

        // Compute diffusion shocks for alive forwards only
        for i in first..n {
            let d_i_shifted = x[i] + params.displacements[i];
            let lam = params.factor_loadings(i, t);
            let mut diff_sum = 0.0;
            for k in 0..nf {
                diff_sum += lam[k] * z[k];
            }
            let diffusion_inc = d_i_shifted * diff_sum * sqrt_dt;

            // Predictor: F_i^pred = F_i + drift_curr_i * dt + diffusion_inc
            predicted[i] = x[i] + drift_curr[i] * dt + diffusion_inc;

            // Floor at displacement bound
            predicted[i] = predicted[i].max(-params.displacements[i]);
        }

        // --- Compute drift at predicted state ---
        let drift_pred = &mut rest2[..n];
        process.drift(t, predicted, drift_pred);

        // --- Corrector: average drifts, apply same diffusion (alive forwards only) ---
        for i in first..n {
            let d_i_shifted = x[i] + params.displacements[i];
            let lam = params.factor_loadings(i, t);
            let mut diff_sum = 0.0;
            for k in 0..nf {
                diff_sum += lam[k] * z[k];
            }
            let diffusion_inc = d_i_shifted * diff_sum * sqrt_dt;
            let avg_drift = 0.5 * (drift_curr[i] + drift_pred[i]);

            x[i] += avg_drift * dt + diffusion_inc;

            // Floor at displacement bound
            x[i] = x[i].max(-params.displacements[i]);
        }
    }

    fn work_size(&self, process: &LmmProcess) -> usize {
        let n = process.params().num_forwards;
        // drift_curr(n) + predicted(n) + drift_pred(n) = 3n
        3 * n
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::process::lmm::LmmParams;
    use crate::traits::Discretization;

    fn simple_params() -> LmmParams {
        LmmParams::try_new(
            3,
            2,
            vec![0.0, 1.0, 2.0, 3.0],
            vec![1.0, 1.0, 1.0],
            vec![0.005, 0.005, 0.005],
            vec![],
            vec![vec![
                [0.15, 0.05, 0.0],
                [0.12, 0.08, 0.0],
                [0.10, 0.10, 0.0],
            ]],
            vec![0.03, 0.03, 0.03],
        )
        .expect("valid")
    }

    #[test]
    fn test_single_step_preserves_positivity() {
        let params = simple_params();
        let process = LmmProcess::new(params);
        let disc = LmmPredictorCorrector::new();
        let ws = disc.work_size(&process);
        let mut work = vec![0.0; ws];
        let mut x = vec![0.03, 0.03, 0.03];
        let z = vec![0.5, -0.3]; // 2 normal shocks

        disc.step(&process, 0.0, 0.01, &mut x, &z, &mut work);

        // All forwards should remain > -displacement
        for (i, &f) in x.iter().enumerate() {
            assert!(f >= -0.005, "forward {i} below displacement floor: {f}");
        }
    }

    #[test]
    fn test_zero_shocks_drift_only() {
        let params = simple_params();
        let process = LmmProcess::new(params);
        let disc = LmmPredictorCorrector::new();
        let ws = disc.work_size(&process);
        let mut work = vec![0.0; ws];
        let mut x = vec![0.03, 0.03, 0.03];
        let z = vec![0.0, 0.0]; // zero shocks

        disc.step(&process, 0.0, 0.01, &mut x, &z, &mut work);

        // With zero shocks, only drift affects the forwards.
        // Terminal forward (index 2) should be unchanged (zero drift).
        assert!(
            (x[2] - 0.03).abs() < 1e-10,
            "terminal forward should be unchanged with zero shocks"
        );
    }

    #[test]
    fn test_dead_forwards_frozen() {
        let params = simple_params();
        let process = LmmProcess::new(params);
        let disc = LmmPredictorCorrector::new();
        let ws = disc.work_size(&process);
        let mut work = vec![0.0; ws];
        let mut x = vec![0.03, 0.03, 0.03];
        let z = vec![1.0, -1.0];

        // At t=1.5, forward 0 is dead (T_0=0.0) and forward 1 is dead (T_1=1.0)
        let x0_before = x[0];
        let x1_before = x[1];
        disc.step(&process, 1.5, 0.01, &mut x, &z, &mut work);

        assert!(
            (x[0] - x0_before).abs() < 1e-15,
            "dead forward 0 should be frozen"
        );
        assert!(
            (x[1] - x1_before).abs() < 1e-15,
            "dead forward 1 should be frozen"
        );
    }

    #[test]
    fn test_work_size() {
        let params = simple_params();
        let process = LmmProcess::new(params);
        let disc = LmmPredictorCorrector::new();
        assert_eq!(disc.work_size(&process), 9); // 3 * 3
    }
}
