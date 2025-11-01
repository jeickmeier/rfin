//! Core traits for Monte Carlo pricing engine.
//!
//! This module defines the fundamental abstractions that enable composable,
//! testable Monte Carlo simulation:
//!
//! - `RandomStream`: RNG abstraction with splittable streams
//! - `StochasticProcess`: SDE specification (drift, diffusion)
//! - `Discretization`: Time-stepping schemes
//! - `PathState`: State information at a point along a path

use std::collections::HashMap;

/// Random stream trait for generating random numbers.
///
/// Implementations must support deterministic stream splitting for parallel execution.
/// Each stream is independent and can be split into substreams identified by a unique ID.
///
/// See unit tests and `examples/` for usage.
pub trait RandomStream: Send + Sync {
    /// Split this stream into a new independent substream.
    ///
    /// The `stream_id` should be unique across all substreams to ensure independence.
    /// This enables deterministic parallel execution where each path gets its own stream.
    fn split(&self, stream_id: u64) -> Self
    where
        Self: Sized;

    /// Fill a buffer with uniform random numbers in [0, 1).
    ///
    /// This is a vectorized operation for efficiency.
    fn fill_u01(&mut self, out: &mut [f64]);

    /// Fill a buffer with standard normal random numbers N(0,1).
    ///
    /// Implementations may use Box-Muller, inverse CDF, or other transforms.
    fn fill_std_normals(&mut self, out: &mut [f64]);

    /// Generate a single uniform random number in [0, 1).
    fn next_u01(&mut self) -> f64 {
        let mut buf = [0.0];
        self.fill_u01(&mut buf);
        buf[0]
    }

    /// Generate a single standard normal random number.
    fn next_std_normal(&mut self) -> f64 {
        let mut buf = [0.0];
        self.fill_std_normals(&mut buf);
        buf[0]
    }
}

/// Map of state variables for a path node.
///
/// Uses static string keys for zero-cost abstraction.
/// Common keys are defined in `state_keys` module.
pub type StateVariables = HashMap<&'static str, f64>;

/// Standard state variable keys.
pub mod state_keys {
    /// Spot price (equity/FX)
    pub const SPOT: &str = "spot";
    /// Stochastic volatility (Heston, etc.)
    pub const VARIANCE: &str = "variance";
    /// Short rate (Hull-White, etc.)
    pub const SHORT_RATE: &str = "short_rate";
    /// Time (in years from valuation)
    pub const TIME: &str = "time";
    /// Step index
    pub const STEP: &str = "step";
    /// FX rate (for multi-asset/quanto products)
    pub const FX_RATE: &str = "fx_rate";
    /// Equity spot (for multi-asset, when SPOT refers to FX)
    pub const EQUITY_SPOT: &str = "equity_spot";
}

/// State information for a point along a Monte Carlo path.
///
/// This struct captures all relevant state variables at a specific time step,
/// analogous to `NodeState` in the tree framework but for MC paths.
#[derive(Clone, Debug)]
pub struct PathState {
    /// Time step index (0 = initial, N = final)
    pub step: usize,
    /// Time in years from valuation date
    pub time: f64,
    /// State variables (spot, variance, rate, etc.)
    pub vars: StateVariables,
}

impl PathState {
    /// Create a new path state.
    pub fn new(step: usize, time: f64) -> Self {
        Self {
            step,
            time,
            vars: StateVariables::new(),
        }
    }

    /// Create a path state with initial variables.
    pub fn with_vars(step: usize, time: f64, vars: StateVariables) -> Self {
        Self { step, time, vars }
    }

    /// Get a state variable by key.
    pub fn get(&self, key: &str) -> Option<f64> {
        self.vars.get(key).copied()
    }

    /// Get a state variable with a default value.
    pub fn get_or(&self, key: &str, default: f64) -> f64 {
        self.vars.get(key).copied().unwrap_or(default)
    }

    /// Set a state variable.
    pub fn set(&mut self, key: &'static str, value: f64) {
        self.vars.insert(key, value);
    }

    /// Get spot price (convenience method).
    pub fn spot(&self) -> Option<f64> {
        self.get(state_keys::SPOT)
    }

    /// Get variance (convenience method).
    pub fn variance(&self) -> Option<f64> {
        self.get(state_keys::VARIANCE)
    }
}

/// Stochastic process specification (SDE).
///
/// A stochastic process defines the drift and diffusion coefficients
/// for a system of SDEs:
///
/// ```text
/// dX_t = μ(t, X_t) dt + Σ(t, X_t) dW_t
/// ```
///
/// where μ is the drift vector and Σ is the diffusion matrix (or diagonal).
///
/// # Example
///
/// GBM for equity under risk-neutral measure:
///
/// ```text
/// dS_t = (r - q) S_t dt + σ S_t dW_t
/// ```
pub trait StochasticProcess: Send + Sync {
    /// Number of state variables.
    fn dim(&self) -> usize;

    /// Number of independent Brownian motions (may differ from dim).
    ///
    /// For example, Heston has dim=2 (S, v) but may have 2 correlated BMs.
    fn num_factors(&self) -> usize {
        self.dim()
    }

    /// Compute drift vector: μ(t, x).
    ///
    /// # Arguments
    ///
    /// * `t` - Time in years
    /// * `x` - Current state vector (length = dim)
    /// * `out` - Output drift vector (length = dim)
    fn drift(&self, t: f64, x: &[f64], out: &mut [f64]);

    /// Compute diffusion matrix or diagonal: Σ(t, x).
    ///
    /// For diagonal diffusion, `out` contains the diagonal elements.
    /// For full matrix, `out` is row-major (or Cholesky factor).
    ///
    /// # Arguments
    ///
    /// * `t` - Time in years
    /// * `x` - Current state vector (length = dim)
    /// * `out` - Output diffusion vector/matrix
    fn diffusion(&self, t: f64, x: &[f64], out: &mut [f64]);

    /// Check if diffusion is diagonal (most common case).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::instruments::common::mc::traits::StochasticProcess;
    /// use finstack_valuations::instruments::common::mc::process::gbm::GbmProcess;
    ///
    /// let gbm = GbmProcess::with_params(0.05, 0.02, 0.2);
    /// assert!(gbm.is_diagonal());  // GBM has diagonal diffusion
    /// ```
    fn is_diagonal(&self) -> bool {
        true
    }
}

/// Time discretization scheme for SDEs.
///
/// A discretization scheme advances the state from time t to t + Δt
/// given random shocks.
///
/// # Example Schemes
///
/// - **Exact**: Analytical solution (GBM, OU)
/// - **Euler-Maruyama**: First-order explicit
/// - **Milstein**: Improved for diagonal diffusion
/// - **QE**: Quadratic-exponential for CIR/Heston variance
///
/// # Arguments to `step`
///
/// * `process` - The stochastic process
/// * `t` - Current time
/// * `dt` - Time step size
/// * `x` - Current state (updated in-place)
/// * `z` - Standard normal shocks (length = num_factors)
/// * `work` - Workspace buffer for intermediate calculations
pub trait Discretization<P: StochasticProcess + ?Sized>: Send + Sync {
    /// Advance state from t to t + dt.
    ///
    /// This method updates `x` in-place using the provided random shocks `z`.
    fn step(&self, process: &P, t: f64, dt: f64, x: &mut [f64], z: &[f64], work: &mut [f64]);

    /// Workspace size required for intermediate calculations.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::instruments::common::mc::traits::{Discretization, StochasticProcess};
    /// use finstack_valuations::instruments::common::mc::process::gbm::GbmProcess;
    /// use finstack_valuations::instruments::common::mc::discretization::exact::ExactGbm;
    ///
    /// let gbm = GbmProcess::with_params(0.05, 0.02, 0.2);
    /// let disc = ExactGbm::new();
    /// let work_size = disc.work_size(&gbm);
    /// assert_eq!(work_size, 0);  // ExactGbm requires no workspace
    /// ```
    fn work_size(&self, process: &P) -> usize {
        process.dim()
    }
}

// Payoff and PathObserver are moved to `instruments::common::models::monte_carlo::traits`.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_state_creation() {
        let state = PathState::new(5, 0.5);
        assert_eq!(state.step, 5);
        assert_eq!(state.time, 0.5);
        assert!(state.vars.is_empty());
    }

    #[test]
    fn test_path_state_vars() {
        let mut state = PathState::new(0, 0.0);
        state.set(state_keys::SPOT, 100.0);
        state.set(state_keys::VARIANCE, 0.04);

        assert_eq!(state.spot(), Some(100.0));
        assert_eq!(state.variance(), Some(0.04));
        assert_eq!(state.get("nonexistent"), None);
        assert_eq!(state.get_or("nonexistent", 42.0), 42.0);
    }
}
