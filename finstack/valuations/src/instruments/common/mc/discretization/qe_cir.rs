//! Quadratic-Exponential (QE) scheme for CIR process.
//!
//! The QE scheme (Andersen, 2008) ensures positive variance while maintaining
//! accuracy for the square-root diffusion process.
//!
//! This is extracted from the Heston QE implementation and can be reused
//! for standalone CIR processes (short rates, intensities).
//!
//! Reference: Andersen (2008) - "Simple and efficient simulation of the Heston stochastic volatility model"

use super::super::process::cir::CirProcess;
use super::super::traits::Discretization;
use finstack_core::math::special_functions::norm_cdf;

/// QE discretization for CIR process.
///
/// Handles the square-root diffusion while ensuring positivity.
///
/// # Algorithm
///
/// For variance:
/// - Compute ψ = s²/m² (scaled variance)
/// - If ψ ≤ ψ_c: use power/gamma approximation
/// - If ψ > ψ_c: use exponential/uniform mixture
#[derive(Clone, Debug)]
pub struct QeCir {
    /// Critical value for ψ (default 1.5)
    psi_c: f64,
}

impl QeCir {
    /// Create a new QE CIR discretization.
    pub fn new() -> Self {
        Self { psi_c: 1.5 }
    }

    /// Create with custom ψ_c threshold.
    pub fn with_psi_c(psi_c: f64) -> Self {
        Self { psi_c }
    }

    /// Compute next variance using QE scheme.
    ///
    /// This is the core QE algorithm from Andersen (2008).
    fn step_variance(&self, v_t: f64, kappa: f64, theta: f64, sigma: f64, dt: f64, z: f64) -> f64 {
        // Ensure non-negative input
        let v_t = v_t.max(0.0);

        // Compute conditional mean and variance
        let exp_kappa_dt = (-kappa * dt).exp();
        let m = theta + (v_t - theta) * exp_kappa_dt;
        let s2 = v_t * sigma * sigma * exp_kappa_dt * (1.0 - exp_kappa_dt) / kappa
            + theta * sigma * sigma * (1.0 - exp_kappa_dt).powi(2) / (2.0 * kappa);

        // Compute ψ = s²/m²
        let psi = if m > 1e-10 { s2 / (m * m) } else { 0.0 };

        if psi <= self.psi_c {
            // Case A: Power/gamma approximation
            // Solve: 2ψ^(-1) - 1 + sqrt(2ψ^(-1)) sqrt(2ψ^(-1) - 1) = Φ(Z)
            let b_squared = 2.0 / psi - 1.0 + (2.0 / psi * (2.0 / psi - 1.0)).sqrt();
            let a = m / (1.0 + b_squared);

            // Transform standard normal to chi-squared-like
            let v_next = a * (z + b_squared.sqrt()).powi(2);
            v_next.max(0.0)
        } else {
            // Case B: Exponential/uniform mixture
            let p = (psi - 1.0) / (psi + 1.0);
            let beta = (1.0 - p) / m;

            // Inverse CDF method
            let u = norm_cdf(z);

            if u <= p {
                // Point mass at zero
                0.0
            } else {
                // Exponential part
                let v_next = ((1.0 - p) / (u - p)).ln() / beta;
                v_next.max(0.0)
            }
        }
    }
}

impl Default for QeCir {
    fn default() -> Self {
        Self::new()
    }
}

impl Discretization<CirProcess> for QeCir {
    fn step(
        &self,
        process: &CirProcess,
        _t: f64,
        dt: f64,
        x: &mut [f64],
        z: &[f64],
        _work: &mut [f64],
    ) {
        let params = process.params();
        let v_t = x[0].max(0.0);

        // Apply QE scheme
        let v_next = self.step_variance(v_t, params.kappa, params.theta, params.sigma, dt, z[0]);

        x[0] = v_next;
    }

    fn work_size(&self, _process: &CirProcess) -> usize {
        0 // No workspace needed
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::super::super::process::cir::{CirParams, CirProcess};
    use super::*;

    #[test]
    fn test_qe_cir_positivity() {
        let params = CirParams::new(0.5, 0.04, 0.1);
        let process = CirProcess::new(params);
        let disc = QeCir::new();

        let t: f64 = 0.0;
        let dt: f64 = 0.01;
        let mut x = vec![0.04];
        let mut work = vec![0.0; disc.work_size(&process)];

        // Test many shocks - variance should stay non-negative
        for shock in [-3.0, -2.0, -1.0, 0.0, 1.0, 2.0, 3.0] {
            let z = vec![shock];
            disc.step(&process, t, dt, &mut x, &z, &mut work);

            assert!(x[0] >= 0.0, "Variance should be non-negative");
        }
    }

    #[test]
    fn test_qe_cir_mean_reversion() {
        let params = CirParams::new(0.5, 0.04, 0.1);
        let process = CirProcess::new(params);
        let disc = QeCir::new();

        let t: f64 = 0.0;
        let dt: f64 = 0.1;
        let mut work = vec![0.0; disc.work_size(&process)];

        // Start above mean with no shock
        let mut x = vec![0.06];
        let z = vec![0.0];
        disc.step(&process, t, dt, &mut x, &z, &mut work);

        // Should move toward mean
        assert!(x[0] < 0.06);
        assert!(x[0] > 0.04);
    }

    #[test]
    fn test_qe_cir_feller_violation() {
        // Test case where Feller condition is violated
        let params = CirParams::new(0.1, 0.01, 0.2);
        assert!(!params.satisfies_feller());

        let process = CirProcess::new(params);
        let disc = QeCir::new();

        let t: f64 = 0.0;
        let dt: f64 = 0.05;
        let mut x = vec![0.005];
        let z = vec![-2.0]; // Large negative shock
        let mut work = vec![0.0; disc.work_size(&process)];

        disc.step(&process, t, dt, &mut x, &z, &mut work);

        // QE should handle this gracefully and maintain non-negativity
        assert!(x[0] >= 0.0);
    }
}
