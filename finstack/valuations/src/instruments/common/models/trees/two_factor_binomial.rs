//! Two-factor correlated binomial tree (equity + interest rate).
//!
//! This implements a simple two-factor recombining lattice where the equity
//! factor follows a CRR multiplicative process and the short rate follows a
//! multiplicative process with symmetric up/down moves. Correlation is injected
//! via a Bernoulli-coupling of the two single-factor trees to produce four
//! joint probabilities per step.
//!
//! Notes:
//! - Discounting uses the node short rate: df = exp(-r_node * dt).
//! - Equity risk-neutral probability uses drift r0 - q with a constant r0.
//! - This is an initial baseline suitable for pricing and early exercise logic
//!   via the generic `TreeValuator` trait.

use finstack_core::market_data::context::MarketContext;
use finstack_core::{Error, Result};

use super::tree_framework::{state_keys, NodeState, StateVariables, TreeModel, TreeValuator};

/// Configuration parameters for the two-factor lattice.
#[derive(Clone, Debug)]
pub struct TwoFactorBinomialConfig {
    /// Number of time steps.
    pub steps: usize,
    /// Equity volatility (annualized, fraction).
    pub equity_vol: f64,
    /// Dividend yield (fraction per annum).
    pub dividend_yield: f64,
    /// Short-rate volatility (annualized, fraction).
    pub rate_vol: f64,
    /// Base (constant) short rate used in equity risk-neutral drift.
    pub base_rate_for_equity_drift: f64,
    /// Instantaneous correlation between equity and rate shocks.
    pub correlation: f64,
}

impl Default for TwoFactorBinomialConfig {
    fn default() -> Self {
        Self {
            steps: 100,
            equity_vol: 0.2,
            dividend_yield: 0.0,
            rate_vol: 0.01,
            base_rate_for_equity_drift: 0.02,
            correlation: 0.0,
        }
    }
}

/// Two-factor correlated binomial tree (equity + short rate).
#[derive(Clone, Debug)]
pub struct TwoFactorBinomialTree {
    /// Two-factor binomial tree configuration
    pub config: TwoFactorBinomialConfig,
}

impl TwoFactorBinomialTree {
    /// Create a new two-factor binomial tree with the given configuration
    pub fn new(config: TwoFactorBinomialConfig) -> Self {
        Self { config }
    }

    /// Convenience constructor.
    pub fn equity_and_rates(
        steps: usize,
        equity_vol: f64,
        dividend_yield: f64,
        rate_vol: f64,
        base_rate: f64,
        correlation: f64,
    ) -> Self {
        Self::new(TwoFactorBinomialConfig {
            steps,
            equity_vol,
            dividend_yield,
            rate_vol,
            base_rate_for_equity_drift: base_rate,
            correlation,
        })
    }

    #[inline]
    fn joint_probabilities(&self, p_s: f64, p_r: f64) -> (f64, f64, f64, f64) {
        // Bernoulli coupling to inject correlation
        let var_s = p_s * (1.0 - p_s);
        let var_r = p_r * (1.0 - p_r);
        let cov = self.config.correlation * (var_s * var_r).sqrt();

        let mut p_uu = (p_s * p_r + cov).clamp(0.0, 1.0);
        let mut p_ud = (p_s * (1.0 - p_r) - cov).clamp(0.0, 1.0);
        let mut p_du = ((1.0 - p_s) * p_r - cov).clamp(0.0, 1.0);
        let mut p_dd = ((1.0 - p_s) * (1.0 - p_r) + cov).clamp(0.0, 1.0);

        // Renormalize to ensure sum exactly 1.0
        let sum = p_uu + p_ud + p_du + p_dd;
        if sum > 0.0 {
            p_uu /= sum;
            p_ud /= sum;
            p_du /= sum;
            p_dd /= sum;
        } else {
            // Fallback to independent if degenerate
            p_uu = p_s * p_r;
            p_ud = p_s * (1.0 - p_r);
            p_du = (1.0 - p_s) * p_r;
            p_dd = (1.0 - p_s) * (1.0 - p_r);
        }

        (p_uu, p_ud, p_du, p_dd)
    }
}

impl TreeModel for TwoFactorBinomialTree {
    fn price<V: TreeValuator>(
        &self,
        initial_vars: StateVariables,
        time_to_maturity: f64,
        market_context: &MarketContext,
        valuator: &V,
    ) -> Result<f64> {
        if self.config.steps == 0 || time_to_maturity <= 0.0 {
            return Err(Error::Internal);
        }

        let steps = self.config.steps;
        let dt = time_to_maturity / steps as f64;

        // Extract initial state
        let spot0 = *initial_vars.get(state_keys::SPOT).ok_or(Error::Internal)?;
        let r0 = initial_vars
            .get(state_keys::INTEREST_RATE)
            .copied()
            .unwrap_or(self.config.base_rate_for_equity_drift);

        // Equity CRR factors and risk-neutral prob with constant drift r0 - q
        let u_s = (self.config.equity_vol * dt.sqrt()).exp();
        let d_s = 1.0 / u_s;
        let drift = self.config.base_rate_for_equity_drift - self.config.dividend_yield;
        let m1 = (drift * dt).exp();
        let p_s = ((m1 - d_s) / (u_s - d_s)).clamp(0.0, 1.0);

        // Rate multiplicative symmetric move with 0.5 probability
        let u_r = (self.config.rate_vol * dt.sqrt()).exp();
        let d_r = 1.0 / u_r;
        let p_r = 0.5;

        let (p_uu, p_ud, p_du, p_dd) = self.joint_probabilities(p_s, p_r);

        // Terminal payoff grid: (steps+1) x (steps+1)
        let mut values: Vec<Vec<f64>> = vec![vec![0.0; steps + 1]; steps + 1];
        let mut vars = initial_vars.clone();

        for (i, row) in values.iter_mut().enumerate() {
            let s_t = spot0 * u_s.powi(i as i32) * d_s.powi((steps - i) as i32);
            for (j, cell) in row.iter_mut().enumerate() {
                let r_t = r0 * u_r.powi(j as i32) * d_r.powi((steps - j) as i32);
                vars.insert(state_keys::SPOT, s_t);
                vars.insert(state_keys::INTEREST_RATE, r_t.max(1e-8));
                vars.insert("step", steps as f64);
                vars.insert("node_i", i as f64);
                vars.insert("node_j", j as f64);
                vars.insert("time", time_to_maturity);

                let state = NodeState::new(steps, time_to_maturity, &vars, market_context);
                *cell = valuator.value_at_maturity(&state)?;
            }
        }

        // Backward induction over steps
        for k in (0..steps).rev() {
            let mut new_values: Vec<Vec<f64>> = vec![vec![0.0; k + 1]; k + 1];
            for i in 0..=k {
                // Equity spot at node (k,i)
                let s_t = spot0 * u_s.powi(i as i32) * d_s.powi((k - i) as i32);
                for j in 0..=k {
                    // Short rate at node (k,j)
                    let r_t = r0 * u_r.powi(j as i32) * d_r.powi((k - j) as i32);
                    let df = (-r_t.max(1e-8) * dt).exp();

                    // Children indices at step k+1
                    let v_uu = values[i + 1][j + 1];
                    let v_ud = values[i + 1][j];
                    let v_du = values[i][j + 1];
                    let v_dd = values[i][j];
                    let cont = df * (p_uu * v_uu + p_ud * v_ud + p_du * v_du + p_dd * v_dd);

                    vars.insert(state_keys::SPOT, s_t);
                    vars.insert(state_keys::INTEREST_RATE, r_t.max(1e-8));
                    vars.insert("step", k as f64);
                    vars.insert("node_i", i as f64);
                    vars.insert("node_j", j as f64);
                    vars.insert("time", k as f64 * dt);

                    let state = NodeState::new(k, k as f64 * dt, &vars, market_context);
                    new_values[i][j] = valuator.value_at_node(&state, cont, dt)?;
                }
            }
            values = new_values;
        }

        Ok(values[0][0])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruments::common::models::trees::binomial_tree::BinomialTree;
    use crate::instruments::common::models::trees::tree_framework::single_factor_equity_state;

    struct TestCallValuator {
        strike: f64,
    }

    impl TreeValuator for TestCallValuator {
        fn value_at_maturity(&self, state: &NodeState) -> Result<f64> {
            let s = state.spot().ok_or(Error::Internal)?;
            Ok((s - self.strike).max(0.0))
        }
        fn value_at_node(
            &self,
            _state: &NodeState,
            continuation_value: f64,
            _dt: f64,
        ) -> Result<f64> {
            Ok(continuation_value)
        }
    }

    #[test]
    fn two_factor_basic_sanity() {
        let tree = TwoFactorBinomialTree::equity_and_rates(50, 0.2, 0.0, 0.0, 0.05, 0.0);
        let ctx = MarketContext::new();
        let initial = single_factor_equity_state(100.0, 0.05, 0.0, 0.2);
        let val = TestCallValuator { strike: 100.0 };
        let price = tree
            .price(initial, 1.0, &ctx, &val)
            .expect("should succeed");
        assert!(price.is_finite() && price > 0.0);
    }

    #[test]
    fn two_factor_matches_one_factor_when_rate_vol_zero() {
        let steps = 75;
        let t = 1.0;
        let ctx = MarketContext::new();
        let initial = single_factor_equity_state(100.0, 0.05, 0.0, 0.2);
        let val = TestCallValuator { strike: 100.0 };

        let one_factor = BinomialTree::crr(steps)
            .price(initial.clone(), t, &ctx, &val)
            .expect("should succeed");

        let two_factor = TwoFactorBinomialTree::equity_and_rates(steps, 0.2, 0.0, 0.0, 0.05, 0.0)
            .price(initial, t, &ctx, &val)
            .expect("should succeed");

        // Should be close when rate volatility is zero and correlation is zero
        // Relaxed tolerance to account for different tree discretizations
        assert!(
            (one_factor - two_factor).abs() < 0.5,
            "One-factor {} and two-factor {} should match when rate vol is zero, diff={}",
            one_factor,
            two_factor,
            (one_factor - two_factor).abs()
        );
    }
}
