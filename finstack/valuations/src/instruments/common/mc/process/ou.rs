//! Ornstein-Uhlenbeck and Hull-White 1-factor processes.
//!
//! Implements the standard single-factor short rate model:
//!
//! ```text
//! dr_t = κ[θ(t) - r_t]dt + σ dW_t
//! ```
//!
//! where:
//! - κ = mean reversion speed
//! - θ(t) = time-dependent mean reversion level  
//! - σ = instantaneous volatility
//! - W_t = Brownian motion
//!
//! The Hull-White 1F model uses a time-dependent θ(t) to fit the initial
//! yield curve, while the Ornstein-Uhlenbeck (Vasicek) model uses constant θ.

use super::super::traits::StochasticProcess;

/// Hull-White 1-factor parameters.
#[derive(Clone, Debug)]
pub struct HullWhite1FParams {
    /// Mean reversion speed (κ)
    pub kappa: f64,
    /// Instantaneous volatility (σ)
    pub sigma: f64,
    /// Time-dependent mean reversion level θ(t)
    /// Stored as piecewise-constant segments
    pub theta_curve: Vec<f64>,
    /// Time breakpoints for θ(t) (must be sorted)
    pub theta_times: Vec<f64>,
}

impl HullWhite1FParams {
    /// Create new Hull-White 1F parameters with constant θ.
    pub fn new(kappa: f64, sigma: f64, theta: f64) -> Self {
        Self {
            kappa,
            sigma,
            theta_curve: vec![theta],
            theta_times: vec![0.0],
        }
    }

    /// Create with time-dependent θ(t).
    ///
    /// # Arguments
    ///
    /// * `kappa` - Mean reversion speed
    /// * `sigma` - Volatility
    /// * `theta_curve` - θ values (piecewise constant)
    /// * `theta_times` - Time breakpoints (must be sorted)
    pub fn with_time_dependent_theta(
        kappa: f64,
        sigma: f64,
        theta_curve: Vec<f64>,
        theta_times: Vec<f64>,
    ) -> Self {
        assert_eq!(
            theta_curve.len(),
            theta_times.len(),
            "Theta curve and times must have same length"
        );
        assert!(
            !theta_times.is_empty(),
            "Must have at least one theta value"
        );

        Self {
            kappa,
            sigma,
            theta_curve,
            theta_times,
        }
    }

    /// Get θ(t) at a given time.
    pub fn theta_at_time(&self, t: f64) -> f64 {
        // Find the appropriate theta value for time t
        // Use piecewise-constant interpolation
        for i in (0..self.theta_times.len()).rev() {
            if t >= self.theta_times[i] {
                return self.theta_curve[i];
            }
        }

        // If t < first breakpoint, use first value
        self.theta_curve[0]
    }
}

/// Hull-White 1-factor short rate process.
///
/// State dimension: 1 (short rate r)
/// Factor dimension: 1 (Brownian motion)
///
/// # SDE
///
/// ```text
/// dr_t = κ[θ(t) - r_t]dt + σ dW_t
/// ```
///
/// # Exact Solution
///
/// For piecewise-constant θ(t), there exists an exact discretization:
///
/// ```text
/// r_{t+Δt} = r_t e^{-κΔt} + θ(1 - e^{-κΔt}) + σ√[(1-e^{-2κΔt})/(2κ)] Z
/// ```
///
/// Use `ExactHullWhite1F` discretization for best accuracy.
#[derive(Clone, Debug)]
pub struct HullWhite1FProcess {
    params: HullWhite1FParams,
}

impl HullWhite1FProcess {
    /// Create a new Hull-White 1F process.
    pub fn new(params: HullWhite1FParams) -> Self {
        Self { params }
    }

    /// Create with constant θ (Vasicek model).
    pub fn vasicek(kappa: f64, theta: f64, sigma: f64) -> Self {
        Self::new(HullWhite1FParams::new(kappa, sigma, theta))
    }

    /// Get parameters.
    pub fn params(&self) -> &HullWhite1FParams {
        &self.params
    }

    /// Get θ(t) at a given time.
    pub fn theta_at_time(&self, t: f64) -> f64 {
        self.params.theta_at_time(t)
    }
}

impl StochasticProcess for HullWhite1FProcess {
    fn dim(&self) -> usize {
        1
    }

    fn num_factors(&self) -> usize {
        1
    }

    fn drift(&self, t: f64, x: &[f64], out: &mut [f64]) {
        // μ(r) = κ[θ(t) - r]
        let theta = self.theta_at_time(t);
        out[0] = self.params.kappa * (theta - x[0]);
    }

    fn diffusion(&self, _t: f64, _x: &[f64], out: &mut [f64]) {
        // σ(r) = σ (constant volatility)
        out[0] = self.params.sigma;
    }

    fn is_diagonal(&self) -> bool {
        true
    }
}

/// Ornstein-Uhlenbeck process (constant mean reversion level).
///
/// This is a special case of Hull-White with constant θ, also known as
/// the Vasicek short rate model.
pub type VasicekProcess = HullWhite1FProcess;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hw1f_params_constant_theta() {
        let params = HullWhite1FParams::new(0.1, 0.01, 0.03);

        assert_eq!(params.kappa, 0.1);
        assert_eq!(params.sigma, 0.01);
        assert_eq!(params.theta_at_time(0.0), 0.03);
        assert_eq!(params.theta_at_time(1.0), 0.03);
        assert_eq!(params.theta_at_time(10.0), 0.03);
    }

    #[test]
    fn test_hw1f_params_time_dependent_theta() {
        let theta_curve = vec![0.02, 0.03, 0.04];
        let theta_times = vec![0.0, 1.0, 2.0];

        let params =
            HullWhite1FParams::with_time_dependent_theta(0.1, 0.01, theta_curve, theta_times);

        assert_eq!(params.theta_at_time(0.0), 0.02);
        assert_eq!(params.theta_at_time(0.5), 0.02);
        assert_eq!(params.theta_at_time(1.0), 0.03);
        assert_eq!(params.theta_at_time(1.5), 0.03);
        assert_eq!(params.theta_at_time(2.0), 0.04);
        assert_eq!(params.theta_at_time(10.0), 0.04);
    }

    #[test]
    fn test_hw1f_drift() {
        let params = HullWhite1FParams::new(0.1, 0.01, 0.03);
        let process = HullWhite1FProcess::new(params);

        let x = vec![0.04]; // Rate above mean
        let mut drift = vec![0.0];

        process.drift(0.0, &x, &mut drift);

        // Drift should be negative (pull back to mean)
        assert!(drift[0] < 0.0);
        assert_eq!(drift[0], 0.1 * (0.03 - 0.04));
    }

    #[test]
    fn test_hw1f_diffusion() {
        let params = HullWhite1FParams::new(0.1, 0.01, 0.03);
        let process = HullWhite1FProcess::new(params);

        let x = vec![0.05];
        let mut diffusion = vec![0.0];

        process.diffusion(0.0, &x, &mut diffusion);

        // Constant diffusion
        assert_eq!(diffusion[0], 0.01);
    }

    #[test]
    fn test_vasicek_alias() {
        let process = VasicekProcess::vasicek(0.1, 0.03, 0.01);

        assert_eq!(process.params().kappa, 0.1);
        assert_eq!(process.params().sigma, 0.01);
        assert_eq!(process.theta_at_time(0.0), 0.03);
    }
}
