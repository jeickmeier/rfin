//! Trinomial tree models for option pricing.
//!
//! Implements trinomial tree methods with three-way branching (up/middle/down)
//! for improved convergence and flexibility in modeling complex instruments.

use finstack_core::market_data::context::MarketContext;
use finstack_core::{Error, Result, F};

use super::tree_framework::{
    price_recombining_tree, state_keys, RecombiningInputs, StateVariables, TreeBranching,
    TreeGreeks, TreeModel, TreeValuator,
};

#[cfg(test)]
use super::tree_framework::NodeState;

/// Trinomial tree types
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrinomialTreeType {
    /// Standard trinomial model with moment matching
    Standard,
    /// Boyle trinomial model (simplified probabilities)
    Boyle,
}

/// Trinomial tree for option pricing
#[derive(Clone, Debug)]
pub struct TrinomialTree {
    /// Number of time steps
    pub steps: usize,
    /// Tree type
    pub tree_type: TrinomialTreeType,
}

impl TrinomialTree {
    /// Create new trinomial tree with specified steps and type
    pub fn new(steps: usize, tree_type: TrinomialTreeType) -> Self {
        Self {
            steps,
            tree_type,
        }
    }

    /// Create a standard trinomial tree
    pub fn standard(steps: usize) -> Self {
        Self::new(steps, TrinomialTreeType::Standard)
    }

    /// Create a Boyle trinomial tree
    pub fn boyle(steps: usize) -> Self {
        Self::new(steps, TrinomialTreeType::Boyle)
    }

    /// Calculate trinomial tree parameters
    fn calculate_parameters(&self, r: F, sigma: F, t: F, q: F) -> Result<(F, F, F, F, F, F)> {
        if t <= 0.0 || sigma <= 0.0 {
            return Err(Error::Internal);
        }

        let dt = t / self.steps as f64;
        let drift = r - q;

        let (u, d, m, p_u, p_d, p_m) = match self.tree_type {
            TrinomialTreeType::Standard => {
                // Standard trinomial model with moment matching
                let u = (sigma * (2.0 * dt).sqrt()).exp();
                let d = 1.0 / u;
                let m = 1.0;

                // Calculate probabilities to match first two moments
                let sqrt_dt_half = (dt / 2.0).sqrt();
                let exp_drift_half = (drift * dt / 2.0).exp();

                let exp_vol_up = (sigma * sqrt_dt_half).exp();
                let exp_vol_down = (-sigma * sqrt_dt_half).exp();
                let denominator = exp_vol_up - exp_vol_down;

                let p_u = ((exp_drift_half - exp_vol_down) / denominator).powi(2);
                let p_d = ((exp_vol_up - exp_drift_half) / denominator).powi(2);
                let p_m = 1.0 - p_u - p_d;

                // Validate probabilities
                if p_u < 0.0 || p_d < 0.0 || p_m < 0.0 {
                    return Err(Error::Internal);
                }

                (u, d, m, p_u, p_d, p_m)
            }
            TrinomialTreeType::Boyle => {
                // Simplified Boyle model
                let lambda = (sigma * sigma * dt + drift * drift * dt * dt).sqrt();
                let u = lambda.exp();
                let d = (-lambda).exp();
                let m = 1.0;

                let p_u = 0.5
                    * ((sigma * sigma * dt + drift * drift * dt * dt) / (lambda * lambda)
                        + drift * dt / lambda);
                let p_d = 0.5
                    * ((sigma * sigma * dt + drift * drift * dt * dt) / (lambda * lambda)
                        - drift * dt / lambda);
                let p_m = 1.0 - p_u - p_d;

                // Validate probabilities
                if p_u < 0.0 || p_d < 0.0 || p_m < 0.0 {
                    return Err(Error::Internal);
                }

                (u, d, m, p_u, p_d, p_m)
            }
        };

        Ok((u, d, m, p_u, p_d, p_m))
    }

    /// Generic pricing engine for arbitrary instruments
    #[inline(never)] // Prevent inlining to reduce coverage metadata conflicts
    pub fn price_generic<V: TreeValuator>(
        &self,
        initial_vars: StateVariables,
        time_to_maturity: F,
        market_context: &MarketContext,
        valuator: &V,
    ) -> Result<F> {
        // Extract required parameters from state variables
        let r = *initial_vars
            .get(state_keys::INTEREST_RATE)
            .ok_or(Error::Internal)?;
        let q = initial_vars
            .get(state_keys::DIVIDEND_YIELD)
            .copied()
            .unwrap_or(0.0);
        let sigma = *initial_vars
            .get(state_keys::VOLATILITY)
            .ok_or(Error::Internal)?;

        let (u, d, m, p_u, p_d, p_m) = self.calculate_parameters(r, sigma, time_to_maturity, q)?;

        price_recombining_tree(RecombiningInputs {
            branching: TreeBranching::Trinomial,
            steps: self.steps,
            initial_vars,
            time_to_maturity,
            market_context,
            valuator,
            up_factor: u,
            down_factor: d,
            middle_factor: Some(m),
            prob_up: p_u,
            prob_down: p_d,
            prob_middle: Some(p_m),
            interest_rate: r,
        })
    }
}

/// Implementation of TreeModel trait for TrinomialTree
impl TreeModel for TrinomialTree {
    fn price<V: TreeValuator>(
        &self,
        initial_vars: StateVariables,
        time_to_maturity: F,
        market_context: &MarketContext,
        valuator: &V,
    ) -> Result<F> {
        self.price_generic(initial_vars, time_to_maturity, market_context, valuator)
    }

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

        let mut greeks = TreeGreeks {
            price: base_price,
            delta: 0.0,
            gamma: 0.0,
            vega: 0.0,
            theta: 0.0,
            rho: 0.0,
        };

        // Calculate Delta and Gamma (spot sensitivity)
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

#[cfg(test)]
mod tests {
    use super::super::tree_framework::single_factor_equity_state;
    use super::*;

    // Simple test valuator that returns intrinsic value of a call option
    struct TestCallValuator {
        strike: F,
    }

    impl TreeValuator for TestCallValuator {
        fn value_at_maturity(&self, state: &NodeState) -> Result<F> {
            let spot = state.spot().unwrap_or(0.0);
            Ok((spot - self.strike).max(0.0))
        }

        fn value_at_node(&self, _state: &NodeState, continuation_value: F) -> Result<F> {
            // European-style: just return continuation value
            Ok(continuation_value)
        }
    }

    #[test]
    fn test_trinomial_tree_basic_functionality() {
        let tree = TrinomialTree::standard(50);
        let market_context = MarketContext::new();

        let initial_vars = single_factor_equity_state(100.0, 0.05, 0.0, 0.20);
        let valuator = TestCallValuator { strike: 100.0 };

        let price = tree.price(initial_vars, 1.0, &market_context, &valuator);
        assert!(price.is_ok());
        let price = price.unwrap();

        // Should be positive for ATM call
        assert!(price > 0.0);
        // Should be reasonable value (close to Black-Scholes ~10.45)
        assert!((price - 10.45).abs() < 2.0);
    }

    #[test]
    fn test_trinomial_vs_binomial_convergence() {
        use super::super::binomial_tree::BinomialTree;

        let binomial = BinomialTree::crr(100);
        let trinomial = TrinomialTree::standard(100);
        let market_context = MarketContext::new();

        let initial_vars = single_factor_equity_state(100.0, 0.05, 0.0, 0.20);
        let valuator = TestCallValuator { strike: 100.0 };

        let bin_price = binomial
            .price(initial_vars.clone(), 1.0, &market_context, &valuator)
            .unwrap();
        let tri_price = trinomial
            .price(initial_vars, 1.0, &market_context, &valuator)
            .unwrap();

        // Should converge to similar values
        assert!((bin_price - tri_price).abs() < 0.5);
    }
}
