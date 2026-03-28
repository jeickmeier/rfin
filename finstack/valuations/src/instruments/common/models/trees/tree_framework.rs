//! Generic tree-based pricing framework for financial instruments.
//!
//! This module provides a flexible lattice pricing engine that can accommodate
//! various tree types (binomial, trinomial) and multiple state variables
//! (equity + rates, equity + credit spread, etc.) without requiring code changes
//! to the core pricing logic.
//!
//! Barrier option support is provided via the `BarrierState` structure in
//! `NodeState`. Tree models can track barrier conditions and check knock-in/out
//! status using the provided helper methods.
//!
//! NOTE: Performance enhancements (parallel Greeks, caching of node values,
//!       and optional SIMD) are intentionally deferred to keep the initial
//!       implementation simple and deterministic.
//!
//! ## Serialization Policy
//!
//! Tree models and their parameter types are **transient runtime structures** and
//! do not currently implement `Serialize`/`Deserialize`. This is by design:
//! - Tree configurations are created on-demand during pricing
//! - Parameters are derived from market data or hardcoded defaults
//! - No current use case requires persisting tree configurations
//!
//! If a future requirement emerges (e.g., scenario storage, calibration persistence),
//! add serde support **only to configuration structs** (e.g., `TreeParameters`,
//! `EvolutionParams`) using `#[derive(Serialize, Deserialize)]`.
//! Keep runtime engine types (`BinomialTree`, etc.) non-serializable.
//!
//! See `docs/TREE_PARAMS_SERIALIZATION_AUDIT.md` for audit results and extension pattern.

use finstack_core::market_data::context::MarketContext;
use finstack_core::HashMap;
use finstack_core::Result;

pub use finstack_core::math::time_grid::{
    map_date_to_step, map_dates_to_steps, map_exercise_dates_to_steps,
};

/// Standard state variable keys for consistency
pub mod state_keys {
    /// Underlying asset price (equity)
    pub const SPOT: &str = "spot";
    /// Risk-free interest rate
    pub const INTEREST_RATE: &str = "interest_rate";
    /// Credit spread
    pub const CREDIT_SPREAD: &str = "credit_spread";
    /// Hazard rate (default intensity) for credit modeling
    pub const HAZARD_RATE: &str = "hazard_rate";
    /// Dividend yield
    pub const DIVIDEND_YIELD: &str = "dividend_yield";
    /// Volatility
    pub const VOLATILITY: &str = "volatility";
    /// Barrier touched up-flag (1.0 if touched at this node, else 0.0)
    pub const BARRIER_TOUCHED_UP: &str = "barrier_touched_up";
    /// Barrier touched down-flag (1.0 if touched at this node, else 0.0)
    pub const BARRIER_TOUCHED_DOWN: &str = "barrier_touched_down";
    /// Discount factor at the current node (pre-computed for performance)
    pub const DF: &str = "df";
    /// Rate volatility for two-factor equity+rates models
    pub const RATE_VOLATILITY: &str = "rate_volatility";
}

/// Map of state variables for a tree node
pub type StateVariables = HashMap<&'static str, f64>;

/// Complete state information for a node in the pricing tree
#[derive(Clone)]
pub struct NodeState<'a> {
    /// Time step index (0 to N)
    pub step: usize,
    /// Time in years from valuation date
    pub time: f64,
    /// Map of all state variables at this node (reference to avoid cloning)
    pub vars: &'a StateVariables,
    /// Access to market context for additional data
    pub market_context: &'a MarketContext,
    /// Barrier state tracking (if applicable)
    pub barrier_state: Option<BarrierState>,
    /// Cached spot price for performance (avoids hash lookup)
    pub spot: Option<f64>,
    /// Cached interest rate for performance (avoids hash lookup)
    pub interest_rate: Option<f64>,
    /// Cached hazard rate for performance (avoids hash lookup)
    pub hazard_rate: Option<f64>,
    /// Cached discount factor for performance (avoids hash lookup)
    pub df: Option<f64>,
}

/// Simple barrier state tracking for barrier options
#[derive(Debug, Clone, Default)]
pub struct BarrierState {
    /// Whether barrier has been hit during the path
    pub barrier_hit: bool,
    /// Barrier level (for checking)
    pub barrier_level: f64,
    /// Barrier type
    pub barrier_type: BarrierType,
}

/// Types of barrier conditions
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BarrierType {
    /// Up-and-out (option knocks out when spot > barrier)
    #[default]
    UpAndOut,
    /// Up-and-in (option knocks in when spot > barrier)
    UpAndIn,
    /// Down-and-out (option knocks out when spot < barrier)
    DownAndOut,
    /// Down-and-in (option knocks in when spot < barrier)
    DownAndIn,
}

/// Pre-extracted state variable cache to avoid redundant HashMap lookups in hot paths.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct CachedValues {
    /// Spot price
    pub spot: Option<f64>,
    /// Interest rate
    pub interest_rate: Option<f64>,
    /// Hazard rate (default intensity)
    pub hazard_rate: Option<f64>,
    /// Discount factor
    pub df: Option<f64>,
}

impl<'a> NodeState<'a> {
    /// Create a new node state
    pub fn new(
        step: usize,
        time: f64,
        vars: &'a StateVariables,
        market_context: &'a MarketContext,
    ) -> Self {
        let cached = CachedValues {
            spot: vars.get(state_keys::SPOT).copied(),
            interest_rate: vars.get(state_keys::INTEREST_RATE).copied(),
            hazard_rate: vars.get(state_keys::HAZARD_RATE).copied(),
            df: vars.get(state_keys::DF).copied(),
        };

        Self {
            step,
            time,
            vars,
            market_context,
            barrier_state: None,
            spot: cached.spot,
            interest_rate: cached.interest_rate,
            hazard_rate: cached.hazard_rate,
            df: cached.df,
        }
    }

    /// Create a new node state with pre-extracted cached values.
    ///
    /// Avoids redundant HashMap lookups when the caller already knows the values.
    /// Used in hot paths (backward induction) where we just inserted the values.
    #[inline]
    pub(crate) fn with_cached(
        step: usize,
        time: f64,
        vars: &'a StateVariables,
        market_context: &'a MarketContext,
        cached: CachedValues,
    ) -> Self {
        Self {
            step,
            time,
            vars,
            market_context,
            barrier_state: None,
            spot: cached.spot,
            interest_rate: cached.interest_rate,
            hazard_rate: cached.hazard_rate,
            df: cached.df,
        }
    }

    /// Create a new node state with barrier tracking and pre-extracted cached values.
    #[inline]
    pub(crate) fn with_cached_barrier(
        step: usize,
        time: f64,
        vars: &'a StateVariables,
        market_context: &'a MarketContext,
        barrier_state: BarrierState,
        cached: CachedValues,
    ) -> Self {
        Self {
            step,
            time,
            vars,
            market_context,
            barrier_state: Some(barrier_state),
            spot: cached.spot,
            interest_rate: cached.interest_rate,
            hazard_rate: cached.hazard_rate,
            df: cached.df,
        }
    }

    /// Create a new node state with barrier tracking
    pub fn new_with_barrier(
        step: usize,
        time: f64,
        vars: &'a StateVariables,
        market_context: &'a MarketContext,
        barrier_state: BarrierState,
    ) -> Self {
        // Pre-extract commonly accessed variables to avoid hash lookups in hot path
        let spot = vars.get(state_keys::SPOT).copied();
        let interest_rate = vars.get(state_keys::INTEREST_RATE).copied();
        let hazard_rate = vars.get(state_keys::HAZARD_RATE).copied();
        let df = vars.get(state_keys::DF).copied();

        Self {
            step,
            time,
            vars,
            market_context,
            barrier_state: Some(barrier_state),
            spot,
            interest_rate,
            hazard_rate,
            df,
        }
    }

    /// Get a state variable by key
    #[inline]
    pub fn get_var(&self, key: &str) -> Option<f64> {
        self.vars.get(key).copied()
    }

    /// Get a state variable by key with a default value
    #[inline]
    pub fn get_var_or(&self, key: &str, default: f64) -> f64 {
        self.vars.get(key).copied().unwrap_or(default)
    }

    /// Get spot price (convenience method, uses cached value)
    #[inline]
    pub fn spot(&self) -> Option<f64> {
        self.spot
    }

    /// Get interest rate (convenience method, uses cached value)
    #[inline]
    pub fn interest_rate(&self) -> Option<f64> {
        self.interest_rate
    }

    /// Get credit spread (convenience method)
    #[inline]
    pub fn credit_spread(&self) -> Option<f64> {
        self.get_var(state_keys::CREDIT_SPREAD)
    }

    /// Get hazard rate (convenience method, uses cached value)
    #[inline]
    pub fn hazard_rate(&self) -> Option<f64> {
        self.hazard_rate
    }

    /// Get discount factor (convenience method, uses cached value)
    #[inline]
    pub fn discount_factor(&self) -> Option<f64> {
        self.df
    }

    /// Check if barrier has been hit (for barrier options)
    pub fn is_barrier_hit(&self) -> bool {
        self.barrier_state.as_ref().is_some_and(|bs| bs.barrier_hit)
    }

    /// Update barrier state based on current spot price
    pub fn update_barrier_state(&mut self, spot_price: f64) {
        if let Some(ref mut barrier_state) = self.barrier_state {
            if !barrier_state.barrier_hit {
                let hit = match barrier_state.barrier_type {
                    BarrierType::UpAndOut | BarrierType::UpAndIn => {
                        spot_price >= barrier_state.barrier_level
                    }
                    BarrierType::DownAndOut | BarrierType::DownAndIn => {
                        spot_price <= barrier_state.barrier_level
                    }
                };
                barrier_state.barrier_hit = hit;
            }
        }
    }

    /// Check if option should be knocked out (for barrier options)
    pub fn is_knocked_out(&self) -> bool {
        if let Some(ref barrier_state) = self.barrier_state {
            barrier_state.barrier_hit
                && matches!(
                    barrier_state.barrier_type,
                    BarrierType::UpAndOut | BarrierType::DownAndOut
                )
        } else {
            false
        }
    }

    /// Check if option should be knocked in (for barrier options)
    pub fn is_knocked_in(&self) -> bool {
        if let Some(ref barrier_state) = self.barrier_state {
            barrier_state.barrier_hit
                && matches!(
                    barrier_state.barrier_type,
                    BarrierType::UpAndIn | BarrierType::DownAndIn
                )
        } else {
            true // If no barrier, always "knocked in"
        }
    }

    /// Whether the up barrier was touched at this node (discrete monitoring flag)
    pub fn barrier_touched_up(&self) -> bool {
        self.get_var(state_keys::BARRIER_TOUCHED_UP)
            .map(|v| v > 0.5)
            .unwrap_or(false)
    }

    /// Whether the down barrier was touched at this node (discrete monitoring flag)
    pub fn barrier_touched_down(&self) -> bool {
        self.get_var(state_keys::BARRIER_TOUCHED_DOWN)
            .map(|v| v > 0.5)
            .unwrap_or(false)
    }
}

/// Trait for instrument-specific valuation logic on a tree
pub trait TreeValuator {
    /// Calculate the instrument's value at a terminal node (maturity)
    fn value_at_maturity(&self, state: &NodeState) -> Result<f64>;

    /// Calculate the instrument's value at an intermediate node
    ///
    /// This method implements the core decision logic (e.g., hold vs. exercise)
    /// and receives the discounted expected continuation value from child nodes.
    ///
    /// # Arguments
    ///
    /// * `state` - Node state with cached common variables
    /// * `continuation_value` - Discounted expected value from child nodes
    /// * `dt` - Time step size (passed explicitly to avoid hash lookup)
    fn value_at_node(&self, state: &NodeState, continuation_value: f64, dt: f64) -> Result<f64>;
}

/// Trait for generic tree models (binomial, trinomial, etc.)
pub trait TreeModel {
    /// Price an instrument using this tree model
    ///
    /// # Arguments
    /// * `initial_vars` - Initial state variables at t=0
    /// * `time_to_maturity` - Total time to maturity in years
    /// * `market_context` - Market data context
    /// * `valuator` - Instrument-specific valuation logic
    #[must_use = "pricing result should not be discarded"]
    fn price<V: TreeValuator>(
        &self,
        initial_vars: StateVariables,
        time_to_maturity: f64,
        market_context: &MarketContext,
        valuator: &V,
    ) -> Result<f64>;

    /// Calculate Greeks using finite differences
    ///
    /// # Arguments
    /// * `initial_vars` - Initial state variables at t=0
    /// * `time_to_maturity` - Total time to maturity in years
    /// * `market_context` - Market data context
    /// * `valuator` - Instrument-specific valuation logic
    /// * `bump_size` - Size of finite difference bumps (default: 1% of base value)
    fn calculate_greeks<V: TreeValuator>(
        &self,
        initial_vars: StateVariables,
        time_to_maturity: f64,
        market_context: &MarketContext,
        valuator: &V,
        bump_size: Option<f64>,
    ) -> Result<TreeGreeks> {
        let bump = bump_size.unwrap_or(0.01);

        // Use a single mutable clone for all bump scenarios, restoring each
        // variable after its up/down pricing calls to avoid repeated cloning.
        let mut vars = initial_vars;

        // Base price
        let base_price = self.price(vars.clone(), time_to_maturity, market_context, valuator)?;

        // Calculate Delta (spot sensitivity)
        let mut greeks = TreeGreeks {
            price: base_price,
            delta: 0.0,
            gamma: 0.0,
            vega: 0.0,
            theta: 0.0,
            rho: 0.0,
        };

        if let Some(&spot) = vars.get(state_keys::SPOT) {
            let h = bump * spot;

            vars.insert(state_keys::SPOT, spot + h);
            let price_up = self.price(vars.clone(), time_to_maturity, market_context, valuator)?;

            vars.insert(state_keys::SPOT, spot - h);
            let price_down =
                self.price(vars.clone(), time_to_maturity, market_context, valuator)?;

            vars.insert(state_keys::SPOT, spot); // restore

            greeks.delta = (price_up - price_down) / (2.0 * h);
            greeks.gamma = (price_up - 2.0 * base_price + price_down) / (h * h);
        }

        // Calculate Vega (volatility sensitivity) using central difference
        // This reduces first-order error compared to one-sided bumps
        if let Some(&vol) = vars.get(state_keys::VOLATILITY) {
            let h = 0.01; // 1% vol bump

            vars.insert(state_keys::VOLATILITY, vol + h);
            let price_vol_up =
                self.price(vars.clone(), time_to_maturity, market_context, valuator)?;

            // Vol down (ensure positive volatility)
            let vol_down = (vol - h).max(1e-6);
            vars.insert(state_keys::VOLATILITY, vol_down);
            let price_vol_down =
                self.price(vars.clone(), time_to_maturity, market_context, valuator)?;

            vars.insert(state_keys::VOLATILITY, vol); // restore

            // Central difference vega (per 1% vol move)
            greeks.vega = (price_vol_up - price_vol_down) / 2.0;
        }

        // Calculate Rho (rate sensitivity) using central difference
        if let Some(&rate) = vars.get(state_keys::INTEREST_RATE) {
            let h = 0.0001; // 1bp rate bump

            vars.insert(state_keys::INTEREST_RATE, rate + h);
            let price_rate_up =
                self.price(vars.clone(), time_to_maturity, market_context, valuator)?;

            vars.insert(state_keys::INTEREST_RATE, rate - h);
            let price_rate_down =
                self.price(vars.clone(), time_to_maturity, market_context, valuator)?;

            vars.insert(state_keys::INTEREST_RATE, rate); // restore

            // Central difference rho (per 1bp move)
            greeks.rho = (price_rate_up - price_rate_down) / 2.0;
        }

        // Calculate Theta (time decay) - use 1 day bump
        let dt = 1.0 / 365.25;
        if time_to_maturity > dt {
            let price_tomorrow =
                self.price(vars, time_to_maturity - dt, market_context, valuator)?;
            greeks.theta = -(base_price - price_tomorrow) / dt;
        }

        Ok(greeks)
    }
}

/// Greeks calculated from tree models.
///
/// # Units and Conventions
///
/// - **Delta**: Per unit of spot (e.g., delta=0.5 means $0.50 per $1 spot move)
/// - **Gamma**: Per unit of spot squared (second derivative)
/// - **Vega**: Per 1% absolute volatility move (e.g., 20% → 21%)
/// - **Theta**: Per day (negative for long positions typically)
/// - **Rho**: Per 1 basis point (0.01%) interest rate move
#[derive(Debug, Clone)]
pub struct TreeGreeks {
    /// Instrument price
    pub price: f64,
    /// Delta (spot sensitivity per unit spot move)
    pub delta: f64,
    /// Gamma (curvature, second derivative w.r.t. spot)
    pub gamma: f64,
    /// Vega (volatility sensitivity per 1% vol move)
    pub vega: f64,
    /// Theta (time decay per day)
    pub theta: f64,
    /// Rho (interest rate sensitivity per 1bp rate move)
    pub rho: f64,
}

impl TreeGreeks {
    /// Apply Richardson extrapolation to combine Greeks from two step sizes.
    ///
    /// Richardson extrapolation improves accuracy by combining results from
    /// trees with N and 2N steps:
    ///
    /// ```text
    /// result_improved = (4 × result_fine - result_coarse) / 3
    /// ```
    ///
    /// This cancels the O(h²) error term, achieving O(h⁴) accuracy.
    ///
    /// # Important: Refinement Ratio
    ///
    /// The `(4*fine - coarse)/3` formula is only correct when `fine` uses
    /// exactly **2x** the number of steps as `coarse` (i.e., step size ratio = 2).
    /// For a different refinement ratio r, the formula becomes:
    /// `(r² × fine - coarse) / (r² - 1)`.
    ///
    /// # Arguments
    ///
    /// * `coarse` - Greeks from tree with N steps
    /// * `fine` - Greeks from tree with 2N steps (must be exactly 2x)
    ///
    /// # Returns
    ///
    /// Extrapolated Greeks with improved accuracy.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let coarse = tree_n.calculate_greeks(...)?;
    /// let fine = tree_2n.calculate_greeks(...)?;
    /// let improved = TreeGreeks::richardson_extrapolate(&coarse, &fine);
    /// ```
    ///
    /// # References
    ///
    /// - Broadie, M. & Detemple, J. (1996). "American Option Valuation: New Bounds,
    ///   Approximations, and a Comparison of Existing Methods." Review of Financial
    ///   Studies, 9(4), 1211-1250.
    #[must_use]
    pub fn richardson_extrapolate(coarse: &Self, fine: &Self) -> Self {
        Self {
            price: (4.0 * fine.price - coarse.price) / 3.0,
            delta: (4.0 * fine.delta - coarse.delta) / 3.0,
            gamma: (4.0 * fine.gamma - coarse.gamma) / 3.0,
            vega: (4.0 * fine.vega - coarse.vega) / 3.0,
            theta: (4.0 * fine.theta - coarse.theta) / 3.0,
            rho: (4.0 * fine.rho - coarse.rho) / 3.0,
        }
    }

    /// Apply Richardson extrapolation to a price value only.
    ///
    /// Useful when only the price is needed, not all Greeks.
    #[must_use]
    pub fn richardson_price(price_coarse: f64, price_fine: f64) -> f64 {
        (4.0 * price_fine - price_coarse) / 3.0
    }
}

/// Configuration for Greek bump sizes.
///
/// Provides control over finite-difference bump sizes used in Greek calculations.
/// Supports both fixed and adaptive bump sizing based on moneyness.
///
/// # Adaptive Bump Sizing
///
/// When `adaptive` is true, spot bumps are scaled based on moneyness:
/// - **Near ATM** (0.8 ≤ S/K ≤ 1.2): Use smaller bumps (0.5%) for accuracy
/// - **Deep ITM/OTM** (S/K < 0.8 or S/K > 1.2): Use larger bumps (2%) for stability
///
/// This improves Greek accuracy across the moneyness spectrum.
#[derive(Debug, Clone)]
pub struct GreeksBumpConfig {
    /// Base spot bump as fraction of spot (default: 0.01 = 1%)
    pub spot_bump_fraction: f64,
    /// Volatility bump in absolute terms (default: 0.01 = 1% vol)
    pub vol_bump_absolute: f64,
    /// Interest rate bump in absolute terms (default: 0.0001 = 1bp)
    pub rate_bump_absolute: f64,
    /// Time bump in years (default: 1/365.25 = 1 day)
    pub time_bump_years: f64,
    /// Enable adaptive spot bump sizing based on moneyness
    pub adaptive: bool,
}

impl Default for GreeksBumpConfig {
    fn default() -> Self {
        Self {
            spot_bump_fraction: 0.01,      // 1% of spot
            vol_bump_absolute: 0.01,       // 1% vol (absolute, e.g., 20% → 21%)
            rate_bump_absolute: 0.0001,    // 1bp
            time_bump_years: 1.0 / 365.25, // 1 day
            adaptive: false,
        }
    }
}

impl GreeksBumpConfig {
    /// Create config with adaptive bump sizing enabled.
    pub fn adaptive() -> Self {
        Self {
            adaptive: true,
            ..Default::default()
        }
    }

    /// Create config with custom spot bump fraction.
    pub fn with_spot_bump(mut self, fraction: f64) -> Self {
        self.spot_bump_fraction = fraction;
        self
    }

    /// Calculate the actual spot bump size, optionally adapting to moneyness.
    ///
    /// # Arguments
    /// * `spot` - Current spot price
    /// * `strike` - Option strike (if available, for moneyness calculation)
    ///
    /// # Returns
    /// Absolute bump size in spot units
    #[inline]
    pub fn spot_bump(&self, spot: f64, strike: Option<f64>) -> f64 {
        if self.adaptive {
            if let Some(k) = strike {
                let moneyness = spot / k;

                // Adaptive scaling based on moneyness
                let scale = if (0.8..=1.2).contains(&moneyness) {
                    // Near ATM: use smaller bump for accuracy
                    0.5
                } else if (0.5..=1.5).contains(&moneyness) {
                    // Moderate ITM/OTM: standard bump
                    1.0
                } else {
                    // Deep ITM/OTM: larger bump for stability
                    2.0
                };

                return self.spot_bump_fraction * scale * spot;
            }
        }

        // Default: simple percentage of spot
        self.spot_bump_fraction * spot
    }
}

/// Tree branching type for evolution
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeBranching {
    /// Two-way branching (up/down)
    Binomial,
    /// Three-way branching (up/middle/down)
    Trinomial,
}

/// Generic tree parameters for state variable evolution
#[derive(Debug, Clone)]
pub struct TreeParameters {
    /// Number of time steps
    pub steps: usize,
    /// Time step size
    pub dt: f64,
    /// Tree branching type
    pub branching: TreeBranching,
    /// Evolution parameters for each state variable
    pub evolution_params: HashMap<&'static str, EvolutionParams>,
}

/// Parameters controlling how a state variable evolves in the tree
#[derive(Debug, Clone)]
pub struct EvolutionParams {
    /// Volatility for this factor
    pub volatility: f64,
    /// Drift rate (e.g., r-q for equity)
    pub drift: f64,
    /// Up factor
    pub up_factor: f64,
    /// Down factor
    pub down_factor: f64,
    /// Middle factor (for trinomial)
    pub middle_factor: Option<f64>,
    /// Probability of up move
    pub prob_up: f64,
    /// Probability of down move
    pub prob_down: f64,
    /// Probability of middle move (for trinomial)
    pub prob_middle: Option<f64>,
}

impl EvolutionParams {
    /// Create evolution parameters for a single equity factor (CRR model)
    pub fn equity_crr(volatility: f64, risk_free_rate: f64, dividend_yield: f64, dt: f64) -> Self {
        let u = (volatility * dt.sqrt()).exp();
        let d = 1.0 / u;
        let drift = risk_free_rate - dividend_yield;
        let p = ((drift * dt).exp() - d) / (u - d);

        // Debug assertions for probability bounds
        debug_assert!(
            (0.0..=1.0).contains(&p),
            "CRR probability p={} out of bounds [0,1]. Check parameters: vol={}, r={}, q={}, dt={}",
            p,
            volatility,
            risk_free_rate,
            dividend_yield,
            dt
        );
        debug_assert!(u > 0.0, "Up factor must be positive: u={}", u);
        debug_assert!(d > 0.0, "Down factor must be positive: d={}", d);

        Self {
            volatility,
            drift,
            up_factor: u,
            down_factor: d,
            middle_factor: None,
            prob_up: p,
            prob_down: 1.0 - p,
            prob_middle: None,
        }
    }

    /// Create evolution parameters for trinomial tree
    pub fn equity_trinomial(
        volatility: f64,
        risk_free_rate: f64,
        dividend_yield: f64,
        dt: f64,
    ) -> Self {
        let u = (volatility * (2.0 * dt).sqrt()).exp();
        let d = 1.0 / u;
        let m = 1.0;

        let drift = risk_free_rate - dividend_yield;
        let sqrt_dt_half = (dt / 2.0).sqrt();
        let exp_drift_half = (drift * dt / 2.0).exp();

        let denominator = (volatility * sqrt_dt_half).exp() - (-volatility * sqrt_dt_half).exp();
        let p_u = ((exp_drift_half - (-volatility * sqrt_dt_half).exp()) / denominator).powi(2);
        let p_d = (((volatility * sqrt_dt_half).exp() - exp_drift_half) / denominator).powi(2);
        let p_m = 1.0 - p_u - p_d;

        // Debug assertions for probability bounds
        debug_assert!(
            p_u >= 0.0 && p_d >= 0.0 && p_m >= 0.0,
            "Trinomial probabilities must be non-negative: p_u={}, p_d={}, p_m={}",
            p_u,
            p_d,
            p_m
        );
        debug_assert!(
            (p_u + p_d + p_m - 1.0).abs() < 1e-10,
            "Trinomial probabilities must sum to 1: p_u + p_d + p_m = {}",
            p_u + p_d + p_m
        );

        Self {
            volatility,
            drift,
            up_factor: u,
            down_factor: d,
            middle_factor: Some(m),
            prob_up: p_u,
            prob_down: p_d,
            prob_middle: Some(p_m),
        }
    }
}

/// Barrier option configuration for discrete monitoring.
#[derive(Debug, Clone)]
pub enum BarrierStyle {
    /// Knock-out barrier: option becomes void upon breach (rebate may apply)
    KnockOut,
    /// Knock-in barrier: engine tracks barrier hit state for path-dependent pricing
    KnockIn,
}

/// Barrier specification for discrete barrier monitoring in tree pricing.
///
/// Defines barrier levels, rebate, and style for incorporating barrier
/// conditions into recombining tree valuation.
///
/// # Barrier Touch Convention
///
/// This implementation uses **non-strict inequality** for barrier observation:
/// - Up barrier: triggered when `spot >= up_level`
/// - Down barrier: triggered when `spot <= down_level`
///
/// This differs from QuantLib's default (strict inequality: `>` and `<`).
/// The non-strict convention is more conservative for knock-out options
/// (barrier is triggered at the exact level) and matches Bloomberg's behavior.
#[derive(Debug, Clone)]
pub struct BarrierSpec {
    /// Up barrier level (S >= up triggers a touch; non-strict inequality)
    pub up_level: Option<f64>,
    /// Down barrier level (S <= down triggers a touch; non-strict inequality)
    pub down_level: Option<f64>,
    /// Rebate amount paid on knock-out (or at expiry if knock-in never triggers)
    pub rebate: f64,
    /// Barrier style (engine only enforces KnockOut directly)
    pub style: BarrierStyle,
}

/// Custom state generator function type for flexible tree evolution.
///
/// Given a step index and node index, returns the state variable value at that node.
/// This allows for pre-calibrated trees (e.g., short-rate trees) to inject
/// custom state values instead of using multiplicative factors.
///
/// # Arguments
/// * `step` - Time step index (0 to N)
/// * `node` - Node index at this step
///
/// # Returns
/// * State variable value (e.g., interest rate, spot price)
pub type StateGenerator = Box<dyn Fn(usize, usize) -> f64>;

/// Shared recombining tree engine that performs backward induction given constant
/// per-step evolution parameters and a branching policy.
#[derive(Clone)]
pub struct RecombiningInputs<'a, V: TreeValuator> {
    /// Branching structure (binomial or trinomial)
    pub branching: TreeBranching,
    /// Number of time steps in the tree
    pub steps: usize,
    /// Initial state variable values at root node
    pub initial_vars: StateVariables,
    /// Time to maturity in years
    pub time_to_maturity: f64,
    /// Market data context for curve lookups
    pub market_context: &'a MarketContext,
    /// Payoff valuator implementing TreeValuator trait
    pub valuator: &'a V,
    /// Multiplicative factor for up move (e.g., exp(σ√dt))
    pub up_factor: f64,
    /// Multiplicative factor for down move (e.g., exp(-σ√dt))
    pub down_factor: f64,
    /// Multiplicative factor for middle move (trinomial only)
    pub middle_factor: Option<f64>,
    /// Risk-neutral probability of up move
    pub prob_up: f64,
    /// Risk-neutral probability of down move
    pub prob_down: f64,
    /// Risk-neutral probability of middle move (trinomial only)
    pub prob_middle: Option<f64>,
    /// Risk-free interest rate per annum (used for discounting if custom_rate_generator is None)
    pub interest_rate: f64,
    /// Optional barrier configuration (discrete monitoring per step)
    pub barrier: Option<BarrierSpec>,
    /// Optional custom state generator for primary state variable (overrides up/down factors)
    pub custom_state_generator: Option<&'a StateGenerator>,
    /// Optional custom rate generator for discounting (overrides interest_rate)
    pub custom_rate_generator: Option<&'a StateGenerator>,
}

/// Price an option using a recombining tree with backward induction.
///
/// Supports binomial and trinomial trees with optional barrier monitoring.
/// The tree is built forward, payoffs are evaluated at maturity, and expected
/// values are discounted backward to the root.
///
/// # Arguments
///
/// * `inputs` - Complete tree configuration including evolution parameters,
///   valuator, and optional barrier specification
///
/// # Returns
///
/// Present value of the option at time 0
pub fn price_recombining_tree<V: TreeValuator>(inputs: RecombiningInputs<'_, V>) -> Result<f64> {
    let dt = inputs.time_to_maturity / inputs.steps as f64;

    // Pre-compute constant discount factor when no custom rate generator
    let const_df = if inputs.custom_rate_generator.is_none() {
        Some((-inputs.interest_rate * dt).exp())
    } else {
        None
    };

    // Helper: compute discount factor at a given step/node
    let get_df = |step: usize, node: usize| -> f64 {
        if let Some(df) = const_df {
            df
        } else if let Some(rate_gen) = &inputs.custom_rate_generator {
            let r = rate_gen(step, node);
            (-r * dt).exp()
        } else {
            unreachable!()
        }
    };

    // Pre-compute ratio for incremental spot computation
    let ud_ratio = inputs.up_factor / inputs.down_factor;

    // Helper: compute state value (spot or rate) at a given step/node
    let get_state = |step: usize, node: usize, spot0: f64| -> f64 {
        if let Some(state_gen) = &inputs.custom_state_generator {
            state_gen(step, node)
        } else {
            // Default multiplicative evolution for binomial/trinomial
            match inputs.branching {
                TreeBranching::Binomial => {
                    // Node i at step n has i up moves and (n-i) down moves
                    let ups = node as i32;
                    let downs = step as i32 - node as i32;
                    spot0 * inputs.up_factor.powi(ups) * inputs.down_factor.powi(downs)
                }
                TreeBranching::Trinomial => {
                    // Trinomial tree: at step n, nodes j ∈ [0, 2n] with center at j=n
                    // j_centered = j - n ranges from -n to +n
                    // S(n,j) = S₀ * u^j_centered (since d = 1/u in standard setup)
                    //
                    // For generality (when d ≠ 1/u), we use:
                    // S(n,j) = S₀ * u^max(j_centered, 0) * d^max(-j_centered, 0)
                    let j_centered = node as i32 - step as i32;
                    if j_centered >= 0 {
                        spot0 * inputs.up_factor.powi(j_centered)
                    } else {
                        spot0 * inputs.down_factor.powi(-j_centered)
                    }
                }
            }
        }
    };

    // Hoist contains_key check: determine once which state key to use
    let uses_spot_key = inputs.initial_vars.contains_key(state_keys::SPOT);
    let state_key: &'static str = if uses_spot_key {
        state_keys::SPOT
    } else {
        state_keys::INTEREST_RATE
    };
    let has_barrier = inputs.barrier.is_some();
    // Pre-extract hazard rate from initial vars (constant across tree)
    let cached_hazard = inputs.initial_vars.get(state_keys::HAZARD_RATE).copied();

    // Helper: evaluate barrier touch at a given spot
    let barrier_touch = |spot: f64| -> (bool, bool, bool, f64) {
        if let Some(spec) = &inputs.barrier {
            let touched_up = spec.up_level.map(|lvl| spot >= lvl).unwrap_or(false);
            let touched_down = spec.down_level.map(|lvl| spot <= lvl).unwrap_or(false);
            let breached =
                matches!(spec.style, BarrierStyle::KnockOut) && (touched_up || touched_down);
            (touched_up, touched_down, breached, spec.rebate)
        } else {
            (false, false, false, 0.0)
        }
    };

    let barrier_is_knock_in = inputs
        .barrier
        .as_ref()
        .is_some_and(|spec| matches!(spec.style, BarrierStyle::KnockIn));

    match inputs.branching {
        TreeBranching::Binomial => {
            // Initialize terminal values
            let spot0 = *inputs
                .initial_vars
                .get(state_keys::SPOT)
                .or_else(|| inputs.initial_vars.get(state_keys::INTEREST_RATE))
                .ok_or_else(|| {
                    finstack_core::Error::internal(
                        "tree pricing requires initial SPOT or INTEREST_RATE state",
                    )
                })?;

            let mut node_vars = inputs.initial_vars.clone(); // Clone once outside loops

            if barrier_is_knock_in {
                let spec = inputs.barrier.as_ref().ok_or_else(|| {
                    finstack_core::Error::internal(
                        "knock-in tree pricing requires a barrier specification",
                    )
                })?;
                let num_barriers =
                    spec.up_level.is_some() as usize + spec.down_level.is_some() as usize;
                if num_barriers != 1 {
                    return Err(finstack_core::Error::Validation(
                        "Knock-in tree pricing requires exactly one barrier (up or down)".into(),
                    ));
                }

                let (barrier_level, barrier_type) = if let Some(up) = spec.up_level {
                    (up, BarrierType::UpAndIn)
                } else if let Some(down) = spec.down_level {
                    (down, BarrierType::DownAndIn)
                } else {
                    return Err(finstack_core::Error::internal(
                        "knock-in tree pricing requires exactly one configured barrier level",
                    ));
                };
                let hit_state = BarrierState {
                    barrier_hit: true,
                    barrier_level,
                    barrier_type,
                };

                let mut hit_values = Vec::with_capacity(inputs.steps + 1);
                let mut not_hit_values = Vec::with_capacity(inputs.steps + 1);

                // Initialize terminal values with hit/not-hit states
                for i in 0..=inputs.steps {
                    let time_t = inputs.time_to_maturity;
                    let terminal_spot = get_state(inputs.steps, i, spot0);

                    node_vars.insert(state_key, terminal_spot);

                    let (t_up, t_dn, _breached, rebate) = barrier_touch(terminal_spot);
                    let touched = t_up || t_dn;
                    node_vars.insert(state_keys::BARRIER_TOUCHED_UP, if t_up { 1.0 } else { 0.0 });
                    node_vars.insert(
                        state_keys::BARRIER_TOUCHED_DOWN,
                        if t_dn { 1.0 } else { 0.0 },
                    );

                    let terminal_state = NodeState::new_with_barrier(
                        inputs.steps,
                        time_t,
                        &node_vars,
                        inputs.market_context,
                        hit_state.clone(),
                    );
                    let payoff_hit = inputs.valuator.value_at_maturity(&terminal_state)?;
                    let payoff_not_hit = if touched { payoff_hit } else { rebate };

                    hit_values.push(payoff_hit);
                    not_hit_values.push(payoff_not_hit);
                }

                // Backward induction with path-dependent barrier state
                // Reuse scratch vectors to avoid per-step allocation
                let mut next_hit = Vec::with_capacity(inputs.steps + 1);
                let mut next_not_hit = Vec::with_capacity(inputs.steps + 1);
                for step in (0..inputs.steps).rev() {
                    next_hit.clear();
                    next_not_hit.clear();
                    for i in 0..=step {
                        let spot_t = get_state(step, i, spot0);
                        let time_t = step as f64 * dt;
                        let df_node = get_df(step, i);

                        node_vars.insert(state_key, spot_t);

                        let (t_up, t_dn, _breached, _rebate) = barrier_touch(spot_t);
                        let touched = t_up || t_dn;
                        node_vars
                            .insert(state_keys::BARRIER_TOUCHED_UP, if t_up { 1.0 } else { 0.0 });
                        node_vars.insert(
                            state_keys::BARRIER_TOUCHED_DOWN,
                            if t_dn { 1.0 } else { 0.0 },
                        );

                        let continuation_hit = df_node
                            * (inputs.prob_up * hit_values[i + 1]
                                + inputs.prob_down * hit_values[i]);
                        let node_state_hit = NodeState::new_with_barrier(
                            step,
                            time_t,
                            &node_vars,
                            inputs.market_context,
                            hit_state.clone(),
                        );
                        let value_hit =
                            inputs
                                .valuator
                                .value_at_node(&node_state_hit, continuation_hit, dt)?;

                        let value_not_hit = if touched {
                            value_hit
                        } else {
                            let spot_up = get_state(step + 1, i + 1, spot0);
                            let spot_down = get_state(step + 1, i, spot0);
                            let (up_t_up, up_t_dn, _up_breached, _up_rebate) =
                                barrier_touch(spot_up);
                            let (dn_t_up, dn_t_dn, _dn_breached, _dn_rebate) =
                                barrier_touch(spot_down);
                            let child_up_touched = up_t_up || up_t_dn;
                            let child_down_touched = dn_t_up || dn_t_dn;

                            let next_up = if child_up_touched {
                                hit_values[i + 1]
                            } else {
                                not_hit_values[i + 1]
                            };
                            let next_down = if child_down_touched {
                                hit_values[i]
                            } else {
                                not_hit_values[i]
                            };
                            df_node * (inputs.prob_up * next_up + inputs.prob_down * next_down)
                        };

                        next_hit.push(value_hit);
                        next_not_hit.push(value_not_hit);
                    }
                    std::mem::swap(&mut hit_values, &mut next_hit);
                    std::mem::swap(&mut not_hit_values, &mut next_not_hit);
                }

                return Ok(not_hit_values[0]);
            }

            let mut values = Vec::with_capacity(inputs.steps + 1);

            let use_incremental = inputs.custom_state_generator.is_none();

            // Initialize terminal values using custom state generator if provided
            if use_incremental {
                // Incremental spot computation: start from spot0 * d^N, multiply by u/d
                let mut terminal_spot = spot0 * inputs.down_factor.powi(inputs.steps as i32);
                for i in 0..=inputs.steps {
                    let time_t = inputs.time_to_maturity;

                    node_vars.insert(state_key, terminal_spot);
                    if has_barrier {
                        let (t_up, t_dn, breached, rebate) = barrier_touch(terminal_spot);
                        node_vars
                            .insert(state_keys::BARRIER_TOUCHED_UP, if t_up { 1.0 } else { 0.0 });
                        node_vars.insert(
                            state_keys::BARRIER_TOUCHED_DOWN,
                            if t_dn { 1.0 } else { 0.0 },
                        );
                        let (cached_spot, cached_rate) = if uses_spot_key {
                            (
                                Some(terminal_spot),
                                node_vars.get(state_keys::INTEREST_RATE).copied(),
                            )
                        } else {
                            (
                                node_vars.get(state_keys::SPOT).copied(),
                                Some(terminal_spot),
                            )
                        };
                        let terminal_state = NodeState::with_cached(
                            inputs.steps,
                            time_t,
                            &node_vars,
                            inputs.market_context,
                            CachedValues {
                                spot: cached_spot,
                                interest_rate: cached_rate,
                                hazard_rate: cached_hazard,
                                df: None,
                            },
                        );
                        values.push(if breached {
                            rebate
                        } else {
                            inputs.valuator.value_at_maturity(&terminal_state)?
                        });
                    } else {
                        let (cached_spot, cached_rate) = if uses_spot_key {
                            (
                                Some(terminal_spot),
                                node_vars.get(state_keys::INTEREST_RATE).copied(),
                            )
                        } else {
                            (
                                node_vars.get(state_keys::SPOT).copied(),
                                Some(terminal_spot),
                            )
                        };
                        let terminal_state = NodeState::with_cached(
                            inputs.steps,
                            time_t,
                            &node_vars,
                            inputs.market_context,
                            CachedValues {
                                spot: cached_spot,
                                interest_rate: cached_rate,
                                hazard_rate: cached_hazard,
                                df: None,
                            },
                        );
                        values.push(inputs.valuator.value_at_maturity(&terminal_state)?);
                    }
                    if i < inputs.steps {
                        terminal_spot *= ud_ratio;
                    }
                }
            } else {
                for i in 0..=inputs.steps {
                    let time_t = inputs.time_to_maturity;
                    let terminal_spot = get_state(inputs.steps, i, spot0);
                    node_vars.insert(state_key, terminal_spot);
                    if has_barrier {
                        let (t_up, t_dn, breached, rebate) = barrier_touch(terminal_spot);
                        node_vars
                            .insert(state_keys::BARRIER_TOUCHED_UP, if t_up { 1.0 } else { 0.0 });
                        node_vars.insert(
                            state_keys::BARRIER_TOUCHED_DOWN,
                            if t_dn { 1.0 } else { 0.0 },
                        );
                        let terminal_state =
                            NodeState::new(inputs.steps, time_t, &node_vars, inputs.market_context);
                        values.push(if breached {
                            rebate
                        } else {
                            inputs.valuator.value_at_maturity(&terminal_state)?
                        });
                    } else {
                        let terminal_state =
                            NodeState::new(inputs.steps, time_t, &node_vars, inputs.market_context);
                        values.push(inputs.valuator.value_at_maturity(&terminal_state)?);
                    }
                }
            }

            // Backward induction
            for step in (0..inputs.steps).rev() {
                let time_t = step as f64 * dt;

                if use_incremental {
                    // Incremental spot computation for this step
                    let mut spot_t = spot0 * inputs.down_factor.powi(step as i32);
                    for i in 0..=step {
                        let df_node = get_df(step, i);
                        let continuation = df_node
                            * (inputs.prob_up * values[i + 1] + inputs.prob_down * values[i]);

                        node_vars.insert(state_key, spot_t);

                        if has_barrier {
                            let (t_up, t_dn, breached, rebate) = barrier_touch(spot_t);
                            node_vars.insert(
                                state_keys::BARRIER_TOUCHED_UP,
                                if t_up { 1.0 } else { 0.0 },
                            );
                            node_vars.insert(
                                state_keys::BARRIER_TOUCHED_DOWN,
                                if t_dn { 1.0 } else { 0.0 },
                            );
                            let node_state =
                                NodeState::new(step, time_t, &node_vars, inputs.market_context);
                            values[i] = if breached {
                                rebate
                            } else {
                                inputs
                                    .valuator
                                    .value_at_node(&node_state, continuation, dt)?
                            };
                        } else {
                            let (cached_spot, cached_rate) = if uses_spot_key {
                                (
                                    Some(spot_t),
                                    node_vars.get(state_keys::INTEREST_RATE).copied(),
                                )
                            } else {
                                (node_vars.get(state_keys::SPOT).copied(), Some(spot_t))
                            };
                            let node_state = NodeState::with_cached(
                                step,
                                time_t,
                                &node_vars,
                                inputs.market_context,
                                CachedValues {
                                    spot: cached_spot,
                                    interest_rate: cached_rate,
                                    hazard_rate: cached_hazard,
                                    df: None,
                                },
                            );
                            values[i] =
                                inputs
                                    .valuator
                                    .value_at_node(&node_state, continuation, dt)?;
                        }
                        if i < step {
                            spot_t *= ud_ratio;
                        }
                    }
                } else {
                    for i in 0..=step {
                        let spot_t = get_state(step, i, spot0);
                        let (t_up, t_dn, breached, rebate) = barrier_touch(spot_t);
                        let df_node = get_df(step, i);
                        let continuation = df_node
                            * (inputs.prob_up * values[i + 1] + inputs.prob_down * values[i]);

                        node_vars.insert(state_key, spot_t);
                        if has_barrier {
                            node_vars.insert(
                                state_keys::BARRIER_TOUCHED_UP,
                                if t_up { 1.0 } else { 0.0 },
                            );
                            node_vars.insert(
                                state_keys::BARRIER_TOUCHED_DOWN,
                                if t_dn { 1.0 } else { 0.0 },
                            );
                        }
                        let node_state =
                            NodeState::new(step, time_t, &node_vars, inputs.market_context);
                        values[i] = if breached {
                            rebate
                        } else {
                            inputs
                                .valuator
                                .value_at_node(&node_state, continuation, dt)?
                        };
                    }
                }
                values.pop();
            }

            Ok(values[0])
        }
        TreeBranching::Trinomial => {
            let spot0 = *inputs
                .initial_vars
                .get(state_keys::SPOT)
                .or_else(|| inputs.initial_vars.get(state_keys::INTEREST_RATE))
                .ok_or_else(|| {
                    finstack_core::Error::internal(
                        "tree pricing requires initial SPOT or INTEREST_RATE state",
                    )
                })?;

            let p_m = inputs.prob_middle.unwrap_or(0.0);

            let max_nodes = 2 * inputs.steps + 1;
            let mut node_vars = inputs.initial_vars.clone(); // Clone once

            if barrier_is_knock_in {
                let spec = inputs.barrier.as_ref().ok_or_else(|| {
                    finstack_core::Error::internal(
                        "knock-in tree pricing requires a barrier specification",
                    )
                })?;
                let num_barriers =
                    spec.up_level.is_some() as usize + spec.down_level.is_some() as usize;
                if num_barriers != 1 {
                    return Err(finstack_core::Error::Validation(
                        "Knock-in tree pricing requires exactly one barrier (up or down)".into(),
                    ));
                }

                let (barrier_level, barrier_type) = if let Some(up) = spec.up_level {
                    (up, BarrierType::UpAndIn)
                } else if let Some(down) = spec.down_level {
                    (down, BarrierType::DownAndIn)
                } else {
                    return Err(finstack_core::Error::internal(
                        "knock-in tree pricing requires exactly one configured barrier level",
                    ));
                };
                let hit_state = BarrierState {
                    barrier_hit: true,
                    barrier_level,
                    barrier_type,
                };

                let mut hit_values = vec![vec![0.0; max_nodes]; inputs.steps + 1];
                let mut not_hit_values = vec![vec![0.0; max_nodes]; inputs.steps + 1];

                // Terminal values
                for j in 0..max_nodes {
                    if j <= 2 * inputs.steps {
                        let spot_t = get_state(inputs.steps, j, spot0);
                        let time_t = inputs.time_to_maturity;

                        node_vars.insert(state_key, spot_t);

                        let (t_up, t_dn, _breached, rebate) = barrier_touch(spot_t);
                        let touched = t_up || t_dn;
                        node_vars
                            .insert(state_keys::BARRIER_TOUCHED_UP, if t_up { 1.0 } else { 0.0 });
                        node_vars.insert(
                            state_keys::BARRIER_TOUCHED_DOWN,
                            if t_dn { 1.0 } else { 0.0 },
                        );

                        let terminal_state = NodeState::new_with_barrier(
                            inputs.steps,
                            time_t,
                            &node_vars,
                            inputs.market_context,
                            hit_state.clone(),
                        );
                        let payoff_hit = inputs.valuator.value_at_maturity(&terminal_state)?;
                        let payoff_not_hit = if touched { payoff_hit } else { rebate };

                        hit_values[inputs.steps][j] = payoff_hit;
                        not_hit_values[inputs.steps][j] = payoff_not_hit;
                    }
                }

                // Backward induction
                for step in (0..inputs.steps).rev() {
                    let nodes_at_step = 2 * step + 1;
                    for j in 0..nodes_at_step {
                        let spot_t = get_state(step, j, spot0);
                        let time_t = step as f64 * dt;
                        let df_node = get_df(step, j);

                        // Child indices: up=j+2, mid=j+1, down=j
                        let up_idx = j + 2;
                        let mid_idx = j + 1;
                        let down_idx = j;

                        node_vars.insert(state_key, spot_t);

                        let (t_up, t_dn, _breached, _rebate) = barrier_touch(spot_t);
                        let touched = t_up || t_dn;
                        node_vars
                            .insert(state_keys::BARRIER_TOUCHED_UP, if t_up { 1.0 } else { 0.0 });
                        node_vars.insert(
                            state_keys::BARRIER_TOUCHED_DOWN,
                            if t_dn { 1.0 } else { 0.0 },
                        );

                        let continuation_hit = df_node
                            * (inputs.prob_up * hit_values[step + 1][up_idx]
                                + p_m * hit_values[step + 1][mid_idx]
                                + inputs.prob_down * hit_values[step + 1][down_idx]);
                        let node_state_hit = NodeState::new_with_barrier(
                            step,
                            time_t,
                            &node_vars,
                            inputs.market_context,
                            hit_state.clone(),
                        );
                        let value_hit =
                            inputs
                                .valuator
                                .value_at_node(&node_state_hit, continuation_hit, dt)?;

                        let value_not_hit = if touched {
                            value_hit
                        } else {
                            let spot_up = get_state(step + 1, up_idx, spot0);
                            let spot_mid = get_state(step + 1, mid_idx, spot0);
                            let spot_down = get_state(step + 1, down_idx, spot0);

                            let (up_t_up, up_t_dn, _up_breached, _up_rebate) =
                                barrier_touch(spot_up);
                            let (mid_t_up, mid_t_dn, _mid_breached, _mid_rebate) =
                                barrier_touch(spot_mid);
                            let (dn_t_up, dn_t_dn, _dn_breached, _dn_rebate) =
                                barrier_touch(spot_down);
                            let child_up_touched = up_t_up || up_t_dn;
                            let child_mid_touched = mid_t_up || mid_t_dn;
                            let child_down_touched = dn_t_up || dn_t_dn;

                            let next_up = if child_up_touched {
                                hit_values[step + 1][up_idx]
                            } else {
                                not_hit_values[step + 1][up_idx]
                            };
                            let next_mid = if child_mid_touched {
                                hit_values[step + 1][mid_idx]
                            } else {
                                not_hit_values[step + 1][mid_idx]
                            };
                            let next_down = if child_down_touched {
                                hit_values[step + 1][down_idx]
                            } else {
                                not_hit_values[step + 1][down_idx]
                            };

                            df_node
                                * (inputs.prob_up * next_up
                                    + p_m * next_mid
                                    + inputs.prob_down * next_down)
                        };

                        hit_values[step][j] = value_hit;
                        not_hit_values[step][j] = value_not_hit;
                    }
                }

                return Ok(not_hit_values[0][0]);
            }

            let mut values = vec![vec![0.0; max_nodes]; inputs.steps + 1];

            // Terminal values
            for (j, terminal_value) in values[inputs.steps].iter_mut().enumerate().take(max_nodes) {
                if j <= 2 * inputs.steps {
                    let spot_t = get_state(inputs.steps, j, spot0);
                    let time_t = inputs.time_to_maturity;

                    // Update state variable (SPOT for equity, INTEREST_RATE for rates)
                    if inputs.initial_vars.contains_key(state_keys::SPOT) {
                        node_vars.insert(state_keys::SPOT, spot_t);
                    } else {
                        node_vars.insert(state_keys::INTEREST_RATE, spot_t);
                    }

                    let (t_up, t_dn, breached, rebate) = barrier_touch(spot_t);
                    node_vars.insert(state_keys::BARRIER_TOUCHED_UP, if t_up { 1.0 } else { 0.0 });
                    node_vars.insert(
                        state_keys::BARRIER_TOUCHED_DOWN,
                        if t_dn { 1.0 } else { 0.0 },
                    );

                    let terminal_state =
                        NodeState::new(inputs.steps, time_t, &node_vars, inputs.market_context);
                    let payoff = if breached {
                        rebate
                    } else {
                        inputs.valuator.value_at_maturity(&terminal_state)?
                    };
                    *terminal_value = payoff;
                }
            }

            // Backward induction
            for step in (0..inputs.steps).rev() {
                let nodes_at_step = 2 * step + 1;
                for j in 0..nodes_at_step {
                    let spot_t = get_state(step, j, spot0);
                    let time_t = step as f64 * dt;

                    // Child indices: up=j+2, mid=j+1, down=j
                    let up_idx = j + 2;
                    let mid_idx = j + 1;
                    let down_idx = j;

                    // Discounted expected continuation value using custom discount if provided
                    let df_node = get_df(step, j);
                    let continuation = df_node
                        * (inputs.prob_up * values[step + 1][up_idx]
                            + p_m * values[step + 1][mid_idx]
                            + inputs.prob_down * values[step + 1][down_idx]);

                    // Update state variable (SPOT for equity, INTEREST_RATE for rates)
                    if inputs.initial_vars.contains_key(state_keys::SPOT) {
                        node_vars.insert(state_keys::SPOT, spot_t);
                    } else {
                        node_vars.insert(state_keys::INTEREST_RATE, spot_t);
                    }

                    let (t_up, t_dn, breached, rebate) = barrier_touch(spot_t);
                    node_vars.insert(state_keys::BARRIER_TOUCHED_UP, if t_up { 1.0 } else { 0.0 });
                    node_vars.insert(
                        state_keys::BARRIER_TOUCHED_DOWN,
                        if t_dn { 1.0 } else { 0.0 },
                    );
                    let node_state =
                        NodeState::new(step, time_t, &node_vars, inputs.market_context);
                    values[step][j] = if breached {
                        rebate
                    } else {
                        inputs
                            .valuator
                            .value_at_node(&node_state, continuation, dt)?
                    };
                }
            }

            Ok(values[0][0])
        }
    }
}

/// Helper function to create initial state variables for single-factor equity model
pub fn single_factor_equity_state(
    spot: f64,
    risk_free_rate: f64,
    dividend_yield: f64,
    volatility: f64,
) -> StateVariables {
    let mut vars = HashMap::default();
    vars.insert(state_keys::SPOT, spot);
    vars.insert(state_keys::INTEREST_RATE, risk_free_rate);
    vars.insert(state_keys::DIVIDEND_YIELD, dividend_yield);
    vars.insert(state_keys::VOLATILITY, volatility);
    vars
}

/// Helper function to create initial state variables for two-factor model
pub fn two_factor_equity_rates_state(
    spot: f64,
    risk_free_rate: f64,
    dividend_yield: f64,
    equity_volatility: f64,
    rate_volatility: f64,
) -> StateVariables {
    let mut vars = HashMap::default();
    vars.insert(state_keys::SPOT, spot);
    vars.insert(state_keys::INTEREST_RATE, risk_free_rate);
    vars.insert(state_keys::DIVIDEND_YIELD, dividend_yield);
    vars.insert(state_keys::VOLATILITY, equity_volatility);
    vars.insert(state_keys::RATE_VOLATILITY, rate_volatility);
    vars
}

#[cfg(test)]
mod tests {
    use super::*;

    struct QuadraticValuator;

    impl TreeValuator for QuadraticValuator {
        fn value_at_maturity(&self, state: &NodeState) -> Result<f64> {
            self.value_at_node(state, 0.0, 0.0)
        }

        fn value_at_node(
            &self,
            state: &NodeState,
            _continuation_value: f64,
            _dt: f64,
        ) -> Result<f64> {
            let spot = state.spot().unwrap_or(0.0);
            let vol = state.get_var_or(state_keys::VOLATILITY, 0.0);
            let rate = state.interest_rate().unwrap_or(0.0);
            Ok(spot * spot + 10.0 * vol + 100.0 * rate + 5.0 * state.time)
        }
    }

    struct MockTreeModel;

    impl TreeModel for MockTreeModel {
        fn price<V: TreeValuator>(
            &self,
            initial_vars: StateVariables,
            time_to_maturity: f64,
            market_context: &MarketContext,
            valuator: &V,
        ) -> Result<f64> {
            let state = NodeState::new(0, time_to_maturity, &initial_vars, market_context);
            valuator.value_at_node(&state, 0.0, 0.0)
        }
    }

    fn sample_state_variables() -> StateVariables {
        let mut vars = HashMap::default();
        vars.insert(state_keys::SPOT, 100.0);
        vars.insert(state_keys::INTEREST_RATE, 0.03);
        vars.insert(state_keys::HAZARD_RATE, 0.02);
        vars.insert(state_keys::DF, 0.95);
        vars.insert(state_keys::CREDIT_SPREAD, 0.015);
        vars
    }

    #[test]
    fn node_state_caches_common_fields_and_barrier_variants() {
        let market = MarketContext::new();
        let vars = sample_state_variables();

        let state = NodeState::new(2, 0.5, &vars, &market);
        assert_eq!(state.step, 2);
        assert_eq!(state.time, 0.5);
        assert_eq!(state.spot(), Some(100.0));
        assert_eq!(state.interest_rate(), Some(0.03));
        assert_eq!(state.hazard_rate(), Some(0.02));
        assert_eq!(state.discount_factor(), Some(0.95));
        assert!(state.barrier_state.is_none());

        let barrier_state = BarrierState {
            barrier_hit: false,
            barrier_level: 105.0,
            barrier_type: BarrierType::UpAndOut,
        };
        let with_barrier = NodeState::new_with_barrier(3, 0.75, &vars, &market, barrier_state);
        assert_eq!(with_barrier.step, 3);
        assert_eq!(with_barrier.time, 0.75);
        assert!(with_barrier.barrier_state.is_some());
        assert_eq!(with_barrier.spot(), Some(100.0));
    }

    #[test]
    fn node_state_accessors_and_barrier_helpers_cover_thresholds() {
        let market = MarketContext::new();
        let mut vars = sample_state_variables();
        vars.insert(state_keys::BARRIER_TOUCHED_UP, 0.5);
        vars.insert(state_keys::BARRIER_TOUCHED_DOWN, 0.9);

        let mut up_and_out = NodeState::new_with_barrier(
            0,
            0.0,
            &vars,
            &market,
            BarrierState {
                barrier_hit: false,
                barrier_level: 110.0,
                barrier_type: BarrierType::UpAndOut,
            },
        );
        assert_eq!(up_and_out.get_var(state_keys::CREDIT_SPREAD), Some(0.015));
        assert_eq!(up_and_out.get_var_or("missing", 42.0), 42.0);
        assert_eq!(up_and_out.credit_spread(), Some(0.015));
        assert!(
            !up_and_out.barrier_touched_up(),
            "0.5 should not count as touched"
        );
        assert!(up_and_out.barrier_touched_down());
        assert!(!up_and_out.is_barrier_hit());
        assert!(!up_and_out.is_knocked_out());
        up_and_out.update_barrier_state(110.0);
        assert!(up_and_out.is_barrier_hit());
        assert!(up_and_out.is_knocked_out());
        up_and_out.update_barrier_state(90.0);
        assert!(up_and_out.is_barrier_hit(), "barrier hit should latch");

        let mut down_and_in = NodeState::new_with_barrier(
            0,
            0.0,
            &vars,
            &market,
            BarrierState {
                barrier_hit: false,
                barrier_level: 95.0,
                barrier_type: BarrierType::DownAndIn,
            },
        );
        assert!(!down_and_in.is_knocked_in());
        down_and_in.update_barrier_state(95.0);
        assert!(down_and_in.is_barrier_hit());
        assert!(down_and_in.is_knocked_in());

        let no_barrier = NodeState::new(0, 0.0, &vars, &market);
        assert!(!no_barrier.is_knocked_out());
        assert!(no_barrier.is_knocked_in());
    }

    #[test]
    fn tree_greeks_richardson_extrapolation_matches_formula() {
        let coarse = TreeGreeks {
            price: 10.0,
            delta: 1.0,
            gamma: 2.0,
            vega: 3.0,
            theta: 4.0,
            rho: 5.0,
        };
        let fine = TreeGreeks {
            price: 13.0,
            delta: 1.75,
            gamma: 2.5,
            vega: 3.5,
            theta: 4.5,
            rho: 5.5,
        };

        let extrapolated = TreeGreeks::richardson_extrapolate(&coarse, &fine);
        assert!((extrapolated.price - 14.0).abs() < 1e-12);
        assert!((extrapolated.delta - 2.0).abs() < 1e-12);
        assert!((extrapolated.gamma - (8.0 / 3.0)).abs() < 1e-12);
        assert!((extrapolated.vega - (11.0 / 3.0)).abs() < 1e-12);
        assert!((extrapolated.theta - (14.0 / 3.0)).abs() < 1e-12);
        assert!((extrapolated.rho - (17.0 / 3.0)).abs() < 1e-12);
        assert!((TreeGreeks::richardson_price(10.0, 13.0) - 14.0).abs() < 1e-12);
    }

    #[test]
    fn greeks_bump_config_defaults_adaptive_ranges_and_custom_spot_bump() {
        let default_cfg = GreeksBumpConfig::default();
        assert!((default_cfg.spot_bump_fraction - 0.01).abs() < 1e-12);
        assert!((default_cfg.vol_bump_absolute - 0.01).abs() < 1e-12);
        assert!((default_cfg.rate_bump_absolute - 0.0001).abs() < 1e-12);
        assert!((default_cfg.time_bump_years - 1.0 / 365.25).abs() < 1e-12);
        assert!(!default_cfg.adaptive);
        assert!((default_cfg.spot_bump(100.0, Some(100.0)) - 1.0).abs() < 1e-12);

        let adaptive = GreeksBumpConfig::adaptive().with_spot_bump(0.02);
        assert!(adaptive.adaptive);
        assert!((adaptive.spot_bump(100.0, Some(100.0)) - 1.0).abs() < 1e-12);
        assert!((adaptive.spot_bump(100.0, Some(160.0)) - 2.0).abs() < 1e-12);
        assert!((adaptive.spot_bump(100.0, Some(260.0)) - 4.0).abs() < 1e-12);
        assert!((adaptive.spot_bump(100.0, None) - 2.0).abs() < 1e-12);
    }

    #[test]
    fn default_tree_model_calculate_greeks_uses_central_differences_and_restores_vars() {
        let model = MockTreeModel;
        let valuator = QuadraticValuator;
        let market = MarketContext::new();
        let initial_vars = single_factor_equity_state(100.0, 0.03, 0.01, 0.20);

        let greeks =
            model.calculate_greeks(initial_vars.clone(), 1.0, &market, &valuator, Some(0.01));
        assert!(greeks.is_ok(), "greeks calculation should succeed");

        let expected_price = 100.0_f64.powi(2) + 10.0 * 0.20 + 100.0 * 0.03 + 5.0;
        if let Ok(greeks) = greeks {
            assert!((greeks.price - expected_price).abs() < 1e-12);
            assert!((greeks.delta - 200.0).abs() < 1e-12);
            assert!((greeks.gamma - 2.0).abs() < 1e-12);
            assert!((greeks.vega - 0.1).abs() < 1e-12);
            assert!((greeks.rho - 0.01).abs() < 1e-12);
            assert!((greeks.theta + 5.0).abs() < 1e-10);
        }

        let restored_price = model.price(initial_vars, 1.0, &market, &valuator);
        assert!(restored_price.is_ok(), "restored price should succeed");
        if let Ok(restored_price) = restored_price {
            assert!((restored_price - expected_price).abs() < 1e-12);
        }
    }

    #[test]
    fn default_tree_model_greeks_clamp_vol_down_and_skip_theta_near_expiry() {
        let model = MockTreeModel;
        let valuator = QuadraticValuator;
        let market = MarketContext::new();
        let initial_vars = single_factor_equity_state(100.0, 0.03, 0.01, 0.005);

        let greeks = model.calculate_greeks(initial_vars, 0.001, &market, &valuator, Some(0.01));
        assert!(greeks.is_ok(), "greeks calculation should succeed");

        let expected_vega = (10.0 * (0.005 + 0.01) - 10.0 * 1e-6) / 2.0;
        if let Ok(greeks) = greeks {
            assert!((greeks.vega - expected_vega).abs() < 1e-12);
            assert_eq!(
                greeks.theta, 0.0,
                "theta should be skipped for near-expiry inputs"
            );
        }
    }

    #[test]
    fn state_helper_builders_populate_expected_keys() {
        let single = single_factor_equity_state(100.0, 0.03, 0.01, 0.20);
        assert_eq!(single.get(state_keys::SPOT), Some(&100.0));
        assert_eq!(single.get(state_keys::INTEREST_RATE), Some(&0.03));
        assert_eq!(single.get(state_keys::DIVIDEND_YIELD), Some(&0.01));
        assert_eq!(single.get(state_keys::VOLATILITY), Some(&0.20));
        assert_eq!(single.get(state_keys::RATE_VOLATILITY), None);

        let two_factor = two_factor_equity_rates_state(100.0, 0.03, 0.01, 0.20, 0.015);
        assert_eq!(two_factor.get(state_keys::SPOT), Some(&100.0));
        assert_eq!(two_factor.get(state_keys::INTEREST_RATE), Some(&0.03));
        assert_eq!(two_factor.get(state_keys::DIVIDEND_YIELD), Some(&0.01));
        assert_eq!(two_factor.get(state_keys::VOLATILITY), Some(&0.20));
        assert_eq!(two_factor.get(state_keys::RATE_VOLATILITY), Some(&0.015));
    }

    #[test]
    fn evolution_params_builders_satisfy_basic_probability_invariants() {
        let crr = EvolutionParams::equity_crr(0.2, 0.05, 0.01, 0.25);
        assert!(crr.up_factor > 1.0);
        assert!(crr.down_factor < 1.0);
        assert!((crr.up_factor * crr.down_factor - 1.0).abs() < 1e-12);
        assert!(crr.prob_up >= 0.0 && crr.prob_up <= 1.0);
        assert!(crr.prob_down >= 0.0 && crr.prob_down <= 1.0);
        assert!((crr.prob_up + crr.prob_down - 1.0).abs() < 1e-12);

        let trinomial = EvolutionParams::equity_trinomial(0.2, 0.05, 0.01, 0.25);
        assert!(trinomial.up_factor > 1.0);
        assert!(trinomial.down_factor < 1.0);
        assert_eq!(trinomial.middle_factor, Some(1.0));
        assert!(trinomial.prob_up >= 0.0);
        assert!(trinomial.prob_down >= 0.0);
        assert!(
            trinomial.prob_middle.is_some(),
            "middle probability should exist"
        );
        if let Some(p_mid) = trinomial.prob_middle {
            assert!(p_mid >= 0.0);
            assert!((trinomial.prob_up + trinomial.prob_down + p_mid - 1.0).abs() < 1e-10);
        }
    }
}
