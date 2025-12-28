//! Trinomial tree models for option pricing.
//!
//! Implements trinomial tree methods with three-way branching (up/middle/down)
//! for improved convergence and flexibility in modeling complex instruments.

use finstack_core::market_data::context::MarketContext;
use finstack_core::{Error, Result};

use super::tree_framework::{
    price_recombining_tree, state_keys, RecombiningInputs, StateVariables, TreeBranching,
    TreeModel, TreeValuator,
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
        Self { steps, tree_type }
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
    fn calculate_parameters(
        &self,
        r: f64,
        sigma: f64,
        t: f64,
        q: f64,
    ) -> Result<(f64, f64, f64, f64, f64, f64)> {
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
        time_to_maturity: f64,
        market_context: &MarketContext,
        valuator: &V,
    ) -> Result<f64> {
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
            barrier: None,
            custom_state_generator: None,
            custom_rate_generator: None,
        })
    }
}

/// Implementation of TreeModel trait for TrinomialTree
impl TreeModel for TrinomialTree {
    fn price<V: TreeValuator>(
        &self,
        initial_vars: StateVariables,
        time_to_maturity: f64,
        market_context: &MarketContext,
        valuator: &V,
    ) -> Result<f64> {
        self.price_generic(initial_vars, time_to_maturity, market_context, valuator)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::super::tree_framework::single_factor_equity_state;
    use super::*;

    // Simple test valuator that returns intrinsic value of a call option
    struct TestCallValuator {
        strike: f64,
    }

    impl TreeValuator for TestCallValuator {
        fn value_at_maturity(&self, state: &NodeState) -> Result<f64> {
            let spot = state.spot().unwrap_or(0.0);
            Ok((spot - self.strike).max(0.0))
        }

        fn value_at_node(
            &self,
            _state: &NodeState,
            continuation_value: f64,
            _dt: f64,
        ) -> Result<f64> {
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
        let price = price.expect("should succeed");

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
            .expect("should succeed");
        let tri_price = trinomial
            .price(initial_vars, 1.0, &market_context, &valuator)
            .expect("should succeed");

        // Should converge to similar values (both approximating Black-Scholes)
        // Allow larger tolerance due to different tree structures
        assert!(
            (bin_price - tri_price).abs() < 1.0,
            "Binomial {} and trinomial {} should be within 1.0, diff={}",
            bin_price,
            tri_price,
            (bin_price - tri_price).abs()
        );
    }
}
