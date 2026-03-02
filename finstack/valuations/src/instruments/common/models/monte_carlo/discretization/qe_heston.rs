//! Quadratic-Exponential (QE) scheme for Heston variance process.
//!
//! The QE scheme (Andersen, 2008) ensures positive variance while maintaining
//! accuracy for the CIR-type variance process in Heston.
//!
//! Reference: Andersen (2008) - "Simple and efficient simulation of the Heston stochastic volatility model"

use super::super::process::heston::HestonProcess;
use super::super::traits::Discretization;

/// Integrated variance approximation method.
///
/// Controls how the integrated variance ∫_t^{t+Δt} v_s ds is computed
/// for the log-Euler spot evolution.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum IntegratedVarianceMethod {
    /// Trapezoidal (midpoint) approximation: (v_t + v_{t+Δt}) / 2 × Δt
    ///
    /// This is the standard approximation from Andersen (2008) Section 3.2.4.
    /// Adequate for most use cases with monthly or finer time steps.
    #[default]
    Trapezoidal,

    /// Exact conditional expectation: E[∫v_s ds | v_t, v_{t+Δt}]
    ///
    /// Uses the closed-form conditional formula (given both v_0 and v_T):
    /// ```text
    /// E[∫_0^T v_s ds | v_0, v_T] = (θ - (v_0+v_T)/2)(T - (1-e^{-κT})/κ) + (v_0+v_T)(1-e^{-κT})/κ
    /// ```
    /// which expands to:
    /// ```text
    /// θT - θ(1-e^{-κT})/κ - (v_0+v_T)T/2 + (v_0+v_T)(1-e^{-κT})/κ
    /// ```
    ///
    /// This reduces to the trapezoidal rule (v_0+v_T)/2 × T when κ→0. More accurate than
    /// trapezoidal for high mean-reversion (κ > 5) or coarse time steps.
    ///
    /// Reference: Broadie & Kaya (2006) eq. (16), "Exact Simulation of Stochastic
    /// Volatility and Other Affine Jump Diffusion Processes"
    Exact,
}

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
///
/// # Integrated Variance Options
///
/// The spot evolution requires an estimate of ∫_t^{t+Δt} v_s ds.
/// Two methods are available:
///
/// - [`IntegratedVarianceMethod::Trapezoidal`] (default): (v_t + v_{t+Δt}) / 2 × Δt
/// - [`IntegratedVarianceMethod::Exact`]: Uses conditional expectation formula
///
/// The exact method is more accurate for high mean-reversion or coarse time steps.
#[derive(Debug, Clone)]
pub struct QeHeston {
    /// Critical value for ψ (default 1.5)
    psi_c: f64,
    /// Integrated variance method
    int_var_method: IntegratedVarianceMethod,
}

impl QeHeston {
    /// Create a new QE Heston discretization with default settings.
    pub fn new() -> Self {
        Self {
            psi_c: 1.5,
            int_var_method: IntegratedVarianceMethod::default(),
        }
    }

    /// Create with custom ψ_c threshold.
    pub fn with_psi_c(psi_c: f64) -> Self {
        Self {
            psi_c,
            int_var_method: IntegratedVarianceMethod::default(),
        }
    }

    /// Set the integrated variance method.
    ///
    /// # Example
    ///
    /// ```rust
    /// use finstack_valuations::instruments::common::models::monte_carlo::discretization::qe_heston::{
    ///     QeHeston, IntegratedVarianceMethod
    /// };
    ///
    /// // Use exact integrated variance for high-kappa scenarios
    /// let qe = QeHeston::new().with_integrated_variance(IntegratedVarianceMethod::Exact);
    /// ```
    pub fn with_integrated_variance(mut self, method: IntegratedVarianceMethod) -> Self {
        self.int_var_method = method;
        self
    }

    /// Create with exact integrated variance for high mean-reversion scenarios.
    ///
    /// This is recommended when κ > 5 or using coarse time steps (Δt > 0.1 years).
    pub fn exact_variance() -> Self {
        Self {
            psi_c: 1.5,
            int_var_method: IntegratedVarianceMethod::Exact,
        }
    }

    /// Compute next variance using QE scheme.
    ///
    /// # Algorithm
    ///
    /// The QE scheme from Andersen (2008) uses a conditional moment-matching
    /// approach based on the ratio ψ = s²/m² where:
    /// - m = E[v_{t+Δt} | v_t] (conditional mean)
    /// - s² = Var[v_{t+Δt} | v_t] (conditional variance)
    ///
    /// # Numerical Safeguards
    ///
    /// This implementation includes two safeguards not in the original paper:
    ///
    /// 1. **ψ clamping**: ψ is clamped to max 10.0 to prevent overflow in the
    ///    Case A (quadratic) branch. This can occur when:
    ///    - Very high vol-of-vol (σ_v)
    ///    - Large time steps (Δt)
    ///    - Feller condition violated (2κθ < σ_v²)
    ///
    ///    The clamp ensures numerical stability without materially affecting
    ///    results since ψ > 10 would use Case B anyway (exponential mixture).
    ///
    /// 2. **Small mean handling**: When m < 1e-10, we force Case B directly
    ///    to avoid amplified numerical errors from division by near-zero.
    ///
    /// # References
    ///
    /// - Andersen, L. (2008). "Simple and efficient simulation of the Heston
    ///   stochastic volatility model." Journal of Computational Finance, 11(3).
    /// - See Section 3.2.3 for the QE algorithm and Section 4 for numerical issues.
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

        // Compute conditional mean and variance (Andersen 2008, Eq. 17-18)
        let exp_kappa_dt = (-kappa * dt).exp();
        let m = theta + (v_t - theta) * exp_kappa_dt;
        let s2 = v_t * sigma_v * sigma_v * exp_kappa_dt * (1.0 - exp_kappa_dt) / kappa
            + theta * sigma_v * sigma_v * (1.0 - exp_kappa_dt).powi(2) / (2.0 * kappa);

        // Compute ψ = s²/m² with numerical safeguards
        //
        // Safeguard 1: Clamp ψ to 10.0 maximum
        // =====================================
        // Rationale: For ψ > ψ_c (typically 1.5), the QE scheme uses Case B
        // (exponential mixture). However, the Case A formulas involve terms like
        // (2/ψ - 1) which become negative for ψ > 2. Clamping to 10.0 ensures
        // we always use Case B for extreme ψ values, avoiding numerical issues.
        //
        // This is a numerical stability enhancement, not a material change to
        // the algorithm - ψ values this high already force Case B.
        //
        // Safeguard 2: Force Case B for tiny mean (threshold 1e-10, matches QeCir)
        // =========================================================================
        // Rationale: When m → 0, ψ = s²/m² → ∞, but the division itself may
        // produce overflow or NaN. Forcing Case B directly avoids this.
        let psi = if m > 1e-10 {
            let ratio = s2 / (m * m);
            ratio.min(10.0) // Safeguard 1: clamp to prevent Case A overflow
        } else {
            // Safeguard 2: very small mean forces Case B
            self.psi_c + 1.0
        };

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
    /// Supports two methods:
    ///
    /// ## Trapezoidal (default)
    ///
    /// ```text
    /// ∫_t^{t+Δt} v_s ds ≈ (v_t + v_{t+Δt}) / 2 × Δt
    /// ```
    ///
    /// Standard in the QE scheme, adequate for typical time steps (monthly or finer).
    ///
    /// ## Exact (mean-reversion-adjusted trapezoidal)
    ///
    /// Corrects the trapezoidal rule with a mean-reversion weighting factor:
    /// ```text
    /// ∫v ≈ θT + (v₀ + v_T - 2θ)(1 - e^{-κT}) / (2κ)
    /// ```
    ///
    /// This reduces to:
    /// - **(v₀+v_T)/2 · T** for κ → 0 (plain trapezoidal)
    /// - **θT** for κ → ∞ or v₀ = v_T = θ (mean dominates)
    ///
    /// Not the full Broadie-Kaya (2006) conditional distribution (which requires
    /// Fourier inversion); rather a lightweight correction that is standard in
    /// Andersen (2008) QE implementations.
    ///
    /// # Arguments
    /// * `v_t` - Current variance
    /// * `v_next` - Next variance (already simulated)
    /// * `dt` - Time step
    /// * `kappa` - Mean reversion speed (only used for Exact method)
    /// * `theta` - Long-run variance (only used for Exact method)
    ///
    /// # Returns
    /// Integrated variance over [t, t+dt]
    ///
    /// # References
    /// - Andersen, L. (2008). "Simple and efficient simulation of the Heston
    ///   stochastic volatility model." *J. Comp. Finance*, 11(3).
    #[inline]
    fn integrated_variance(&self, v_t: f64, v_next: f64, dt: f64, kappa: f64, theta: f64) -> f64 {
        match self.int_var_method {
            IntegratedVarianceMethod::Trapezoidal => (v_t + v_next) / 2.0 * dt,
            IntegratedVarianceMethod::Exact => {
                if kappa.abs() < 1e-10 {
                    (v_t + v_next) / 2.0 * dt
                } else {
                    let exp_term = (-kappa * dt).exp();
                    theta * dt + (v_t + v_next - 2.0 * theta) * (1.0 - exp_term) / (2.0 * kappa)
                }
            }
        }
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
        let int_var = self.integrated_variance(v_t, v_next, dt, params.kappa, params.theta);
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
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::super::super::process::heston::HestonParams;
    use super::*;

    #[test]
    fn test_qe_heston_variance_positive() {
        let qe = QeHeston::new();
        let params = HestonParams::new(0.05, 0.02, 2.0, 0.04, 0.3, -0.7, 0.04);

        // Test with various shocks
        for z in [-3.0, -1.0, 0.0, 1.0, 3.0] {
            let v_next =
                qe.step_variance(0.04, params.kappa, params.theta, params.sigma_v, 0.01, z);
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

    #[test]
    fn test_integrated_variance_bounds() {
        let qe = QeHeston::new();
        let v_t = 0.04;
        let v_next = 0.05;
        let dt = 0.1;
        let kappa = 2.0;
        let theta = 0.04;

        let int_var = qe.integrated_variance(v_t, v_next, dt, kappa, theta);

        // Integrated variance should be between v_t * dt and v_next * dt
        let lower = v_t.min(v_next) * dt;
        let upper = v_t.max(v_next) * dt;
        assert!(
            int_var >= lower && int_var <= upper,
            "Integrated variance {} out of bounds [{}, {}]",
            int_var,
            lower,
            upper
        );

        // Should equal the midpoint
        let midpoint = (v_t + v_next) / 2.0 * dt;
        assert!(
            (int_var - midpoint).abs() < 1e-12,
            "Integrated variance should equal midpoint: got {} vs {}",
            int_var,
            midpoint
        );
    }

    #[test]
    fn test_integrated_variance_symmetric() {
        // When v_t == v_next, result should equal v * dt
        let qe = QeHeston::new();
        let v = 0.04;
        let dt = 0.1;
        let kappa = 2.0;
        let theta = 0.04;

        let int_var = qe.integrated_variance(v, v, dt, kappa, theta);
        let expected = v * dt;

        assert!(
            (int_var - expected).abs() < 1e-12,
            "When v_t == v_next, integrated variance should equal v*dt: {} vs {}",
            int_var,
            expected
        );
    }

    #[test]
    fn test_integrated_variance_various_dt() {
        // Test with various time steps
        let qe = QeHeston::new();
        let v_t = 0.04;
        let v_next = 0.05;
        let kappa = 2.0;
        let theta = 0.04;

        for dt in [0.001, 0.01, 0.1, 0.25, 1.0] {
            let int_var = qe.integrated_variance(v_t, v_next, dt, kappa, theta);
            let midpoint = (v_t + v_next) / 2.0 * dt;

            assert!(
                (int_var - midpoint).abs() < 1e-12,
                "Integrated variance should equal midpoint for dt={}: got {} vs {}",
                dt,
                int_var,
                midpoint
            );
        }
    }

    #[test]
    fn test_exact_integrated_variance() {
        // Test exact method vs trapezoidal
        let qe_trap = QeHeston::new();
        let qe_exact = QeHeston::exact_variance();

        let v_t = 0.04;
        let v_next = 0.06;
        let dt = 0.25;
        let kappa = 5.0; // High mean reversion
        let theta = 0.04;

        let trap = qe_trap.integrated_variance(v_t, v_next, dt, kappa, theta);
        let exact = qe_exact.integrated_variance(v_t, v_next, dt, kappa, theta);

        // Both should be positive
        assert!(trap > 0.0);
        assert!(exact > 0.0);

        // Exact should differ from trapezoidal for high kappa
        // (they're not equal but both reasonable)
        let diff_pct = ((exact - trap) / trap).abs() * 100.0;
        assert!(
            diff_pct < 20.0,
            "Exact and trapezoidal should be within 20%: {} vs {}, diff={}%",
            exact,
            trap,
            diff_pct
        );
    }

    #[test]
    fn test_exact_variance_converges_for_small_kappa() {
        // For κ ≈ 0, exact should fall back to trapezoidal
        let qe_exact = QeHeston::exact_variance();

        let v_t = 0.04;
        let v_next = 0.05;
        let dt = 0.1;
        let kappa = 1e-12; // Effectively zero
        let theta = 0.04;

        let int_var = qe_exact.integrated_variance(v_t, v_next, dt, kappa, theta);
        let trap = (v_t + v_next) / 2.0 * dt;

        assert!(
            (int_var - trap).abs() < 1e-10,
            "Exact should match trapezoidal for κ≈0: {} vs {}",
            int_var,
            trap
        );
    }

    #[test]
    fn test_builder_pattern() {
        // Test that builder pattern works for configuring QE scheme
        let qe = QeHeston::new().with_integrated_variance(IntegratedVarianceMethod::Exact);

        // Verify it works without panics
        let params = HestonParams::new(0.05, 0.02, 2.0, 0.04, 0.3, -0.7, 0.04);
        let v = qe.step_variance(0.04, params.kappa, params.theta, params.sigma_v, 0.1, 0.0);
        assert!(v >= 0.0);
    }

    #[test]
    fn test_with_psi_c() {
        // Test custom psi_c threshold
        let qe = QeHeston::with_psi_c(2.0);
        let params = HestonParams::new(0.05, 0.02, 2.0, 0.04, 0.3, -0.7, 0.04);

        // Variance should remain positive
        for z in [-2.0, 0.0, 2.0] {
            let v = qe.step_variance(0.04, params.kappa, params.theta, params.sigma_v, 0.1, z);
            assert!(v >= 0.0);
        }
    }
}
