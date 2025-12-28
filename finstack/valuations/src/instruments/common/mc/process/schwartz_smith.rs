//! Schwartz-Smith two-factor commodity model.
//!
//! Implements a two-factor model for commodity prices that captures both
//! short-term mean-reverting deviations and long-term trends.
//!
//! # SDE
//!
//! ```text
//! dX_t = -κ_X X_t dt + σ_X dW_X      // Short-term deviation (mean-reverting)
//! dY_t = μ_Y dt + σ_Y dW_Y           // Long-term trend (GBM)
//! S_t = exp(X_t + Y_t)               // Spot price
//! ```
//!
//! where:
//! - X_t: Short-term deviation from long-term trend (OU process)
//! - Y_t: Long-term trend (arithmetic Brownian motion)
//! - κ_X: Mean reversion speed for short-term component
//! - σ_X, σ_Y: Volatilities
//! - ρ: Correlation between X and Y

use super::super::traits::StochasticProcess;

/// Schwartz-Smith process parameters.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SchwartzSmithParams {
    /// Mean reversion speed for short-term deviation (κ_X)
    pub kappa_x: f64,
    /// Volatility of short-term component (σ_X)
    pub sigma_x: f64,
    /// Drift of long-term trend (μ_Y)
    pub mu_y: f64,
    /// Volatility of long-term component (σ_Y)
    pub sigma_y: f64,
    /// Correlation between X and Y (ρ)
    pub rho: f64,
}

impl SchwartzSmithParams {
    /// Create new Schwartz-Smith parameters.
    ///
    /// # Arguments
    ///
    /// * `kappa_x` - Mean reversion speed (must be > 0)
    /// * `sigma_x` - Short-term volatility (must be > 0)
    /// * `mu_y` - Long-term drift
    /// * `sigma_y` - Long-term volatility (must be > 0)
    /// * `rho` - Correlation between X and Y (must be in [-1, 1])
    pub fn new(kappa_x: f64, sigma_x: f64, mu_y: f64, sigma_y: f64, rho: f64) -> Self {
        assert!(kappa_x > 0.0, "kappa_x must be positive");
        assert!(sigma_x > 0.0, "sigma_x must be positive");
        assert!(sigma_y > 0.0, "sigma_y must be positive");
        assert!(
            (-1.0..=1.0).contains(&rho),
            "Correlation rho must be in [-1, 1]"
        );

        Self {
            kappa_x,
            sigma_x,
            mu_y,
            sigma_y,
            rho,
        }
    }
}

/// Schwartz-Smith two-factor commodity process.
///
/// State: [X_t, Y_t] where X_t is short-term deviation and Y_t is long-term trend
/// Spot price: S_t = exp(X_t + Y_t)
///
/// # State Variables
///
/// - `state[0]` = X_t (short-term deviation)
/// - `state[1]` = Y_t (long-term trend)
///
/// # Factors
///
/// Two correlated Brownian motions with correlation ρ.
#[derive(Clone, Debug)]
pub struct SchwartzSmithProcess {
    params: SchwartzSmithParams,
    /// Initial values [X_0, Y_0]
    initial: [f64; 2],
}

impl SchwartzSmithProcess {
    /// Create a new Schwartz-Smith process.
    ///
    /// # Arguments
    ///
    /// * `params` - Process parameters
    /// * `initial_x` - Initial short-term deviation X_0
    /// * `initial_y` - Initial long-term trend Y_0
    pub fn new(params: SchwartzSmithParams, initial_x: f64, initial_y: f64) -> Self {
        Self {
            params,
            initial: [initial_x, initial_y],
        }
    }

    /// Create from spot price and initial state.
    ///
    /// If X_0 = 0 and Y_0 = ln(S_0), then S_0 = exp(X_0 + Y_0) = exp(ln(S_0)) = S_0.
    ///
    /// # Arguments
    ///
    /// * `params` - Process parameters
    /// * `initial_spot` - Initial spot price S_0
    /// * `initial_x` - Initial short-term deviation (default 0.0 if None)
    pub fn from_spot(
        params: SchwartzSmithParams,
        initial_spot: f64,
        initial_x: Option<f64>,
    ) -> Self {
        let x_0 = initial_x.unwrap_or(0.0);
        let y_0 = initial_spot.ln() - x_0; // Ensure S_0 = exp(X_0 + Y_0)
        Self::new(params, x_0, y_0)
    }

    /// Get parameters.
    pub fn params(&self) -> &SchwartzSmithParams {
        &self.params
    }

    /// Get initial state.
    pub fn initial_state(&self) -> [f64; 2] {
        self.initial
    }

    /// Compute spot price from state [X, Y].
    pub fn spot_from_state(&self, state: &[f64]) -> f64 {
        assert_eq!(state.len(), 2, "State must have dimension 2");
        (state[0] + state[1]).exp()
    }
}

impl StochasticProcess for SchwartzSmithProcess {
    fn dim(&self) -> usize {
        2
    }

    fn num_factors(&self) -> usize {
        2
    }

    fn drift(&self, _t: f64, x: &[f64], out: &mut [f64]) {
        // dX: -κ_X * X (mean-reverting to zero)
        out[0] = -self.params.kappa_x * x[0];
        // dY: μ_Y (constant drift)
        out[1] = self.params.mu_y;
    }

    fn diffusion(&self, _t: f64, _x: &[f64], out: &mut [f64]) {
        // Diffusion matrix (diagonal elements)
        // For correlated factors, the discretization will apply Cholesky
        out[0] = self.params.sigma_x;
        out[1] = self.params.sigma_y;
    }

    fn is_diagonal(&self) -> bool {
        // Not diagonal due to correlation between X and Y
        false
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_schwartz_smith_creation() {
        let params = SchwartzSmithParams::new(2.0, 0.30, 0.02, 0.15, -0.5);
        let process = SchwartzSmithProcess::new(params, 0.0, 4.5);

        assert_eq!(process.dim(), 2);
        assert_eq!(process.num_factors(), 2);
        assert!(!process.is_diagonal()); // Has correlation
    }

    #[test]
    fn test_schwartz_smith_from_spot() {
        let params = SchwartzSmithParams::new(2.0, 0.30, 0.02, 0.15, -0.5);
        let spot_0 = 90.0;
        let process = SchwartzSmithProcess::from_spot(params, spot_0, None);

        let state = process.initial_state();
        let computed_spot = process.spot_from_state(&state);

        // S_0 = exp(X_0 + Y_0) = exp(0 + ln(90)) = 90
        assert!((computed_spot - spot_0).abs() < 1e-10);
    }

    #[test]
    fn test_schwartz_smith_drift() {
        let params = SchwartzSmithParams::new(2.0, 0.30, 0.02, 0.15, -0.5);
        let process = SchwartzSmithProcess::new(params, 0.0, 4.5);

        let x = [0.1, 4.5];
        let mut drift = [0.0; 2];

        process.drift(0.0, &x, &mut drift);

        // dX/dt = -2.0 * 0.1 = -0.2
        assert!((drift[0] - (-0.2)).abs() < 1e-10);
        // dY/dt = 0.02
        assert!((drift[1] - 0.02).abs() < 1e-10);
    }

    #[test]
    fn test_schwartz_smith_diffusion() {
        let params = SchwartzSmithParams::new(2.0, 0.30, 0.02, 0.15, -0.5);
        let process = SchwartzSmithProcess::new(params, 0.0, 4.5);

        let x = [0.1, 4.5];
        let mut diffusion = [0.0; 2];

        process.diffusion(0.0, &x, &mut diffusion);

        assert!((diffusion[0] - 0.30).abs() < 1e-10);
        assert!((diffusion[1] - 0.15).abs() < 1e-10);
    }

    #[test]
    fn test_spot_from_state() {
        let params = SchwartzSmithParams::new(2.0, 0.30, 0.02, 0.15, -0.5);
        let process = SchwartzSmithProcess::new(params, 0.0, 4.5);

        let state = [0.0, 4.5]; // X=0, Y=ln(90)≈4.5
        let spot = process.spot_from_state(&state);

        // S = exp(0 + 4.5) ≈ 90
        assert!((spot - 90.0).abs() < 1.0); // Allow small tolerance
    }
}
