//! Generic tree-based pricing framework for financial instruments.
//!
//! This module provides a flexible lattice pricing engine that can accommodate
//! various tree types (binomial, trinomial) and multiple state variables
//! (equity + rates, equity + credit spread, etc.) without requiring code changes
//! to the core pricing logic.

use finstack_core::market_data::context::MarketContext;
use finstack_core::{Result, F};
use std::collections::HashMap;

/// Standard state variable keys for consistency
pub mod state_keys {
    /// Underlying asset price (equity)
    pub const SPOT: &str = "spot";
    /// Risk-free interest rate
    pub const INTEREST_RATE: &str = "interest_rate";
    /// Credit spread
    pub const CREDIT_SPREAD: &str = "credit_spread";
    /// Dividend yield
    pub const DIVIDEND_YIELD: &str = "dividend_yield";
    /// Volatility
    pub const VOLATILITY: &str = "volatility";
}

/// Map of state variables for a tree node
pub type StateVariables = HashMap<&'static str, F>;

/// Complete state information for a node in the pricing tree
#[derive(Clone)]
pub struct NodeState<'a> {
    /// Time step index (0 to N)
    pub step: usize,
    /// Time in years from valuation date
    pub time: F,
    /// Map of all state variables at this node
    pub vars: StateVariables,
    /// Access to market context for additional data
    pub market_context: &'a MarketContext,
}

impl<'a> NodeState<'a> {
    /// Create a new node state
    pub fn new(
        step: usize,
        time: F,
        vars: StateVariables,
        market_context: &'a MarketContext,
    ) -> Self {
        Self {
            step,
            time,
            vars,
            market_context,
        }
    }

    /// Get a state variable by key
    pub fn get_var(&self, key: &str) -> Option<F> {
        self.vars.get(key).copied()
    }

    /// Get a state variable by key with a default value
    pub fn get_var_or(&self, key: &str, default: F) -> F {
        self.vars.get(key).copied().unwrap_or(default)
    }

    /// Get spot price (convenience method)
    pub fn spot(&self) -> Option<F> {
        self.get_var(state_keys::SPOT)
    }

    /// Get interest rate (convenience method)
    pub fn interest_rate(&self) -> Option<F> {
        self.get_var(state_keys::INTEREST_RATE)
    }

    /// Get credit spread (convenience method)
    pub fn credit_spread(&self) -> Option<F> {
        self.get_var(state_keys::CREDIT_SPREAD)
    }
}

/// Trait for instrument-specific valuation logic on a tree
pub trait TreeValuator {
    /// Calculate the instrument's value at a terminal node (maturity)
    fn value_at_maturity(&self, state: &NodeState) -> Result<F>;

    /// Calculate the instrument's value at an intermediate node
    ///
    /// This method implements the core decision logic (e.g., hold vs. exercise)
    /// and receives the discounted expected continuation value from child nodes.
    fn value_at_node(&self, state: &NodeState, continuation_value: F) -> Result<F>;
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
    fn price<V: TreeValuator>(
        &self,
        initial_vars: StateVariables,
        time_to_maturity: F,
        market_context: &MarketContext,
        valuator: &V,
    ) -> Result<F>;

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
        time_to_maturity: F,
        market_context: &MarketContext,
        valuator: &V,
        bump_size: Option<F>,
    ) -> Result<TreeGreeks> {
        let bump = bump_size.unwrap_or(0.01);

        // Base price
        let base_price = self.price(
            initial_vars.clone(),
            time_to_maturity,
            market_context,
            valuator,
        )?;

        // Calculate Delta (spot sensitivity)
        let mut greeks = TreeGreeks {
            price: base_price,
            delta: 0.0,
            gamma: 0.0,
            vega: 0.0,
            theta: 0.0,
            rho: 0.0,
        };

        if let Some(&spot) = initial_vars.get(state_keys::SPOT) {
            let h = bump * spot;

            // Spot up
            let mut vars_up = initial_vars.clone();
            vars_up.insert(state_keys::SPOT, spot + h);
            let price_up = self.price(vars_up, time_to_maturity, market_context, valuator)?;

            // Spot down
            let mut vars_down = initial_vars.clone();
            vars_down.insert(state_keys::SPOT, spot - h);
            let price_down = self.price(vars_down, time_to_maturity, market_context, valuator)?;

            greeks.delta = (price_up - price_down) / (2.0 * h);
            greeks.gamma = (price_up - 2.0 * base_price + price_down) / (h * h);
        }

        // Calculate Vega (volatility sensitivity)
        if let Some(&vol) = initial_vars.get(state_keys::VOLATILITY) {
            let h = 0.01; // 1% vol bump

            let mut vars_vol_up = initial_vars.clone();
            vars_vol_up.insert(state_keys::VOLATILITY, vol + h);
            let price_vol_up =
                self.price(vars_vol_up, time_to_maturity, market_context, valuator)?;

            greeks.vega = price_vol_up - base_price;
        }

        // Calculate Rho (rate sensitivity)
        if let Some(&rate) = initial_vars.get(state_keys::INTEREST_RATE) {
            let h = 0.0001; // 1bp rate bump

            let mut vars_rate_up = initial_vars.clone();
            vars_rate_up.insert(state_keys::INTEREST_RATE, rate + h);
            let price_rate_up =
                self.price(vars_rate_up, time_to_maturity, market_context, valuator)?;

            greeks.rho = price_rate_up - base_price;
        }

        // Calculate Theta (time decay) - use 1 day bump
        let dt = 1.0 / 365.25;
        if time_to_maturity > dt {
            let price_tomorrow = self.price(
                initial_vars,
                time_to_maturity - dt,
                market_context,
                valuator,
            )?;
            greeks.theta = -(base_price - price_tomorrow) / dt;
        }

        Ok(greeks)
    }
}

/// Greeks calculated from tree models
#[derive(Clone, Debug)]
pub struct TreeGreeks {
    /// Instrument price
    pub price: F,
    /// Delta (spot sensitivity)
    pub delta: F,
    /// Gamma (curvature)
    pub gamma: F,
    /// Vega (volatility sensitivity)
    pub vega: F,
    /// Theta (time decay)
    pub theta: F,
    /// Rho (interest rate sensitivity)
    pub rho: F,
}

/// Tree branching type for evolution
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TreeBranching {
    /// Two-way branching (up/down)
    Binomial,
    /// Three-way branching (up/middle/down)
    Trinomial,
}

/// Generic tree parameters for state variable evolution
#[derive(Clone, Debug)]
pub struct TreeParameters {
    /// Number of time steps
    pub steps: usize,
    /// Time step size
    pub dt: F,
    /// Tree branching type
    pub branching: TreeBranching,
    /// Evolution parameters for each state variable
    pub evolution_params: HashMap<&'static str, EvolutionParams>,
}

/// Parameters controlling how a state variable evolves in the tree
#[derive(Clone, Debug)]
pub struct EvolutionParams {
    /// Volatility for this factor
    pub volatility: F,
    /// Drift rate (e.g., r-q for equity)
    pub drift: F,
    /// Up factor
    pub up_factor: F,
    /// Down factor  
    pub down_factor: F,
    /// Middle factor (for trinomial)
    pub middle_factor: Option<F>,
    /// Probability of up move
    pub prob_up: F,
    /// Probability of down move
    pub prob_down: F,
    /// Probability of middle move (for trinomial)
    pub prob_middle: Option<F>,
}

impl EvolutionParams {
    /// Create evolution parameters for a single equity factor (CRR model)
    pub fn equity_crr(volatility: F, risk_free_rate: F, dividend_yield: F, dt: F) -> Self {
        let u = (volatility * dt.sqrt()).exp();
        let d = 1.0 / u;
        let drift = risk_free_rate - dividend_yield;
        let p = ((drift * dt).exp() - d) / (u - d);

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
    pub fn equity_trinomial(volatility: F, risk_free_rate: F, dividend_yield: F, dt: F) -> Self {
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

    /// Create evolution parameters for interest rate factor (Vasicek-style)
    pub fn interest_rate(mean_reversion: F, long_term_rate: F, volatility: F, dt: F) -> Self {
        // Simplified Vasicek evolution for demonstration
        let drift = mean_reversion * long_term_rate * dt;
        let vol_factor = volatility * dt.sqrt();

        Self {
            volatility,
            drift,
            up_factor: 1.0 + vol_factor,
            down_factor: 1.0 - vol_factor,
            middle_factor: Some(1.0),
            prob_up: 0.5,
            prob_down: 0.5,
            prob_middle: Some(0.0),
        }
    }
}

/// Helper function to create initial state variables for single-factor equity model
pub fn single_factor_equity_state(
    spot: F,
    risk_free_rate: F,
    dividend_yield: F,
    volatility: F,
) -> StateVariables {
    let mut vars = HashMap::new();
    vars.insert(state_keys::SPOT, spot);
    vars.insert(state_keys::INTEREST_RATE, risk_free_rate);
    vars.insert(state_keys::DIVIDEND_YIELD, dividend_yield);
    vars.insert(state_keys::VOLATILITY, volatility);
    vars
}

/// Helper function to create initial state variables for two-factor model
pub fn two_factor_equity_rates_state(
    spot: F,
    risk_free_rate: F,
    dividend_yield: F,
    equity_volatility: F,
    rate_volatility: F,
) -> StateVariables {
    let mut vars = HashMap::new();
    vars.insert(state_keys::SPOT, spot);
    vars.insert(state_keys::INTEREST_RATE, risk_free_rate);
    vars.insert(state_keys::DIVIDEND_YIELD, dividend_yield);
    vars.insert(state_keys::VOLATILITY, equity_volatility);
    vars.insert("rate_volatility", rate_volatility);
    vars
}
