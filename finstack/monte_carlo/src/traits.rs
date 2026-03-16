//! Core traits for Monte Carlo simulation and pricing.
//!
//! This module defines the fundamental abstractions that enable composable,
//! testable Monte Carlo simulation:
//!
//! - `RandomStream`: RNG abstraction with splittable streams
//! - `StochasticProcess`: SDE specification (drift, diffusion)
//! - `Discretization`: Time-stepping schemes
//! - `PathState`: State information at a point along a path
//! - `Payoff`: Payoff computation with currency safety
//! - `PathObserver`: Path observer for collecting statistics along paths

use super::paths::CashflowType;
use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_core::HashMap;

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

    /// Returns whether this RNG supports stream splitting for parallel execution.
    ///
    /// Returns `true` if `split(stream_id)` produces independent streams for any `stream_id`.
    /// Returns `false` for quasi-random sequences (e.g., Sobol) that cannot be meaningfully split.
    ///
    /// The MC engine checks this before enabling parallel mode.
    fn supports_splitting(&self) -> bool {
        true // Default: most PRNGs support splitting
    }
}

/// Map of state variables for a path node (used only for dynamic/non-standard keys).
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
    /// Current NPV of remaining cashflows (for MTM)
    pub const NPV_CURRENT: &str = "npv_current";
    /// NPV from previous timestep (for MTM)
    pub const NPV_PREVIOUS: &str = "npv_previous";
    /// Mark-to-market P&L (change in NPV)
    pub const MTM_PNL: &str = "mtm_pnl";
}

/// Indexed state variable key for O(1) array access in the MC inner loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum StateKey {
    /// Spot price (equity/FX)
    Spot = 0,
    /// Stochastic variance (Heston, etc.)
    Variance = 1,
    /// Short rate (Hull-White, etc.)
    ShortRate = 2,
    /// Time in years from valuation
    Time = 3,
    /// Step index
    Step = 4,
    /// FX rate (multi-asset/quanto)
    FxRate = 5,
    /// Equity spot (multi-asset, when Spot refers to FX)
    EquitySpot = 6,
    /// Current NPV of remaining cashflows
    NpvCurrent = 7,
    /// NPV from previous timestep
    NpvPrevious = 8,
    /// Mark-to-market P&L
    MtmPnl = 9,
    /// Credit spread
    CreditSpread = 10,
}

const STATE_ARRAY_LEN: usize = 11;

/// Resolve a string key to its fixed-slot `StateKey`, if it matches a known key.
fn resolve_key(key: &str) -> Option<StateKey> {
    match key {
        "spot" => Some(StateKey::Spot),
        "variance" => Some(StateKey::Variance),
        "short_rate" => Some(StateKey::ShortRate),
        "time" => Some(StateKey::Time),
        "step" => Some(StateKey::Step),
        "fx_rate" => Some(StateKey::FxRate),
        "equity_spot" => Some(StateKey::EquitySpot),
        "npv_current" => Some(StateKey::NpvCurrent),
        "npv_previous" => Some(StateKey::NpvPrevious),
        "mtm_pnl" => Some(StateKey::MtmPnl),
        "credit_spread" => Some(StateKey::CreditSpread),
        _ => None,
    }
}

/// State information for a point along a Monte Carlo path.
///
/// Uses a fixed-size array for known state keys (O(1) access) with a HashMap
/// fallback for dynamic keys (basket "spot_0", "spot_1", etc.).
#[derive(Debug, Clone)]
pub struct PathState {
    /// Time step index (0 = initial, N = final)
    pub step: usize,
    /// Time in years from valuation date
    pub time: f64,
    /// Fixed-slot storage for known state keys (indexed by `StateKey`)
    fixed: [f64; STATE_ARRAY_LEN],
    /// Bitmask tracking which fixed slots contain valid values
    fixed_set: u16,
    /// Fallback for dynamic keys not in the fixed array (e.g., basket spot keys)
    dynamic: HashMap<&'static str, f64>,
    cashflows: Vec<(f64, f64, CashflowType)>,
    uniform_random: f64,
}

impl PathState {
    /// Create a new path state.
    pub fn new(step: usize, time: f64) -> Self {
        Self {
            step,
            time,
            fixed: [0.0; STATE_ARRAY_LEN],
            fixed_set: 0,
            dynamic: HashMap::default(),
            cashflows: Vec::new(),
            uniform_random: 0.0,
        }
    }

    /// Create a path state with initial variables.
    pub fn with_vars(step: usize, time: f64, vars: StateVariables) -> Self {
        let mut ps = Self::new(step, time);
        for (key, value) in vars {
            ps.set(key, value);
        }
        ps
    }

    /// Set a state variable by indexed key (fast path, no hashing).
    #[inline]
    pub fn set_key(&mut self, key: StateKey, value: f64) {
        let idx = key as usize;
        self.fixed[idx] = value;
        self.fixed_set |= 1 << idx;
    }

    /// Get a state variable by indexed key (fast path, no hashing).
    #[inline]
    pub fn get_key(&self, key: StateKey) -> Option<f64> {
        let idx = key as usize;
        if self.fixed_set & (1 << idx) != 0 {
            Some(self.fixed[idx])
        } else {
            None
        }
    }

    /// Set the uniform random value for this timestep.
    pub fn set_uniform_random(&mut self, u: f64) {
        self.uniform_random = u;
    }

    /// Get the uniform random value for this timestep.
    pub fn uniform_random(&self) -> f64 {
        self.uniform_random
    }

    /// Get a state variable by string key.
    /// Routes known keys to the fixed array; unknown keys to the dynamic HashMap.
    pub fn get(&self, key: &str) -> Option<f64> {
        if let Some(sk) = resolve_key(key) {
            self.get_key(sk)
        } else {
            self.dynamic.get(key).copied()
        }
    }

    /// Get a state variable with a default value.
    pub fn get_or(&self, key: &str, default: f64) -> f64 {
        self.get(key).unwrap_or(default)
    }

    /// Set a state variable by string key.
    /// Routes known keys to the fixed array; unknown keys to the dynamic HashMap.
    pub fn set(&mut self, key: &'static str, value: f64) {
        if let Some(sk) = resolve_key(key) {
            self.set_key(sk, value);
        } else {
            self.dynamic.insert(key, value);
        }
    }

    /// Backward-compatible access to all state variables as a HashMap.
    /// Merges fixed-slot values with dynamic values. This allocates --
    /// prefer `get`/`get_key` on hot paths.
    pub fn vars(&self) -> StateVariables {
        let mut map = self.dynamic.clone();
        let key_names: [&'static str; STATE_ARRAY_LEN] = [
            state_keys::SPOT,
            state_keys::VARIANCE,
            state_keys::SHORT_RATE,
            state_keys::TIME,
            state_keys::STEP,
            state_keys::FX_RATE,
            state_keys::EQUITY_SPOT,
            state_keys::NPV_CURRENT,
            state_keys::NPV_PREVIOUS,
            state_keys::MTM_PNL,
            "credit_spread",
        ];
        for (i, key_name) in key_names.iter().enumerate().take(STATE_ARRAY_LEN) {
            if self.fixed_set & (1 << i) != 0 {
                map.insert(*key_name, self.fixed[i]);
            }
        }
        map
    }

    /// Get spot price (convenience method).
    #[inline]
    pub fn spot(&self) -> Option<f64> {
        self.get_key(StateKey::Spot)
    }

    /// Get variance (convenience method).
    #[inline]
    pub fn variance(&self) -> Option<f64> {
        self.get_key(StateKey::Variance)
    }

    /// Add a cashflow to this state.
    pub fn add_cashflow(&mut self, time: f64, amount: f64) {
        self.cashflows.push((time, amount, CashflowType::Other));
    }

    /// Add a typed cashflow to this state.
    pub fn add_typed_cashflow(&mut self, time: f64, amount: f64, cf_type: CashflowType) {
        self.cashflows.push((time, amount, cf_type));
    }

    /// Take all cashflows from this state, leaving it empty.
    pub fn take_cashflows(&mut self) -> Vec<(f64, f64, CashflowType)> {
        std::mem::take(&mut self.cashflows)
    }

    /// Get a reference to cashflows (for inspection, not taking ownership).
    pub fn cashflows(&self) -> &[(f64, f64, CashflowType)] {
        &self.cashflows
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
    fn is_diagonal(&self) -> bool {
        true
    }

    /// Populate a `PathState` from the raw state vector.
    ///
    /// Maps state vector indices to named keys (SPOT, VARIANCE, etc.)
    /// so payoffs can access state by name. Override for processes whose
    /// state layout differs from the default equity model.
    ///
    /// Default mapping (suitable for GBM and Heston-like models):
    /// - `x[0]` => `SPOT`
    /// - `x[1]` => `VARIANCE` (if dim >= 2)
    fn populate_path_state(&self, x: &[f64], state: &mut PathState) {
        if !x.is_empty() {
            state.set(state_keys::SPOT, x[0]);
        }
        if x.len() >= 2 {
            state.set(state_keys::VARIANCE, x[1]);
        }
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
    fn work_size(&self, process: &P) -> usize {
        process.dim()
    }
}

// ---------------------------------------------------------------------------
// Pricing-specific traits
// ---------------------------------------------------------------------------

/// Payoff computation with currency safety.
///
/// Payoffs accumulate path information via `on_event` calls and
/// return a final `Money` value. This ensures all results carry
/// explicit currency information.
pub trait Payoff: Send + Sync + Clone {
    /// Process a path event (fixing, barrier check, etc.).
    ///
    /// The PathState is mutable to allow payoffs to record cashflows
    /// using `state.add_cashflow()`. These cashflows will be transferred
    /// to PathPoint during path capture.
    fn on_event(&mut self, state: &mut PathState);

    /// Compute final payoff value in the specified currency (undiscounted).
    fn value(&self, currency: Currency) -> Money;

    /// Reset payoff state for next path.
    fn reset(&mut self);

    /// Optional: discount factor to apply; default is 1.0 (no discounting).
    fn discount_factor(&self) -> f64 {
        1.0
    }

    /// Optional hook invoked at the start of each path with access to RNG.
    ///
    /// Useful to draw per-path random variables (e.g., default threshold E ~ Exp(1)).
    fn on_path_start<R: RandomStream>(&mut self, _rng: &mut R) {}
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_path_state_creation() {
        let state = PathState::new(5, 0.5);
        assert_eq!(state.step, 5);
        assert_eq!(state.time, 0.5);
        assert!(state.vars().is_empty());
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

    #[test]
    fn test_path_state_cashflows() {
        let mut state = PathState::new(1, 0.25);

        assert!(state.cashflows().is_empty());

        state.add_cashflow(0.25, 1000.0);
        state.add_cashflow(0.25, 500.0);

        assert_eq!(state.cashflows().len(), 2);
        assert_eq!(state.cashflows()[0], (0.25, 1000.0, CashflowType::Other));
        assert_eq!(state.cashflows()[1], (0.25, 500.0, CashflowType::Other));

        let cashflows = state.take_cashflows();
        assert_eq!(cashflows.len(), 2);
        assert_eq!(cashflows[0], (0.25, 1000.0, CashflowType::Other));
        assert_eq!(cashflows[1], (0.25, 500.0, CashflowType::Other));

        assert!(state.cashflows().is_empty());

        state.add_typed_cashflow(0.5, 2000.0, CashflowType::Interest);
        state.add_typed_cashflow(0.5, 100.0, CashflowType::Principal);
        assert_eq!(state.cashflows().len(), 2);
        assert_eq!(state.cashflows()[0], (0.5, 2000.0, CashflowType::Interest));
        assert_eq!(state.cashflows()[1], (0.5, 100.0, CashflowType::Principal));
    }
}
