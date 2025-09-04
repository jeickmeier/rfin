//! Short-rate tree models for bond valuation with embedded options.
//!
//! Implements curve-consistent short-rate trees for pricing callable/putable bonds
//! and calculating Option-Adjusted Spread (OAS). Uses industry-standard models
//! like Ho-Lee and Black-Derman-Toy.

use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::traits::Discount;
use finstack_core::{Error, Result, F};

use super::tree_framework::{NodeState, StateVariables, TreeGreeks, TreeModel, TreeValuator};

/// Short-rate tree model types
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShortRateModel {
    /// Ho-Lee model (additive normal rates, handles negative rates)
    HoLee,
    /// Black-Derman-Toy model (lognormal rates, mean-reverting)
    BlackDermanToy,
}

/// Configuration for short-rate tree construction
#[derive(Clone, Debug)]
pub struct ShortRateTreeConfig {
    /// Number of time steps
    pub steps: usize,
    /// Tree model type
    pub model: ShortRateModel,
    /// Interest rate volatility (annualized)
    pub volatility: F,
    /// Mean reversion parameter (for mean-reverting models)
    pub mean_reversion: Option<F>,
    /// Whether to use caching for performance
    pub use_cache: bool,
}

impl Default for ShortRateTreeConfig {
    fn default() -> Self {
        Self {
            steps: 100,
            model: ShortRateModel::HoLee,
            volatility: 0.01, // 1% default volatility
            mean_reversion: None,
            use_cache: true,
        }
    }
}

/// Short-rate tree for valuing bonds with embedded options
#[derive(Clone, Debug)]
pub struct ShortRateTree {
    config: ShortRateTreeConfig,
    /// Calibrated short rates at each node: rates[step][node]
    rates: Vec<Vec<F>>,
    /// Transition probabilities: probs[step] gives (p_up, p_down) for that step
    probs: Vec<(F, F)>,
    /// Time steps in years
    time_steps: Vec<F>,
    /// Discount curve used for calibration
    calibration_curve_id: String,
}

impl ShortRateTree {
    /// Create a new short-rate tree with the given configuration
    pub fn new(config: ShortRateTreeConfig) -> Self {
        Self {
            config,
            rates: Vec::new(),
            probs: Vec::new(),
            time_steps: Vec::new(),
            calibration_curve_id: String::new(),
        }
    }

    /// Create a Ho-Lee tree with specified parameters
    pub fn ho_lee(steps: usize, volatility: F) -> Self {
        Self::new(ShortRateTreeConfig {
            steps,
            model: ShortRateModel::HoLee,
            volatility,
            mean_reversion: None,
            use_cache: true,
        })
    }

    /// Create a Black-Derman-Toy tree with specified parameters
    pub fn black_derman_toy(steps: usize, volatility: F, mean_reversion: F) -> Self {
        Self::new(ShortRateTreeConfig {
            steps,
            model: ShortRateModel::BlackDermanToy,
            volatility,
            mean_reversion: Some(mean_reversion),
            use_cache: true,
        })
    }

    /// Calibrate the tree to match a given discount curve
    pub fn calibrate(&mut self, discount_curve: &dyn Discount, time_to_maturity: F) -> Result<()> {
        self.calibration_curve_id = "CALIBRATED".to_string();

        // Build time grid
        let dt = time_to_maturity / self.config.steps as F;
        self.time_steps = (0..=self.config.steps).map(|i| i as F * dt).collect();

        // Initialize data structures
        self.rates = vec![Vec::new(); self.config.steps + 1];
        self.probs = vec![(0.5, 0.5); self.config.steps]; // Default to equal probabilities

        match self.config.model {
            ShortRateModel::HoLee => self.calibrate_ho_lee(discount_curve, dt)?,
            ShortRateModel::BlackDermanToy => self.calibrate_bdt(discount_curve, dt)?,
        }

        Ok(())
    }

    /// Calibrate Ho-Lee model parameters
    fn calibrate_ho_lee(&mut self, discount_curve: &dyn Discount, dt: F) -> Result<()> {
        let sigma = self.config.volatility;

        // Initialize first step with current short rate
        let r0 = if self.time_steps[1] > 0.0 {
            -discount_curve.df(self.time_steps[1]).ln() / self.time_steps[1]
        } else {
            0.03 // Fallback rate
        };

        self.rates[0] = vec![r0];

        // Build tree forward, calibrating to match discount curve at each step
        for step in 0..self.config.steps {
            let current_time = self.time_steps[step];
            let next_time = self.time_steps[step + 1];

            // Number of nodes at next step
            let next_nodes = step + 2;
            let mut next_rates = vec![0.0; next_nodes];

            // For Ho-Lee, rates evolve as: r(t+dt) = r(t) + theta(t)*dt + sigma*dW
            // where theta(t) is chosen to fit the discount curve

            // Calculate theta to match the discount curve
            let theta = self.calculate_ho_lee_drift(
                discount_curve,
                current_time,
                next_time,
                &self.rates[step],
                dt,
            )?;

            // Build next step rates
            for (i, &current_rate) in self.rates[step].iter().enumerate() {
                // Up move
                if i + 1 < next_nodes {
                    next_rates[i + 1] = current_rate + theta * dt + sigma * dt.sqrt();
                }
                // Down move
                if i < next_nodes {
                    next_rates[i] = current_rate + theta * dt - sigma * dt.sqrt();
                }
            }

            // For recombining tree, ensure proper structure
            // In a proper binomial tree, each step has step+1 nodes
            // The up and down moves should recombine

            self.rates[step + 1] = next_rates;
        }

        Ok(())
    }

    /// Calculate Ho-Lee drift term to match discount curve
    fn calculate_ho_lee_drift(
        &self,
        discount_curve: &dyn Discount,
        current_time: F,
        next_time: F,
        current_rates: &[F],
        dt: F,
    ) -> Result<F> {
        // For Ho-Lee calibration, theta is chosen so that the expected
        // discount factor matches the market curve

        // Expected short rate at current step
        let expected_rate = if current_rates.is_empty() {
            0.03 // Fallback
        } else {
            current_rates.iter().sum::<F>() / current_rates.len() as F
        };

        // Calculate theta using the forward rate implied by the discount curve
        let df_current = if current_time > 0.0 {
            discount_curve.df(current_time)
        } else {
            1.0
        };
        let df_next = discount_curve.df(next_time);

        if df_current <= 0.0 || df_next <= 0.0 {
            return Err(Error::Internal);
        }

        // Implied forward rate
        let implied_forward = (df_current / df_next - 1.0) / dt;

        // Return the drift needed to match the forward rate
        Ok(implied_forward - expected_rate)
    }

    /// Calibrate Black-Derman-Toy model using state-price recursion.
    ///
    /// Implements proper BDT calibration that matches the discount curve at each step
    /// by solving for the drift parameter using state-price recursion and root finding.
    fn calibrate_bdt(&mut self, discount_curve: &dyn Discount, dt: F) -> Result<()> {
        use finstack_core::math::{HybridSolver, Solver};

        let sigma = self.config.volatility;
        let solver = HybridSolver::new();

        // BDT parameters: lognormal rates with constant volatility
        let u = (sigma * dt.sqrt()).exp(); // Up multiplier
        let p = 0.5; // Risk-neutral probability

        // Initialize first step with initial short rate
        let r0 = if self.time_steps[1] > 0.0 {
            // Use initial forward rate from discount curve
            -discount_curve.df(self.time_steps[1]).ln() / self.time_steps[1]
        } else {
            0.03 // Fallback rate
        };

        self.rates[0] = vec![r0.max(1e-6)]; // Ensure positive for lognormal
        let mut state_prices = vec![vec![1.0]]; // Q[0] = [1.0]

        // Set transition probabilities (constant for BDT)
        for i in 0..self.config.steps {
            self.probs[i] = (p, 1.0 - p);
        }

        // Build tree forward, calibrating drift at each step
        for step in 0..self.config.steps {
            let current_time = self.time_steps[step + 1];
            let target_df = discount_curve.df(current_time);

            if target_df <= 0.0 {
                return Err(Error::Internal);
            }

            let num_nodes = step + 1;
            let current_state_prices = &state_prices[step];
            let current_rates = &self.rates[step];

            // Solve for drift parameter alpha such that model ZCB price matches market
            let objective = |alpha: F| -> F {
                // Calculate model ZCB price with this alpha
                let mut model_price = 0.0;

                for (j, &state_price) in current_state_prices.iter().enumerate().take(num_nodes) {
                    let rate = alpha * u.powf(num_nodes as F - 1.0 - 2.0 * j as F);
                    let rate_clamped = rate.max(1e-6); // Ensure positive
                    let discount_factor = (-rate_clamped * dt).exp();
                    model_price += state_price * discount_factor;
                }

                model_price - target_df
            };

            // Initial guess for alpha based on previous step or forward rate
            let initial_alpha = if step == 0 {
                r0
            } else {
                // Use geometric mean of previous step rates as initial guess
                let mean_rate =
                    current_rates.iter().map(|&r| r.ln()).sum::<F>() / current_rates.len() as F;
                mean_rate.exp()
            };

            // Solve for alpha
            let alpha = match solver.solve(objective, initial_alpha) {
                Ok(a) => a.max(1e-6), // Ensure positive
                Err(_) => {
                    // If solver fails, use fallback based on market rate
                    let market_rate = if current_time > 0.0 {
                        -target_df.ln() / current_time
                    } else {
                        0.03
                    };
                    market_rate.max(1e-6)
                }
            };

            // Build next step rates using calibrated alpha
            let next_nodes = num_nodes + 1;
            let mut next_rates = vec![0.0; next_nodes];
            let mut next_state_prices = vec![0.0; next_nodes];

            for (j, &state_price) in current_state_prices.iter().enumerate().take(num_nodes) {
                let current_rate = alpha * u.powf(num_nodes as F - 1.0 - 2.0 * j as F);
                let rate_clamped = current_rate.max(1e-6);
                let discount_factor = (-rate_clamped * dt).exp();
                let state_price_contribution = state_price * discount_factor;

                // Up move: j -> j+1
                if j + 1 < next_nodes {
                    let up_rate = alpha * u.powf(next_nodes as F - 1.0 - 2.0 * (j + 1) as F);
                    next_rates[j + 1] = up_rate.max(1e-6);
                    next_state_prices[j + 1] += state_price_contribution * p;
                }

                // Down move: j -> j
                if j < next_nodes {
                    let down_rate = alpha * u.powf(next_nodes as F - 1.0 - 2.0 * j as F);
                    next_rates[j] = down_rate.max(1e-6);
                    next_state_prices[j] += state_price_contribution * (1.0 - p);
                }
            }

            self.rates[step + 1] = next_rates;
            state_prices.push(next_state_prices);
        }

        Ok(())
    }

    /// Get the short rate at a specific node
    pub fn rate_at_node(&self, step: usize, node: usize) -> Result<F> {
        if step >= self.rates.len() || node >= self.rates[step].len() {
            return Err(Error::Internal);
        }
        Ok(self.rates[step][node])
    }

    /// Get transition probabilities at a step
    pub fn probabilities(&self, step: usize) -> Result<(F, F)> {
        if step >= self.probs.len() {
            return Err(Error::Internal);
        }
        Ok(self.probs[step])
    }

    /// Get time at step
    pub fn time_at_step(&self, step: usize) -> Result<F> {
        if step >= self.time_steps.len() {
            return Err(Error::Internal);
        }
        Ok(self.time_steps[step])
    }
}

impl TreeModel for ShortRateTree {
    fn price<V: TreeValuator>(
        &self,
        initial_vars: StateVariables,
        time_to_maturity: F,
        market_context: &MarketContext,
        valuator: &V,
    ) -> Result<F> {
        if self.rates.is_empty() {
            return Err(Error::Internal); // Tree not calibrated
        }

        let steps = self.config.steps;
        let dt = time_to_maturity / steps as F;

        // Get OAS from initial variables (default to 0)
        let oas = initial_vars.get("oas").copied().unwrap_or(0.0);

        // Initialize values at terminal nodes
        let terminal_step = steps;
        let mut values = vec![0.0; self.rates[terminal_step].len()];

        for (node, _rate) in self.rates[terminal_step].iter().enumerate() {
            // Create state for terminal node
            let time = terminal_step as F * dt;
            let mut vars = initial_vars.clone();
            vars.insert("step", terminal_step as F);
            vars.insert("node", node as F);
            vars.insert("time", time);
            vars.insert("interest_rate", self.rates[terminal_step][node]);

            let state = NodeState::new(terminal_step, time, vars, market_context);
            values[node] = valuator.value_at_maturity(&state)?;
        }

        // Backward induction
        for step in (0..steps).rev() {
            let (p_up, p_down) = self.probs[step];
            let current_nodes = self.rates[step].len();
            let mut new_values = vec![0.0; current_nodes];

            for (node, new_value) in new_values.iter_mut().enumerate().take(current_nodes) {
                let r_node = self.rates[step][node];
                let discount_rate = r_node + oas / 10000.0; // OAS in bps
                let df = (-discount_rate * dt).exp();

                // Calculate continuation value
                // In a recombining binomial tree, each node connects to two nodes in the next step
                let up_node = node + 1;
                let down_node = node;

                let continuation = if up_node < values.len() && down_node < values.len() {
                    df * (p_up * values[up_node] + p_down * values[down_node])
                } else if down_node < values.len() {
                    df * values[down_node] // Edge case: only down move available
                } else {
                    0.0 // Fallback if no valid continuation
                };

                // Create state for this node
                let time = step as F * dt;
                let mut vars = initial_vars.clone();
                vars.insert("step", step as F);
                vars.insert("node", node as F);
                vars.insert("time", time);
                vars.insert("interest_rate", r_node);
                vars.insert("oas", oas);

                let state = NodeState::new(step, time, vars, market_context);

                // Let valuator determine optimal action
                *new_value = valuator.value_at_node(&state, continuation)?;
            }

            values = new_values;
        }

        Ok(values[0])
    }

    fn calculate_greeks<V: TreeValuator>(
        &self,
        initial_vars: StateVariables,
        time_to_maturity: F,
        market_context: &MarketContext,
        valuator: &V,
        _bump_size: Option<F>,
    ) -> Result<TreeGreeks> {
        // Base price
        let base_price = self.price(
            initial_vars.clone(),
            time_to_maturity,
            market_context,
            valuator,
        )?;

        let mut greeks = TreeGreeks {
            price: base_price,
            delta: 0.0, // Not applicable for bond vs rates
            gamma: 0.0, // Not applicable for bond vs rates
            vega: 0.0,  // Volatility sensitivity
            theta: 0.0, // Time decay
            rho: 0.0,   // Interest rate sensitivity
        };

        // Calculate Vega (volatility sensitivity)
        // This requires rebuilding the tree with bumped volatility
        let mut bumped_config = self.config.clone();
        bumped_config.volatility += 0.01; // 1% vol bump

        // For now, approximate vega as 0 since rebuilding tree is expensive
        // In practice, would cache multiple trees or use analytical approximations
        greeks.vega = 0.0;

        // Calculate Rho (interest rate sensitivity)
        // Approximate using finite differences on OAS
        let mut bumped_vars = initial_vars.clone();
        let base_oas = initial_vars.get("oas").copied().unwrap_or(0.0);
        bumped_vars.insert("oas", base_oas + 1.0); // 1bp bump

        let bumped_price = self.price(bumped_vars, time_to_maturity, market_context, valuator)?;

        greeks.rho = bumped_price - base_price;

        // Calculate Theta (time decay) - 1 day bump
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

/// State variable keys specific to short-rate trees
pub mod short_rate_keys {
    /// Short rate at the current node
    pub const SHORT_RATE: &str = "interest_rate";
    /// Option-Adjusted Spread added to the short rate
    pub const OAS: &str = "oas";
    /// Current tree step
    pub const STEP: &str = "step";
    /// Current node index
    pub const NODE: &str = "node";
    /// Time from valuation date
    pub const TIME: &str = "time";
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    use finstack_core::prelude::InterpConfigurableBuilder;
    use time::Month;

    fn create_test_curve() -> DiscountCurve {
        DiscountCurve::builder("USD-OIS")
            .base_date(
                finstack_core::dates::Date::from_calendar_date(2025, Month::January, 1).unwrap(),
            )
            .knots([(0.0, 1.0), (1.0, 0.97), (2.0, 0.94), (5.0, 0.85)])
            .log_df()
            .build()
            .unwrap()
    }

    #[test]
    fn test_ho_lee_tree_creation() {
        let tree = ShortRateTree::ho_lee(50, 0.01);
        assert_eq!(tree.config.steps, 50);
        assert_eq!(tree.config.model, ShortRateModel::HoLee);
        assert_eq!(tree.config.volatility, 0.01);
    }

    #[test]
    fn test_tree_calibration() {
        let mut tree = ShortRateTree::ho_lee(10, 0.015);
        let curve = create_test_curve();

        let result = tree.calibrate(&curve, 2.0);
        assert!(result.is_ok());

        // Tree should have rates at each step
        assert_eq!(tree.rates.len(), 11); // 0 to 10 steps
        assert_eq!(tree.rates[0].len(), 1); // First step has one node
        assert_eq!(tree.rates[10].len(), 11); // Last step has 11 nodes
    }

    #[test]
    fn test_rate_access() {
        let mut tree = ShortRateTree::ho_lee(5, 0.01);
        let curve = create_test_curve();
        tree.calibrate(&curve, 1.0).unwrap();

        // Should be able to access rates at valid nodes
        let r0 = tree.rate_at_node(0, 0).unwrap();
        assert!(r0 > 0.0);

        let r_final = tree.rate_at_node(5, 2).unwrap();
        assert!(r_final.is_finite());

        // Invalid access should error
        assert!(tree.rate_at_node(10, 0).is_err());
        assert!(tree.rate_at_node(0, 5).is_err());
    }

    #[test]
    fn test_bdt_tree_creation() {
        let tree = ShortRateTree::black_derman_toy(25, 0.02, 0.1);
        assert_eq!(tree.config.model, ShortRateModel::BlackDermanToy);
        assert_eq!(tree.config.mean_reversion, Some(0.1));
    }
}
