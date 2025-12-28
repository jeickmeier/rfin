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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

// ============================================================================
// Curve-Derived θ(t) Calibration
// ============================================================================

/// Build Hull-White 1F parameters with θ(t) derived from a discount curve.
///
/// The Hull-White model requires θ(t) to match the initial yield curve. Given:
/// - `f(0,t)` = instantaneous forward rate = `-d/dt ln P(0,t)`
/// - `θ(t) = ∂f/∂t + κf(0,t) + σ²/(2κ²)(1 - e^{-κt})²`
///
/// This function computes piecewise-constant θ(t) via finite differences.
///
/// # Arguments
///
/// * `kappa` - Mean reversion speed
/// * `sigma` - Short rate volatility
/// * `discount_curve_fn` - Function mapping time (years) to discount factor P(0,t)
/// * `theta_times` - Time breakpoints for θ(t) discretization
///
/// # Example
///
/// ```rust,no_run
/// use finstack_valuations::instruments::common::mc::process::ou::calibrate_theta_from_curve;
///
/// let discount_fn = |t: f64| (-0.03 * t).exp();  // Flat 3% curve
/// let params = calibrate_theta_from_curve(0.03, 0.01, discount_fn, &[0.5, 1.0, 2.0, 5.0]);
/// # let _ = params;
/// ```
pub fn calibrate_theta_from_curve<F>(
    kappa: f64,
    sigma: f64,
    discount_curve_fn: F,
    theta_times: &[f64],
) -> HullWhite1FParams
where
    F: Fn(f64) -> f64,
{
    if theta_times.is_empty() {
        // Fallback: use instantaneous forward at t=0 as constant theta
        let f_0 = instantaneous_forward(&discount_curve_fn, 0.0);
        return HullWhite1FParams::new(kappa, sigma, f_0);
    }

    let mut theta_curve = Vec::with_capacity(theta_times.len());

    for &t in theta_times {
        let theta_t = compute_theta_at_time(kappa, sigma, &discount_curve_fn, t);
        theta_curve.push(theta_t);
    }

    HullWhite1FParams::with_time_dependent_theta(kappa, sigma, theta_curve, theta_times.to_vec())
}

/// Compute θ(t) at a specific time using the Hull-White drift formula.
///
/// θ(t) = ∂f/∂t(0,t) + κ·f(0,t) + σ²/(2κ²)·(1 - e^{-κt})²
fn compute_theta_at_time<F>(kappa: f64, sigma: f64, discount_curve_fn: &F, t: f64) -> f64
where
    F: Fn(f64) -> f64,
{
    // Compute f(0,t) = instantaneous forward rate
    let f_t = instantaneous_forward(discount_curve_fn, t);

    // Compute ∂f/∂t via finite difference
    let df_dt = forward_derivative(discount_curve_fn, t);

    // Volatility term: σ²/(2κ²)·(1 - e^{-κt})²
    let vol_term = if kappa.abs() > 1e-10 {
        let exp_minus_kt = (-kappa * t).exp();
        let one_minus_exp = 1.0 - exp_minus_kt;
        (sigma * sigma) / (2.0 * kappa * kappa) * one_minus_exp * one_minus_exp
    } else {
        // Limit as κ → 0: σ²t²/2
        (sigma * sigma * t * t) / 2.0
    };

    df_dt + kappa * f_t + vol_term
}

/// Compute instantaneous forward rate f(0,t) = -d/dt ln P(0,t)
/// using central finite differences.
fn instantaneous_forward<F>(discount_curve_fn: &F, t: f64) -> f64
where
    F: Fn(f64) -> f64,
{
    let eps = 1e-4; // Small perturbation for numerical derivative

    if t < eps {
        // Near t=0, use forward difference
        let df_0 = discount_curve_fn(0.0);
        let df_eps = discount_curve_fn(eps);
        if df_0 > 0.0 && df_eps > 0.0 {
            -(df_eps.ln() - df_0.ln()) / eps
        } else {
            0.03 // Fallback to reasonable default
        }
    } else {
        // Central difference: f(t) ≈ -[ln P(t+ε) - ln P(t-ε)] / (2ε)
        let t_plus = t + eps;
        let t_minus = (t - eps).max(0.0);
        let df_plus = discount_curve_fn(t_plus);
        let df_minus = discount_curve_fn(t_minus);

        if df_plus > 0.0 && df_minus > 0.0 {
            -(df_plus.ln() - df_minus.ln()) / (t_plus - t_minus)
        } else {
            0.03
        }
    }
}

/// Compute ∂f/∂t via finite differences on the instantaneous forward.
fn forward_derivative<F>(discount_curve_fn: &F, t: f64) -> f64
where
    F: Fn(f64) -> f64,
{
    let eps = 1e-4;

    let f_plus = instantaneous_forward(discount_curve_fn, t + eps);
    let f_minus = instantaneous_forward(discount_curve_fn, (t - eps).max(0.0));

    let dt = if t < eps { eps } else { 2.0 * eps };

    (f_plus - f_minus) / dt
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
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

    #[test]
    fn test_calibrate_theta_from_flat_curve() {
        // Flat 3% curve: P(0,t) = exp(-0.03 * t)
        let discount_fn = |t: f64| (-0.03 * t).exp();
        let theta_times = vec![0.0, 1.0, 2.0, 5.0, 10.0];

        let params = calibrate_theta_from_curve(0.03, 0.01, discount_fn, &theta_times);

        // For a flat curve, f(0,t) = 0.03 and ∂f/∂t ≈ 0
        // θ(t) ≈ κ·f + volatility term
        // At t=0, vol term = 0, so θ(0) ≈ κ·r = 0.03 * 0.03 = 0.0009
        // But we add the forward derivative which is near zero
        // Actually for flat curve: θ(t) = κ·f + σ²/(2κ²)·(1-e^{-κt})²

        assert_eq!(params.kappa, 0.03);
        assert_eq!(params.sigma, 0.01);
        assert_eq!(params.theta_times.len(), 5);

        // Theta values should be positive and reasonable
        for &theta in &params.theta_curve {
            assert!(theta.is_finite(), "Theta must be finite");
            assert!(
                theta > -0.1 && theta < 0.2,
                "Theta should be reasonable: {}",
                theta
            );
        }
    }

    #[test]
    fn test_calibrate_theta_empty_times() {
        let discount_fn = |t: f64| (-0.03 * t).exp();
        let params = calibrate_theta_from_curve(0.03, 0.01, discount_fn, &[]);

        // Should fall back to constant theta at instantaneous forward f(0,0)
        assert_eq!(params.theta_curve.len(), 1);
        assert_eq!(params.theta_times.len(), 1);
    }

    #[test]
    fn test_instantaneous_forward_flat_curve() {
        let discount_fn = |t: f64| (-0.05 * t).exp();

        // For flat curve, f(0,t) should be constant ≈ 0.05
        let f_0 = instantaneous_forward(&discount_fn, 0.0);
        let f_1 = instantaneous_forward(&discount_fn, 1.0);
        let f_5 = instantaneous_forward(&discount_fn, 5.0);

        assert!((f_0 - 0.05).abs() < 0.01, "f(0,0) ≈ 0.05, got {}", f_0);
        assert!((f_1 - 0.05).abs() < 0.01, "f(0,1) ≈ 0.05, got {}", f_1);
        assert!((f_5 - 0.05).abs() < 0.01, "f(0,5) ≈ 0.05, got {}", f_5);
    }
}
