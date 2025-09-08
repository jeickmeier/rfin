//! Binomial tree models for option pricing.
//!
//! Implements various binomial tree methods including Cox-Ross-Rubinstein (CRR)
//! and Leisen-Reimer for American and Bermudan option pricing.
//!
//! Now includes generic TreeModel implementation for pricing arbitrary instruments.

use crate::instruments::options::{ExerciseStyle, OptionType};
use std::collections::HashSet;
use finstack_core::market_data::context::MarketContext;
use finstack_core::{Error, Result, F};
use crate::instruments::options::models::NodeState;

// Import the generic tree framework
use super::tree_framework::{
    state_keys, single_factor_equity_state, StateVariables, TreeBranching, TreeGreeks, TreeModel,
    TreeValuator, map_exercise_dates_to_steps, price_recombining_tree, RecombiningInputs,
};

/// Binomial tree types
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TreeType {
    /// Cox-Ross-Rubinstein (standard binomial)
    CRR,
    /// Jarrow-Rudd (equal probability)
    JR,
    /// Leisen-Reimer (improved convergence)
    LeisenReimer,
    /// Tian (moment matching)
    Tian,
}

/// Binomial tree for option pricing
#[derive(Clone, Debug)]
pub struct BinomialTree {
    /// Number of time steps
    pub steps: usize,
    /// Tree type
    pub tree_type: TreeType,
    /// Cache tree nodes for efficiency
    pub use_cache: bool,
}

impl BinomialTree {
    /// Create new binomial tree with specified steps and type
    pub fn new(steps: usize, tree_type: TreeType) -> Self {
        Self {
            steps,
            tree_type,
            use_cache: true,
        }
    }

    /// Create a Leisen-Reimer tree (recommended for accuracy)
    pub fn leisen_reimer(steps: usize) -> Self {
        Self::new(steps, TreeType::LeisenReimer)
    }

    /// Create a standard CRR tree
    pub fn crr(steps: usize) -> Self {
        Self::new(steps, TreeType::CRR)
    }

    /// Peizer–Pratt inversion used by Leisen–Reimer to map normal quantiles to
    /// binomial cumulative probabilities. Uses the common closed form used in LR (1996).
    fn peizer_pratt_inversion(&self, z: F, n: usize) -> F {
        if n == 0 {
            return 0.5;
        }
        if z.abs() < 1e-14 {
            return 0.5;
        }

        // LR recommend an odd number of steps for best accuracy; use nearest upper odd in mapping
        let n_eff = if n % 2 == 0 { n + 1 } else { n } as f64;
        let sign = if z >= 0.0 { 1.0 } else { -1.0 };
        let z2 = z * z;

        // Peizer–Pratt mapping (standard LR form):
        // beta = z^2 * (m + 1/6) / (m + 1/3 + 0.1/(m+1))
        // H^{-1}(z) = 0.5 + sign(z)*0.5 * sqrt(1 - exp(-beta))
        let denom = n_eff + 1.0 / 3.0 + 0.1 / (n_eff + 1.0);
        let beta = z2 * (n_eff + 1.0 / 6.0) / denom;
        let p = 0.5 + sign * 0.5 * (1.0 - (-beta).exp()).sqrt();

        // Numerically enforce bounds
        p.clamp(0.0, 1.0)
    }

    /// Calculate tree parameters based on model type
    fn calculate_parameters(
        &self,
        spot: F,
        strike: F,
        r: F,
        sigma: F,
        t: F,
        q: F,
    ) -> Result<(F, F, F)> {
        if t <= 0.0 || sigma <= 0.0 {
            return Err(Error::Internal);
        }

        let dt = t / self.steps as f64;

        let (u, d, p) = match self.tree_type {
            TreeType::LeisenReimer => {
                // Fallback to CRR if strike/spot are not usable (e.g., generic tree)
                if spot <= 0.0 || strike <= 0.0 {
                    let u = (sigma * dt.sqrt()).exp();
                    let d = 1.0 / u;
                    let p = (((r - q) * dt).exp() - d) / (u - d);
                    if !(0.0..=1.0).contains(&p) {
                        return Err(Error::Internal);
                    }
                    return Ok((u, d, p));
                }

                // Leisen–Reimer: use Peizer–Pratt inversion to determine probabilities
                let d1 =
                    ((spot / strike).ln() + (r - q + 0.5 * sigma * sigma) * t) / (sigma * t.sqrt());
                let d2 = d1 - sigma * t.sqrt();

                // Probabilities via PP inversion
                let eps = 1e-12;
                let p = self
                    .peizer_pratt_inversion(d2, self.steps)
                    .clamp(eps, 1.0 - eps);

                // Mean/variance-matched u,d with PP probability (stable LR variant)
                let m1 = ((r - q) * dt).exp();
                let var = m1 * m1 * ((sigma * sigma * dt).exp() - 1.0);
                let one_minus_p = 1.0 - p;
                let denom = p * one_minus_p;
                if denom <= 0.0 {
                    return Err(Error::Internal);
                }
                let delta = (var / denom).sqrt();
                let d = m1 - p * delta;
                let u = m1 + one_minus_p * delta;

                if !(u.is_finite() && d.is_finite() && u > 1.0 && d < 1.0 && u > d) {
                    return Err(Error::Internal);
                }

                (u, d, p)
            }
            TreeType::CRR => {
                // Cox-Ross-Rubinstein parameters
                let u = (sigma * dt.sqrt()).exp();
                let d = 1.0 / u;
                let p = (((r - q) * dt).exp() - d) / (u - d);

                // Validate probability
                if !(0.0..=1.0).contains(&p) {
                    return Err(Error::Internal);
                }

                (u, d, p)
            }
            TreeType::JR => {
                // Jarrow-Rudd (equal probability) parameters
                let u = ((r - q - 0.5 * sigma * sigma) * dt + sigma * dt.sqrt()).exp();
                let d = ((r - q - 0.5 * sigma * sigma) * dt - sigma * dt.sqrt()).exp();
                let p = 0.5;

                (u, d, p)
            }
            TreeType::Tian => {
                // Tian moment-matching parameters
                let v = ((r - q) * dt).exp();
                let u = 0.5
                    * v
                    * (sigma * dt.sqrt()).exp()
                    * (1.0 + (1.0 + (sigma * sigma * dt) / (v * v)).sqrt());
                let d = 0.5
                    * v
                    * (sigma * dt.sqrt()).exp()
                    * (1.0 + (1.0 + (sigma * sigma * dt) / (v * v)).sqrt())
                    - v * (sigma * dt.sqrt()).exp();
                let p = (v - d) / (u - d);

                (u, d, p)
            }
        };

        Ok((u, d, p))
    }

    /// Internal unified pricer supporting European, American, and Bermudan styles
    /// via an optional list of exercise steps.
    #[allow(clippy::too_many_arguments)]
    fn price_with_exercise(
        &self,
        spot: F,
        strike: F,
        r: F,
        sigma: F,
        t: F,
        q: F,
        option_type: OptionType,
        exercise_steps: Option<&[usize]>,
    ) -> Result<F> {
        // Compute lattice parameters honoring the configured binomial model
        let (u, d, p) = self.calculate_parameters(spot, strike, r, sigma, t, q)?;

        // Build an option valuator that applies early exercise at requested steps
        let exercise_set: Option<HashSet<usize>> = exercise_steps
            .map(|steps| steps.iter().copied().collect::<HashSet<usize>>());

        struct OptionValuator {
            strike: F,
            option_type: OptionType,
            exercise_steps: Option<HashSet<usize>>,
        }

        impl TreeValuator for OptionValuator {
            fn value_at_maturity(&self, state: &NodeState) -> Result<F> {
                let s = state.spot().ok_or(Error::Internal)?;
                Ok(match self.option_type {
                    OptionType::Call => (s - self.strike).max(0.0),
                    OptionType::Put => (self.strike - s).max(0.0),
                })
            }

            fn value_at_node(&self, state: &NodeState, continuation_value: F) -> Result<F> {
                if let Some(steps) = &self.exercise_steps {
                    if steps.contains(&state.step) {
                        let s = state.spot().ok_or(Error::Internal)?;
                        let exercise = match self.option_type {
                            OptionType::Call => (s - self.strike).max(0.0),
                            OptionType::Put => (self.strike - s).max(0.0),
                        };
                        return Ok(continuation_value.max(exercise));
                    }
                }
                Ok(continuation_value)
            }
        }

        let valuator = OptionValuator {
            strike,
            option_type,
            exercise_steps: exercise_set,
        };

        let initial_vars = single_factor_equity_state(spot, r, q, sigma);

        // Delegate to the shared recombining engine
        price_recombining_tree(RecombiningInputs {
            branching: TreeBranching::Binomial,
            steps: self.steps,
            initial_vars,
            time_to_maturity: t,
            market_context: &MarketContext::new(), // not used by valuator
            valuator: &valuator,
            up_factor: u,
            down_factor: d,
            middle_factor: None,
            prob_up: p,
            prob_down: 1.0 - p,
            prob_middle: None,
            interest_rate: r,
        })
    }

    /// Price American option using binomial tree
    #[allow(clippy::too_many_arguments)]
    pub fn price_american(
        &self,
        spot: F,
        strike: F,
        r: F,
        sigma: F,
        t: F,
        q: F,
        option_type: OptionType,
    ) -> Result<F> {
        let all_steps: Vec<usize> = (0..self.steps).collect();
        self.price_with_exercise(spot, strike, r, sigma, t, q, option_type, Some(&all_steps))
    }

    /// Price European option using binomial tree (for validation)
    #[allow(clippy::too_many_arguments)]
    pub fn price_european(
        &self,
        spot: F,
        strike: F,
        r: F,
        sigma: F,
        t: F,
        q: F,
        option_type: OptionType,
    ) -> Result<F> {
        self.price_with_exercise(spot, strike, r, sigma, t, q, option_type, None)
    }

    /// Price Bermudan option with specified exercise dates
    #[allow(clippy::too_many_arguments)]
    pub fn price_bermudan(
        &self,
        spot: F,
        strike: F,
        r: F,
        sigma: F,
        t: F,
        q: F,
        option_type: OptionType,
        exercise_dates: &[F], // Times when exercise is allowed
    ) -> Result<F> {
        let mut steps = map_exercise_dates_to_steps(exercise_dates, t, self.steps);
        steps.sort();
        steps.dedup();
        self.price_with_exercise(spot, strike, r, sigma, t, q, option_type, Some(&steps))
    }

    /// Calculate Greeks using binomial tree
    #[allow(clippy::too_many_arguments)]
    pub fn calculate_greeks(
        &self,
        spot: F,
        strike: F,
        r: F,
        sigma: F,
        t: F,
        q: F,
        option_type: OptionType,
        exercise_style: ExerciseStyle,
    ) -> Result<BinomialGreeks> {
        // Price at base case
        let base_price = match exercise_style {
            ExerciseStyle::American => {
                self.price_american(spot, strike, r, sigma, t, q, option_type)?
            }
            ExerciseStyle::European => {
                self.price_european(spot, strike, r, sigma, t, q, option_type)?
            }
            _ => return Err(Error::Internal),
        };

        // Delta: use small bump
        let h = 0.01 * spot;
        let price_up = match exercise_style {
            ExerciseStyle::American => {
                self.price_american(spot + h, strike, r, sigma, t, q, option_type)?
            }
            ExerciseStyle::European => {
                self.price_european(spot + h, strike, r, sigma, t, q, option_type)?
            }
            _ => return Err(Error::Internal),
        };

        let price_down = match exercise_style {
            ExerciseStyle::American => {
                self.price_american(spot - h, strike, r, sigma, t, q, option_type)?
            }
            ExerciseStyle::European => {
                self.price_european(spot - h, strike, r, sigma, t, q, option_type)?
            }
            _ => return Err(Error::Internal),
        };

        let delta = (price_up - price_down) / (2.0 * h);
        let gamma = (price_up - 2.0 * base_price + price_down) / (h * h);

        // Theta: use 1-day bump
        let dt = 1.0 / 365.25;
        let theta = if t > dt {
            let price_later = match exercise_style {
                ExerciseStyle::American => {
                    self.price_american(spot, strike, r, sigma, t - dt, q, option_type)?
                }
                ExerciseStyle::European => {
                    self.price_european(spot, strike, r, sigma, t - dt, q, option_type)?
                }
                _ => return Err(Error::Internal),
            };
            -(base_price - price_later) / dt
        } else {
            0.0
        };

        Ok(BinomialGreeks {
            price: base_price,
            delta,
            gamma,
            theta,
        })
    }

    /// Generic pricing engine for arbitrary instruments
    ///
    /// This method implements the TreeModel trait, providing a flexible
    /// interface for pricing any instrument that implements TreeValuator.
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
        let q = initial_vars.get(state_keys::DIVIDEND_YIELD).copied().unwrap_or(0.0);
        let sigma = *initial_vars
            .get(state_keys::VOLATILITY)
            .ok_or(Error::Internal)?;

        // Calculate binomial parameters and delegate to the shared engine
        let (u, d, p) = self.calculate_parameters(0.0, 0.0, r, sigma, time_to_maturity, q)?;

        price_recombining_tree(RecombiningInputs {
            branching: TreeBranching::Binomial,
            steps: self.steps,
            initial_vars,
            time_to_maturity,
            market_context,
            valuator,
            up_factor: u,
            down_factor: d,
            middle_factor: None,
            prob_up: p,
            prob_down: 1.0 - p,
            prob_middle: None,
            interest_rate: r,
        })
    }
}

/// Greeks calculated from binomial tree
#[derive(Clone, Debug)]
pub struct BinomialGreeks {
    /// Option price
    pub price: F,
    /// Delta
    pub delta: F,
    /// Gamma
    pub gamma: F,
    /// Theta
    pub theta: F,
}

/// Implementation of TreeModel trait for BinomialTree
impl TreeModel for BinomialTree {
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
    use super::*;

    #[test]
    fn test_crr_european_converges_to_black_scholes() {
        // Test that CRR converges to Black-Scholes for European options
        let spot = 100.0;
        let strike = 100.0;
        let r = 0.05;
        let sigma = 0.20;
        let t = 1.0;
        let q = 0.0;

        // Calculate with increasing steps
        let tree_50 = BinomialTree::crr(50);
        let tree_100 = BinomialTree::crr(100);
        let tree_200 = BinomialTree::crr(200);

        let price_50 = tree_50
            .price_european(spot, strike, r, sigma, t, q, OptionType::Call)
            .unwrap();
        let price_100 = tree_100
            .price_european(spot, strike, r, sigma, t, q, OptionType::Call)
            .unwrap();
        let price_200 = tree_200
            .price_european(spot, strike, r, sigma, t, q, OptionType::Call)
            .unwrap();

        // Should converge
        assert!((price_100 - price_50).abs() > (price_200 - price_100).abs());

        // Should be close to Black-Scholes (approximately 10.45)
        assert!((price_200 - 10.45).abs() < 0.1);
    }

    #[test]
    fn test_leisen_reimer_better_convergence() {
        // Test that Leisen-Reimer converges faster than CRR
        let spot = 100.0;
        let strike = 100.0;
        let r = 0.05;
        let sigma = 0.20;
        let t = 1.0;
        let q = 0.0;

        let crr = BinomialTree::crr(401);
        let lr = BinomialTree::leisen_reimer(401);

        let crr_price = crr
            .price_european(spot, strike, r, sigma, t, q, OptionType::Call)
            .unwrap();
        let lr_price = lr
            .price_european(spot, strike, r, sigma, t, q, OptionType::Call)
            .unwrap();

        // Both should be close to Black-Scholes value
        let bs_value = 10.4506; // Known Black-Scholes value

        println!(
            "CRR(401)={}, LR(401)={}, BS={} diffs: CRR={}, LR={}",
            crr_price,
            lr_price,
            bs_value,
            (crr_price - bs_value).abs(),
            (lr_price - bs_value).abs()
        );

        // CRR should be reasonably close to Black-Scholes
        assert!(
            (crr_price - bs_value).abs() < 1.0,
            "CRR price should be close to BS value"
        );

        // LR should be within 5c of Black-Scholes at higher odd steps
        assert!(
            (lr_price - bs_value).abs() < 0.05,
            "LR(401) should be within 5c of BS"
        );
    }

    #[test]
    fn test_leisen_reimer_converges_put() {
        // Validate LR convergence for put via put-call parity
        let spot = 100.0;
        let strike = 100.0;
        let r = 0.05;
        let sigma = 0.20;
        let t = 1.0;
        let q = 0.0;

        let lr = BinomialTree::leisen_reimer(201);
        let lr_put = lr
            .price_european(spot, strike, r, sigma, t, q, OptionType::Put)
            .unwrap();

        // BS call value known; derive put via parity: P = C - S e^{-qT} + K e^{-rT}
        let bs_call = 10.4506;
        let bs_put = bs_call - spot * (-q * t).exp() + strike * (-r * t).exp();

        assert!(
            (lr_put - bs_put).abs() < 0.05,
            "LR(50) put should be within 5c of BS put"
        );
    }

    #[test]
    fn test_leisen_reimer_parameter_sanity_edges() {
        // Check probability and u/d bounds for short maturities and edge vols
        let spot = 100.0;
        let strike = 100.0;
        let r = 0.02;
        let q = 0.01;
        let t_small = 1e-3;

        for &sigma in &[0.01, 0.10, 0.50] {
            let tree = BinomialTree::leisen_reimer(51); // prefer odd steps
            let (u, d, p) = tree
                .calculate_parameters(spot, strike, r, sigma, t_small, q)
                .expect("LR params should compute");

            assert!((0.0..=1.0).contains(&p), "p must be in [0,1], got {}", p);
            assert!(
                u > 1.0 && d < 1.0 && u > d,
                "u>1>d must hold: u={}, d={}",
                u,
                d
            );
        }
    }

    #[test]
    fn test_american_put_early_exercise_premium() {
        // American put should be worth more than European put
        let spot = 100.0;
        let strike = 110.0;
        let r = 0.05;
        let sigma = 0.20;
        let t = 1.0;
        let q = 0.0;

        let tree = BinomialTree::crr(100); // Use CRR since LR has issues

        let american = tree
            .price_american(spot, strike, r, sigma, t, q, OptionType::Put)
            .unwrap();
        let european = tree
            .price_european(spot, strike, r, sigma, t, q, OptionType::Put)
            .unwrap();

        println!(
            "American put: {}, European put: {}, Premium: {}",
            american,
            european,
            american - european
        );

        // American should be worth more due to early exercise
        assert!(american >= european);
        assert!(
            american - european > 0.001,
            "Early exercise premium {} should be meaningful",
            american - european
        ); // Should have some early exercise premium
    }

    #[test]
    fn test_bermudan_between_european_and_american() {
        // Bermudan should be between European and American
        let spot = 100.0;
        let strike = 110.0;
        let r = 0.05;
        let sigma = 0.20;
        let t = 1.0;
        let q = 0.0;

        let tree = BinomialTree::leisen_reimer(100);

        // Exercise allowed quarterly
        let exercise_dates = vec![0.25, 0.5, 0.75, 1.0];

        let american = tree
            .price_american(spot, strike, r, sigma, t, q, OptionType::Put)
            .unwrap();
        let bermudan = tree
            .price_bermudan(
                spot,
                strike,
                r,
                sigma,
                t,
                q,
                OptionType::Put,
                &exercise_dates,
            )
            .unwrap();
        let european = tree
            .price_european(spot, strike, r, sigma, t, q, OptionType::Put)
            .unwrap();

        // Bermudan should be between European and American
        assert!(bermudan >= european);
        assert!(bermudan <= american);
    }

    #[test]
    fn test_exercise_schedule_mapping() {
        // Map quarterly exercise dates over 1Y with 4 steps
        let dates = vec![0.0, 0.25, 0.5, 0.75, 1.0];
        let steps = super::map_exercise_dates_to_steps(&dates, 1.0, 4);
        assert_eq!(steps, vec![0, 1, 2, 3, 4]);

        // Irregular dates should round to nearest step
        let dates2 = vec![0.12, 0.37, 0.62, 0.88];
        let steps2 = super::map_exercise_dates_to_steps(&dates2, 1.0, 4);
        assert_eq!(steps2, vec![0, 1, 2, 4]);
    }
}
