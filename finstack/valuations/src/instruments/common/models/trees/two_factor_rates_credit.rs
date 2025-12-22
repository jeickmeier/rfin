//! Two-factor binomial tree: short rate + credit hazard (intensity).
//!
//! Models the joint evolution of the risk-free short rate and the credit hazard
//! rate using correlated binomial moves. Suitable for pricing credit-sensitive
//! instruments (e.g., risky bonds) where both discounting and default intensity
//! evolve over time.

use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::HazardCurve;
use finstack_core::{Error, Result};

use super::state_keys;
use super::tree_framework::{NodeState, StateVariables, TreeModel, TreeValuator};

/// Configuration for rates + credit two-factor tree.
#[derive(Clone, Debug)]
pub struct RatesCreditConfig {
    /// Number of time steps
    pub steps: usize,
    /// Short-rate volatility (annualized)
    pub rate_vol: f64,
    /// Credit hazard volatility (annualized)
    pub hazard_vol: f64,
    /// Base short rate used for drift (optional; used for stability when missing from vars)
    pub base_rate: f64,
    /// Base hazard used when missing from vars
    pub base_hazard: f64,
    /// Instantaneous correlation between rate and hazard shocks
    pub correlation: f64,
    /// Mean reversion speed for short rate (0.0 = no reversion)
    pub rate_mean_reversion: f64,
    /// Mean reversion speed for hazard rate (0.0 = no reversion)
    pub hazard_mean_reversion: f64,
}

impl Default for RatesCreditConfig {
    fn default() -> Self {
        Self {
            steps: 100,
            rate_vol: 0.01,
            hazard_vol: 0.20,
            base_rate: 0.02,
            base_hazard: 0.01,
            correlation: 0.0,
            rate_mean_reversion: 0.0,
            hazard_mean_reversion: 0.0,
        }
    }
}

/// Two-factor correlated binomial tree (short rate + hazard rate).
#[derive(Clone, Debug)]
pub struct RatesCreditTree {
    /// Rates-credit tree configuration
    pub config: RatesCreditConfig,
}

impl RatesCreditTree {
    /// Create a new rates-credit tree with the given configuration
    pub fn new(config: RatesCreditConfig) -> Self {
        Self { config }
    }

    #[inline]
    fn joint_probabilities(&self, p_r: f64, p_h: f64) -> (f64, f64, f64, f64) {
        // Correlated Bernoulli coupling
        let var_r = p_r * (1.0 - p_r);
        let var_h = p_h * (1.0 - p_h);
        let cov = self.config.correlation * (var_r * var_h).sqrt();

        let mut p_uu = (p_r * p_h + cov).clamp(0.0, 1.0);
        let mut p_ud = (p_r * (1.0 - p_h) - cov).clamp(0.0, 1.0);
        let mut p_du = ((1.0 - p_r) * p_h - cov).clamp(0.0, 1.0);
        let mut p_dd = ((1.0 - p_r) * (1.0 - p_h) + cov).clamp(0.0, 1.0);

        let sum = p_uu + p_ud + p_du + p_dd;
        if sum > 0.0 {
            p_uu /= sum;
            p_ud /= sum;
            p_du /= sum;
            p_dd /= sum;
        } else {
            // fallback to independent
            p_uu = p_r * p_h;
            p_ud = p_r * (1.0 - p_h);
            p_du = (1.0 - p_r) * p_h;
            p_dd = (1.0 - p_r) * (1.0 - p_h);
        }
        (p_uu, p_ud, p_du, p_dd)
    }

    /// Calibration hook: align base hazard to a provided hazard curve.
    ///
    /// Uses the first knot lambda as the base hazard and adopts the curve's
    /// recovery rate (returned to the caller for use in valuator logic).
    pub fn align_hazard_from_curve(&mut self, curve: &HazardCurve) -> f64 {
        if let Some((_, lambda0)) = curve.knot_points().next() {
            self.config.base_hazard = lambda0.max(0.0);
        }
        curve.recovery_rate()
    }
}

impl TreeModel for RatesCreditTree {
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

        // Initial state
        let r0 = initial_vars
            .get(state_keys::INTEREST_RATE)
            .copied()
            .unwrap_or(self.config.base_rate);
        let h0 = initial_vars
            .get(state_keys::HAZARD_RATE)
            .copied()
            .unwrap_or(self.config.base_hazard);

        // Multiplicative symmetric moves and equal probabilities (0.5) per factor
        let u_r = (self.config.rate_vol * dt.sqrt()).exp();
        let d_r = 1.0 / u_r;

        let u_h = (self.config.hazard_vol * dt.sqrt()).exp();
        let d_h = 1.0 / u_h;
        // Terminal payoff grid values[i][j] for i: rate ups, j: hazard ups
        let mut values: Vec<Vec<f64>> = vec![vec![0.0; steps + 1]; steps + 1];
        let mut vars = initial_vars.clone();

        for (i, row) in values.iter_mut().enumerate() {
            let r_t = r0 * u_r.powi(i as i32) * d_r.powi((steps - i) as i32);
            for (j, cell) in row.iter_mut().enumerate() {
                let h_t = h0 * u_h.powi(j as i32) * d_h.powi((steps - j) as i32);

                vars.insert(state_keys::INTEREST_RATE, r_t.max(1e-8));
                vars.insert(state_keys::HAZARD_RATE, h_t.max(0.0));
                vars.insert("step", steps as f64);
                vars.insert("node_i", i as f64);
                vars.insert("node_j", j as f64);
                vars.insert("time", time_to_maturity);

                let state = NodeState::new(steps, time_to_maturity, &vars, market_context);
                *cell = valuator.value_at_maturity(&state)?;
            }
        }

        // Backward induction
        for k in (0..steps).rev() {
            let mut new_values: Vec<Vec<f64>> = vec![vec![0.0; k + 1]; k + 1];
            for i in 0..=k {
                let r_t = r0 * u_r.powi(i as i32) * d_r.powi((k - i) as i32);

                // Calculate rate transition probability with mean reversion
                // drift ~ -a * (ln(r) - ln(base))
                // p_r = 0.5 + drift * sqrt(dt) / (2 * vol)
                let p_r = if self.config.rate_mean_reversion > 0.0 {
                    let log_r = r_t.ln();
                    let log_base = self.config.base_rate.ln();
                    let drift = -self.config.rate_mean_reversion * (log_r - log_base);
                    (0.5 + drift * dt.sqrt() / (2.0 * self.config.rate_vol)).clamp(0.0, 1.0)
                } else {
                    0.5
                };

                for j in 0..=k {
                    let h_t = h0 * u_h.powi(j as i32) * d_h.powi((k - j) as i32);

                    // Calculate hazard transition probability with mean reversion
                    let p_h = if self.config.hazard_mean_reversion > 0.0 {
                        let log_h = h_t.ln();
                        let log_base = self.config.base_hazard.ln();
                        let drift = -self.config.hazard_mean_reversion * (log_h - log_base);
                        (0.5 + drift * dt.sqrt() / (2.0 * self.config.hazard_vol)).clamp(0.0, 1.0)
                    } else {
                        0.5
                    };

                    // Calculate joint probabilities for this node
                    let (p_uu, p_ud, p_du, p_dd) = self.joint_probabilities(p_r, p_h);

                    // Continuation from four children at step k+1
                    let v_uu = values[i + 1][j + 1];
                    let v_ud = values[i + 1][j];
                    let v_du = values[i][j + 1];
                    let v_dd = values[i][j];

                    // Risky discounting with short rate; hazard is left to valuator
                    let df = (-r_t.max(1e-8) * dt).exp();
                    let cont = df * (p_uu * v_uu + p_ud * v_ud + p_du * v_du + p_dd * v_dd);

                    vars.insert(state_keys::INTEREST_RATE, r_t.max(1e-8));
                    vars.insert(state_keys::HAZARD_RATE, h_t.max(0.0));
                    vars.insert("df", df);
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
    use finstack_core::market_data::context::MarketContext;

    struct DummyValuator;

    impl TreeValuator for DummyValuator {
        fn value_at_maturity(&self, _state: &NodeState) -> Result<f64> {
            Ok(1.0)
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
    fn rates_credit_prices_positive() {
        let tree = RatesCreditTree::new(RatesCreditConfig::default());
        let ctx = MarketContext::new();
        let mut vars = StateVariables::default();
        vars.insert(state_keys::INTEREST_RATE, 0.03);
        vars.insert(state_keys::HAZARD_RATE, 0.01);
        let val = DummyValuator;
        let price = tree.price(vars, 1.0, &ctx, &val).expect("should succeed");
        assert!(price.is_finite() && price > 0.0);
    }
}
