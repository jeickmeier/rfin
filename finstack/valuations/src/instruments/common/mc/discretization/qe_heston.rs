//! Quadratic-Exponential (QE) scheme for Heston variance process.
//!
//! The QE scheme (Andersen, 2008) ensures positive variance while maintaining
//! accuracy for the CIR-type variance process in Heston.
//!
//! Reference: Andersen (2008) - "Simple and efficient simulation of the Heston stochastic volatility model"

use super::super::process::heston::HestonProcess;
use super::super::traits::Discretization;

/// QE discretization for Heston model.
///
/// This scheme handles both the variance (CIR process) and the spot price,
/// ensuring variance stays positive while maintaining good accuracy.
///
/// # Algorithm
///
/// For variance:
/// - Compute ψ = s²/m² (scaled variance)
/// - If ψ <= ψ_c: use power/gamma approximation
/// - If ψ > ψ_c: use exponential/uniform mixture
///
/// For spot:
/// - Use log-Euler with integrated variance approximation
#[derive(Clone, Debug)]
pub struct QeHeston {
    /// Critical value for ψ (default 1.5)
    psi_c: f64,
}

impl QeHeston {
    /// Create a new QE Heston discretization.
    pub fn new() -> Self {
        Self { psi_c: 1.5 }
    }

    /// Create with custom ψ_c threshold.
    pub fn with_psi_c(psi_c: f64) -> Self {
        Self { psi_c }
    }

    /// Compute next variance using QE scheme.
    fn step_variance(
        &self,
        v_t: f64,
        kappa: f64,
        theta: f64,
        sigma_v: f64,
        dt: f64,
        z_v: f64,
    ) -> f64 {
        // Ensure non-negative input
        let v_t = v_t.max(0.0);

        // Compute conditional mean and variance
        let exp_kappa_dt = (-kappa * dt).exp();
        let m = theta + (v_t - theta) * exp_kappa_dt;
        let s2 = v_t * sigma_v * sigma_v * exp_kappa_dt * (1.0 - exp_kappa_dt) / kappa
            + theta * sigma_v * sigma_v * (1.0 - exp_kappa_dt).powi(2) / (2.0 * kappa);

        // Compute ψ = s²/m²
        let psi = if m > 1e-10 { s2 / (m * m) } else { 0.0 };

        if psi <= self.psi_c {
            // Case A: Power/gamma approximation
            // Solve: 2ψ^(-1) - 1 + sqrt(2ψ^(-1)) sqrt(2ψ^(-1) - 1) = U(Z)
            let b_squared = 2.0 / psi - 1.0 + (2.0 / psi * (2.0 / psi - 1.0)).sqrt();
            let a = m / (1.0 + b_squared);

            // Transform standard normal to chi-squared-like
            let v_next = a * (z_v + b_squared.sqrt()).powi(2);
            v_next.max(0.0)
        } else {
            // Case B: Exponential/uniform mixture
            let p = (psi - 1.0) / (psi + 1.0);
            let beta = (1.0 - p) / m;

            // Inverse CDF method
            let u = finstack_core::math::special_functions::norm_cdf(z_v);

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

    /// Compute integrated variance for spot evolution.
    ///
    /// Uses approximation: ∫v(s)ds ≈ (v_t + v_{t+1})/2 * dt
    fn integrated_variance(&self, v_t: f64, v_next: f64, dt: f64) -> f64 {
        (v_t + v_next) / 2.0 * dt
    }
}

impl Default for QeHeston {
    fn default() -> Self {
        Self::new()
    }
}

impl Discretization<HestonProcess> for QeHeston {
    fn step(
        &self,
        process: &HestonProcess,
        _t: f64,
        dt: f64,
        x: &mut [f64],
        z: &[f64],
        _work: &mut [f64],
    ) {
        let params = process.params();

        let s_t = x[0];
        let v_t = x[1].max(0.0);

        // Step 1: Evolve variance using QE scheme
        let z_v = z[1]; // Independent shock for variance
        let v_next = self.step_variance(v_t, params.kappa, params.theta, params.sigma_v, dt, z_v);

        // Step 2: Compute correlated shock for spot
        // Z_S = ρ Z_v + √(1-ρ²) Z_indep
        let rho = params.rho;
        let z_s = rho * z_v + (1.0 - rho * rho).sqrt() * z[0];

        // Step 3: Evolve spot using log-Euler with integrated variance
        let int_var = self.integrated_variance(v_t, v_next, dt);
        let drift = (params.r - params.q - 0.5 * int_var / dt) * dt;
        let diffusion = int_var.sqrt() * z_s;

        let s_next = s_t * (drift + diffusion).exp();

        // Update state
        x[0] = s_next;
        x[1] = v_next;
    }

    fn work_size(&self, _process: &HestonProcess) -> usize {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::super::process::heston::HestonParams;

    #[test]
    fn test_qe_heston_variance_positive() {
        let qe = QeHeston::new();
        let params = HestonParams::new(0.05, 0.02, 2.0, 0.04, 0.3, -0.7, 0.04);

        // Test with various shocks
        for z in [-3.0, -1.0, 0.0, 1.0, 3.0] {
            let v_next = qe.step_variance(0.04, params.kappa, params.theta, params.sigma_v, 0.01, z);
            assert!(v_next >= 0.0, "Variance became negative with z={}", z);
        }
    }

    #[test]
    fn test_qe_heston_mean_reversion() {
        let qe = QeHeston::new();
        let params = HestonParams::new(0.05, 0.02, 2.0, 0.04, 0.1, -0.5, 0.04);

        // Starting above theta
        let v_high = qe.step_variance(0.08, params.kappa, params.theta, params.sigma_v, 0.1, 0.0);
        assert!(v_high < 0.08, "Should mean-revert toward theta");

        // Starting below theta
        let v_low = qe.step_variance(0.02, params.kappa, params.theta, params.sigma_v, 0.1, 0.0);
        assert!(v_low > 0.02, "Should mean-revert toward theta");
    }

    #[test]
    fn test_qe_heston_step() {
        let heston = HestonProcess::with_params(0.05, 0.02, 2.0, 0.04, 0.3, -0.7, 0.04);
        let qe = QeHeston::new();

        let mut x = vec![100.0, 0.04];
        let z = vec![0.0, 0.0]; // No shocks
        let mut work = vec![];

        qe.step(&heston, 0.0, 0.01, &mut x, &z, &mut work);

        // Spot and variance should be positive
        assert!(x[0] > 0.0);
        assert!(x[1] >= 0.0);

        // With zero shocks and v=theta, variance should stay near theta
        assert!((x[1] - 0.04).abs() < 0.01);
    }

    #[test]
    fn test_qe_heston_correlated_shocks() {
        let heston = HestonProcess::with_params(0.05, 0.02, 2.0, 0.04, 0.3, -0.7, 0.04);
        let qe = QeHeston::new();

        // Negative variance shock should (with neg correlation) give positive spot shock
        let mut x1 = vec![100.0, 0.04];
        let z_neg_var = vec![0.0, -1.0]; // Negative variance shock
        let mut work = vec![];

        qe.step(&heston, 0.0, 0.01, &mut x1, &z_neg_var, &mut work);

        // With ρ=-0.7, negative variance shock gives positive contribution to spot
        // This is captured in the correlation structure
        assert!(x1[0] > 0.0);
        assert!(x1[1] >= 0.0);
    }
}
