//! Discretization scheme for revolving credit multi-factor process.
//!
//! Handles three correlated factors:
//! - Utilization: Euler-Maruyama (OU process)
//! - Short rate: ExactHullWhite1F (for floating) or constant (for fixed)
//! - Credit spread: QeCir (CIR process)
//!
//! When correlation is present, applies Cholesky decomposition to generate
//! correlated Brownian motions.

use super::monte_carlo_process::{InterestRateSpec, RevolvingCreditProcess};
use finstack_core::math::linalg::CholeskyError;
use finstack_monte_carlo::discretization::exact_hw1f::ExactHullWhite1F;
use finstack_monte_carlo::process::correlation::cholesky_correlation;
use finstack_monte_carlo::traits::Discretization;

/// Discretization scheme for revolving credit process.
///
/// Uses specialized schemes for each component:
/// - Utilization: Euler-Maruyama (OU can use exact, but simpler to use Euler)
/// - Short rate: ExactHullWhite1F for floating rates
/// - Credit spread: QeCir for CIR process
///
/// Correlation is handled via pivoted Cholesky decomposition when present.
#[derive(Debug, Clone)]
pub struct RevolvingCreditDiscretization {
    /// Pivoted Cholesky factor of correlation matrix (if correlation is used), in
    /// original variable order [utilization, rate, credit].
    cholesky_factor: Option<finstack_core::math::linalg::CorrelationFactor>,
    /// Hull-White exact discretization (for floating rates)
    hw_disc: Option<ExactHullWhite1F>,
}

impl RevolvingCreditDiscretization {
    /// Create a new discretization scheme.
    ///
    /// # Arguments
    ///
    /// * `correlation` - Optional 3x3 correlation matrix [utilization, rate, credit]
    pub fn new(correlation: Option<&[[f64; 3]; 3]>) -> finstack_core::Result<Self> {
        let cholesky_factor = if let Some(corr) = correlation {
            // Convert 3x3 array to row-major vector
            let corr_vec: Vec<f64> = corr.iter().flat_map(|row| row.iter().copied()).collect();
            Some(cholesky_correlation(&corr_vec, 3).map_err(|e| match e {
                CholeskyError::NotPositiveDefinite { .. } => {
                    finstack_core::Error::Input(finstack_core::InputError::Invalid)
                }
                CholeskyError::DimensionMismatch { .. } => {
                    finstack_core::Error::Input(finstack_core::InputError::DimensionMismatch)
                }
                _ => finstack_core::Error::Input(finstack_core::InputError::Invalid),
            })?)
        } else {
            None
        };

        Ok(Self {
            cholesky_factor,
            hw_disc: Some(ExactHullWhite1F::new()),
        })
    }

    /// Create from process (test convenience method).
    #[cfg(test)]
    pub fn from_process(process: &RevolvingCreditProcess) -> finstack_core::Result<Self> {
        Self::new(process.correlation())
    }
}

impl Discretization<RevolvingCreditProcess> for RevolvingCreditDiscretization {
    fn step(
        &self,
        process: &RevolvingCreditProcess,
        t: f64,
        dt: f64,
        x: &mut [f64],
        z: &[f64],
        work: &mut [f64],
    ) {
        // x[0] = utilization, x[1] = short_rate, x[2] = credit_spread
        // z = independent standard normals

        // Apply correlation if present
        let z_corr = if let Some(ref chol) = self.cholesky_factor {
            // Split work buffer: [z_corr | ...]
            let z_corr_buf = &mut work[0..3];
            // Dimensions are guaranteed by construction: cholesky_factor is 3×3 and
            // z_corr_buf has length 3 matching the 3-factor process.
            let _ = chol.apply(z, z_corr_buf);
            z_corr_buf
        } else {
            // No correlation, use original shocks
            z
        };

        // Step 1: Utilization (OU process) - Euler-Maruyama
        // dU = κ_U (θ_U - U) dt + σ_U dW
        let util_params = &process.params().utilization;
        let util_drift = util_params.kappa * (util_params.theta - x[0].clamp(0.0, 1.0));
        let util_diff = util_params.sigma;
        x[0] += util_drift * dt + util_diff * dt.sqrt() * z_corr[0];
        x[0] = x[0].clamp(0.0, 1.0);

        // Step 2: Short rate
        match &process.params().interest_rate {
            InterestRateSpec::Fixed { .. } => {
                // Fixed rate: no change
                // x[1] stays constant
            }
            InterestRateSpec::Floating { params, .. } => {
                // Floating rate: use exact Hull-White discretization
                // Extract HW1F state and step it
                let mut rate_state = [x[1]];
                let rate_shock = [z_corr[1]];
                let _rate_work = [0.0];

                // Create temporary HW1F process for stepping
                // We need to call the exact discretization
                // INVARIANT: hw_disc is always Some when rate_spec is Floating, enforced by
                // RevolvingCreditDiscretization::new() constructor validation.
                #[allow(clippy::expect_used)]
                let _hw_disc = self
                    .hw_disc
                    .as_ref()
                    .expect("HW discretization must be present for floating rate specification");

                // For HW1F, we need to compute:
                // r_{t+dt} = r_t e^{-κdt} + θ(1 - e^{-κdt}) + σ√[(1-e^{-2κdt})/(2κ)] Z
                let kappa = params.kappa;
                let sigma = params.sigma;
                let theta = params.theta_at_time(t);

                let exp_kappa_dt = (-kappa * dt).exp();
                let mean = rate_state[0] * exp_kappa_dt + theta * (1.0 - exp_kappa_dt);

                let std_dev = if (kappa * dt).abs() < 1e-8 {
                    sigma * dt.sqrt() * (1.0 - kappa * dt / 3.0)
                } else {
                    sigma * ((1.0 - (-2.0 * kappa * dt).exp()) / (2.0 * kappa)).sqrt()
                };

                rate_state[0] = mean + std_dev * rate_shock[0];
                x[1] = rate_state[0];
            }
            InterestRateSpec::DeterministicForward { times, rates } => {
                // Deterministic forward curve: set short rate to fwd(time_offset + t+dt)
                let t_total = process.params().time_offset + t + dt;
                // Linear interpolation with clamp at ends
                let n = times.len();
                let mut r = if n == 0 {
                    0.0
                } else if t_total <= times[0] {
                    rates[0]
                } else if t_total >= times[n - 1] {
                    rates[n - 1]
                } else {
                    // find bracket
                    let mut i = 1usize;
                    while i < n && t_total > times[i] {
                        i += 1;
                    }
                    let i1 = i - 1;
                    let (t1, t2) = (times[i1], times[i]);
                    let (r1, r2) = (rates[i1], rates[i]);
                    let w = (t_total - t1) / (t2 - t1);
                    r1 + w * (r2 - r1)
                };
                // Guard against NaN
                if !r.is_finite() {
                    r = 0.0;
                }
                x[1] = r;
            }
        }

        // Step 3: Credit spread (CIR process) - QE scheme
        // We need to use QeCir discretization
        // Extract CIR state and step it
        let credit_state = [x[2].max(0.0)];
        let credit_shock = [z_corr[2]];
        let _credit_work = [0.0];

        // Get CIR parameters
        let cir_params = &process.params().credit_spread.cir;

        // Apply QE scheme directly
        let v_t = credit_state[0];
        let exp_kappa_dt = (-cir_params.kappa * dt).exp();
        let m = cir_params.theta + (v_t - cir_params.theta) * exp_kappa_dt;
        let s2 = v_t * cir_params.sigma * cir_params.sigma * exp_kappa_dt * (1.0 - exp_kappa_dt)
            / cir_params.kappa
            + cir_params.theta * cir_params.sigma * cir_params.sigma * (1.0 - exp_kappa_dt).powi(2)
                / (2.0 * cir_params.kappa);

        // When m is near zero, force Case B (exponential/uniform mixture) by
        // setting psi above the threshold. Setting psi = 0.0 would send
        // Case A into 2/psi = infinity → NaN (matches canonical QeCir).
        let psi_c = 1.5;
        let psi = if m > 1e-10 { s2 / (m * m) } else { psi_c + 1.0 };

        let v_next = if psi <= psi_c {
            // Case A: Power/gamma approximation
            let b_squared = 2.0 / psi - 1.0 + (2.0 / psi * (2.0 / psi - 1.0)).sqrt();
            let a = m / (1.0 + b_squared);
            a * (credit_shock[0] + b_squared.sqrt()).powi(2).max(0.0)
        } else {
            // Case B: Exponential/uniform mixture
            let p = (psi - 1.0) / (psi + 1.0);
            let beta = (1.0 - p) / m;

            use finstack_core::math::special_functions::norm_cdf;
            let u = norm_cdf(credit_shock[0]);

            if u <= p {
                0.0
            } else {
                ((1.0 - p) / (u - p)).ln() / beta
            }
            .max(0.0)
        };

        x[2] = v_next;
    }

    fn work_size(&self, _process: &RevolvingCreditProcess) -> usize {
        // Need space for correlated shocks (3) + any additional workspace
        if self.cholesky_factor.is_some() {
            3 // For z_corr
        } else {
            0 // No workspace needed if no correlation
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::monte_carlo_process::{
        CreditSpreadParams, InterestRateSpec, RevolvingCreditProcess, RevolvingCreditProcessParams,
        UtilizationParams,
    };
    use super::*;
    use finstack_monte_carlo::process::ou::HullWhite1FParams;

    #[test]
    fn test_discretization_creation() {
        let disc = RevolvingCreditDiscretization::new(None).expect("should succeed");
        assert!(disc.cholesky_factor.is_none());
    }

    #[test]
    fn test_discretization_with_correlation() {
        let correlation = [[1.0, 0.2, 0.1], [0.2, 1.0, 0.3], [0.1, 0.3, 1.0]];
        let disc = RevolvingCreditDiscretization::new(Some(&correlation)).expect("should succeed");
        assert!(disc.cholesky_factor.is_some());
    }

    #[test]
    fn test_discretization_step_fixed_rate() {
        let utilization = UtilizationParams::new(0.5, 0.6, 0.1);
        let interest_rate = InterestRateSpec::Fixed { rate: 0.05 };
        let credit_spread = CreditSpreadParams::new(0.3, 0.02, 0.05, 0.015).unwrap();

        let params = RevolvingCreditProcessParams::new(utilization, interest_rate, credit_spread);
        let process = RevolvingCreditProcess::new(params);
        let disc = RevolvingCreditDiscretization::from_process(&process).expect("should succeed");

        let mut x = [0.5, 0.05, 0.015];
        let z = [0.0, 0.0, 0.0]; // No shocks
        let mut work = vec![0.0; disc.work_size(&process)];

        disc.step(&process, 0.0, 0.01, &mut x, &z, &mut work);

        // Utilization should drift toward mean
        assert!(x[0] > 0.5 && x[0] < 0.6);

        // Fixed rate should stay constant
        assert_eq!(x[1], 0.05);

        // Credit spread should drift toward mean
        assert!(x[2] > 0.015 && x[2] < 0.02);
    }

    #[test]
    fn test_discretization_step_floating_rate() {
        let utilization = UtilizationParams::new(0.5, 0.6, 0.1);
        let hw_params = HullWhite1FParams::new(0.1, 0.01, 0.03);
        let interest_rate = InterestRateSpec::Floating {
            params: hw_params,
            initial: 0.04,
        };
        let credit_spread = CreditSpreadParams::new(0.3, 0.02, 0.05, 0.015).unwrap();

        let params = RevolvingCreditProcessParams::new(utilization, interest_rate, credit_spread);
        let process = RevolvingCreditProcess::new(params);
        let disc = RevolvingCreditDiscretization::from_process(&process).expect("should succeed");

        let mut x = [0.5, 0.04, 0.015];
        let z = [0.0, 0.0, 0.0]; // No shocks
        let mut work = vec![0.0; disc.work_size(&process)];

        disc.step(&process, 0.0, 0.01, &mut x, &z, &mut work);

        // Rate should drift toward mean (0.03)
        assert!(x[1] < 0.04);
    }
}
