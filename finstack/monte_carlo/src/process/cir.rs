//! CIR (Cox-Ingersoll-Ross) and CIR++ processes.
//!
//! Implements the square-root diffusion process used for modeling:
//! - Short rates (Vasicek extension with mean-reverting variance)
//! - Stochastic volatility (variance component in Heston)
//! - Credit intensities (default hazard rates)
//!
//! # CIR SDE
//!
//! ```text
//! dv_t = κ(θ - v_t)dt + σ√v_t dW_t
//! ```
//!
//! where:
//! - κ = mean reversion speed
//! - θ = long-term mean
//! - σ = volatility of volatility
//! - v_t ≥ 0 (ensured by QE discretization)
//!
//! # Feller Condition
//!
//! For positivity: 2κθ ≥ σ²
//! If violated, zero is attainable (but QE handles gracefully).

use super::super::traits::{state_keys, PathState, StochasticProcess};

/// CIR process parameters.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CirParams {
    /// Mean reversion speed (κ)
    pub kappa: f64,
    /// Long-term mean (θ)
    pub theta: f64,
    /// Volatility of volatility (σ)
    pub sigma: f64,
}

impl CirParams {
    /// Create new CIR parameters with validation.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - `kappa <= 0` or non-finite
    /// - `theta < 0` or non-finite
    /// - `sigma <= 0` or non-finite
    pub fn new(kappa: f64, theta: f64, sigma: f64) -> finstack_core::Result<Self> {
        if !kappa.is_finite() || kappa <= 0.0 {
            return Err(finstack_core::Error::Validation(format!(
                "CIR kappa (mean reversion speed) must be positive, got {kappa}"
            )));
        }
        if !theta.is_finite() || theta < 0.0 {
            return Err(finstack_core::Error::Validation(format!(
                "CIR theta (long-term mean) must be non-negative, got {theta}"
            )));
        }
        if !sigma.is_finite() || sigma <= 0.0 {
            return Err(finstack_core::Error::Validation(format!(
                "CIR sigma (volatility) must be positive, got {sigma}"
            )));
        }

        Ok(Self {
            kappa,
            theta,
            sigma,
        })
    }

    /// Check if Feller condition is satisfied.
    ///
    /// Feller condition: 2κθ ≥ σ²
    /// If satisfied, the process stays strictly positive.
    pub fn satisfies_feller(&self) -> bool {
        2.0 * self.kappa * self.theta >= self.sigma * self.sigma
    }

    /// Get the critical ψ threshold for QE scheme.
    ///
    /// This is a typical value used in the QE discretization to decide
    /// between the power/gamma and exponential representations.
    pub fn default_psi_c() -> f64 {
        1.5
    }
}

/// CIR process for modeling short rates or intensities.
///
/// State dimension: 1 (variance/rate v)
/// Factor dimension: 1 (Brownian motion)
///
/// # SDE
///
/// ```text
/// dv_t = κ(θ - v_t)dt + σ√v_t dW_t
/// ```
///
/// # Discretization
///
/// Use `QeCir` discretization (extracted from Heston QE) for best accuracy
/// and guaranteed positivity.
#[derive(Debug, Clone)]
pub struct CirProcess {
    params: CirParams,
}

impl CirProcess {
    /// Create a new CIR process.
    pub fn new(params: CirParams) -> Self {
        Self { params }
    }

    /// Create with explicit parameters.
    ///
    /// # Errors
    ///
    /// Returns an error if any parameter is invalid (see [`CirParams::new`]).
    pub fn with_params(kappa: f64, theta: f64, sigma: f64) -> finstack_core::Result<Self> {
        Ok(Self::new(CirParams::new(kappa, theta, sigma)?))
    }

    /// Get parameters.
    pub fn params(&self) -> &CirParams {
        &self.params
    }
}

impl StochasticProcess for CirProcess {
    fn dim(&self) -> usize {
        1
    }

    fn num_factors(&self) -> usize {
        1
    }

    fn drift(&self, _t: f64, x: &[f64], out: &mut [f64]) {
        // μ(v) = κ(θ - v)
        out[0] = self.params.kappa * (self.params.theta - x[0]);
    }

    fn diffusion(&self, _t: f64, x: &[f64], out: &mut [f64]) {
        // σ(v) = σ√v (ensure non-negative under sqrt)
        let v = x[0].max(0.0);
        out[0] = self.params.sigma * v.sqrt();
    }

    fn populate_path_state(&self, x: &[f64], state: &mut PathState) {
        if !x.is_empty() {
            state.set(state_keys::SHORT_RATE, x[0]);
            state.set(state_keys::SPOT, x[0]);
        }
    }
}

/// CIR++ process (shifted CIR for yield curve fitting).
///
/// The CIR++ model adds a deterministic shift φ(t) to CIR:
///
/// ```text
/// r_t = x_t + φ(t)
/// dx_t = κ(θ - x_t)dt + σ√x_t dW_t
/// ```
///
/// where φ(t) is chosen to fit the initial yield curve.
///
/// State dimension: 1 (shifted rate x, actual rate is x + φ(t))
#[derive(Debug, Clone)]
pub struct CirPlusPlusProcess {
    /// Base CIR process
    cir: CirProcess,
    /// Deterministic shift curve φ(t) (piecewise constant)
    shift_curve: Vec<f64>,
    /// Time breakpoints for φ(t)
    shift_times: Vec<f64>,
}

impl CirPlusPlusProcess {
    /// Create a new CIR++ process.
    ///
    /// # Arguments
    ///
    /// * `cir` - Base CIR process
    /// * `shift_curve` - Deterministic shift values
    /// * `shift_times` - Time breakpoints (must be sorted)
    pub fn new(cir: CirProcess, shift_curve: Vec<f64>, shift_times: Vec<f64>) -> Self {
        assert_eq!(
            shift_curve.len(),
            shift_times.len(),
            "Shift curve and times must have same length"
        );
        assert!(
            !shift_times.is_empty(),
            "Must have at least one shift value"
        );

        Self {
            cir,
            shift_curve,
            shift_times,
        }
    }

    /// Create with constant shift.
    pub fn with_constant_shift(cir: CirProcess, shift: f64) -> Self {
        Self::new(cir, vec![shift], vec![0.0])
    }

    /// Get φ(t) at a given time.
    pub fn shift_at_time(&self, t: f64) -> f64 {
        // Piecewise-constant interpolation
        for i in (0..self.shift_times.len()).rev() {
            if t >= self.shift_times[i] {
                return self.shift_curve[i];
            }
        }

        self.shift_curve[0]
    }

    /// Get actual short rate r_t = x_t + φ(t).
    pub fn actual_rate(&self, x: f64, t: f64) -> f64 {
        x + self.shift_at_time(t)
    }

    /// Get base CIR process.
    pub fn cir(&self) -> &CirProcess {
        &self.cir
    }
}

impl StochasticProcess for CirPlusPlusProcess {
    fn dim(&self) -> usize {
        1
    }

    fn num_factors(&self) -> usize {
        1
    }

    fn drift(&self, t: f64, x: &[f64], out: &mut [f64]) {
        // The state x follows base CIR dynamics (shift doesn't affect SDE)
        self.cir.drift(t, x, out);
    }

    fn diffusion(&self, t: f64, x: &[f64], out: &mut [f64]) {
        // Diffusion is same as base CIR
        self.cir.diffusion(t, x, out);
    }

    fn populate_path_state(&self, x: &[f64], state: &mut PathState) {
        if !x.is_empty() {
            state.set(state_keys::SHORT_RATE, x[0]);
            state.set(state_keys::SPOT, x[0]);
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_cir_params_feller_condition() {
        // Satisfies Feller: 2κθ ≥ σ²
        let params1 = CirParams::new(0.5, 0.04, 0.1).unwrap();
        assert!(params1.satisfies_feller());
        // 2 * 0.5 * 0.04 = 0.04 >= 0.01 ✓

        // Violates Feller
        let params2 = CirParams::new(0.1, 0.01, 0.2).unwrap();
        assert!(!params2.satisfies_feller());
        // 2 * 0.1 * 0.01 = 0.002 < 0.04 ✗
    }

    #[test]
    fn test_cir_drift() {
        let params = CirParams::new(0.3, 0.04, 0.1).unwrap();
        let process = CirProcess::new(params);

        // Above mean
        let x = vec![0.05];
        let mut drift = vec![0.0];
        process.drift(0.0, &x, &mut drift);
        assert!(drift[0] < 0.0); // Pull down
        assert_eq!(drift[0], 0.3 * (0.04 - 0.05));

        // Below mean
        let x2 = vec![0.03];
        process.drift(0.0, &x2, &mut drift);
        assert!(drift[0] > 0.0); // Pull up
        assert_eq!(drift[0], 0.3 * (0.04 - 0.03));
    }

    #[test]
    fn test_cir_diffusion() {
        let params = CirParams::new(0.3, 0.04, 0.1).unwrap();
        let process = CirProcess::new(params);

        let x = vec![0.04];
        let mut diffusion = vec![0.0];
        process.diffusion(0.0, &x, &mut diffusion);

        // σ√v = 0.1 * √0.04 = 0.1 * 0.2 = 0.02
        assert_eq!(diffusion[0], 0.1 * 0.04_f64.sqrt());
    }

    #[test]
    fn test_cir_plus_plus_shift() {
        let cir = CirProcess::with_params(0.1, 0.03, 0.05).unwrap();
        let shift_curve = vec![0.01, 0.02];
        let shift_times = vec![0.0, 1.0];

        let cir_pp = CirPlusPlusProcess::new(cir, shift_curve, shift_times);

        assert_eq!(cir_pp.shift_at_time(0.0), 0.01);
        assert_eq!(cir_pp.shift_at_time(0.5), 0.01);
        assert_eq!(cir_pp.shift_at_time(1.0), 0.02);
        assert_eq!(cir_pp.shift_at_time(2.0), 0.02);

        // Actual rate
        assert_eq!(cir_pp.actual_rate(0.03, 0.0), 0.04); // x + φ(0)
        assert_eq!(cir_pp.actual_rate(0.03, 1.5), 0.05); // x + φ(1.5)
    }

    #[test]
    fn test_cir_plus_plus_dynamics() {
        let cir = CirProcess::with_params(0.1, 0.03, 0.05).unwrap();
        let cir_pp = CirPlusPlusProcess::with_constant_shift(cir, 0.02);

        // The state x follows base CIR dynamics
        let x = vec![0.04];
        let mut drift = vec![0.0];
        cir_pp.drift(0.0, &x, &mut drift);

        // Drift should be same as base CIR
        assert_eq!(drift[0], 0.1 * (0.03 - 0.04));
    }
}
