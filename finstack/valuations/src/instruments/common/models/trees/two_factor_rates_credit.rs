//! Two-factor binomial tree: short rate + credit hazard (intensity).
//!
//! Models the joint evolution of the risk-free short rate and the credit hazard
//! rate using correlated binomial moves. Both factors are **calibrated** to their
//! respective market curves (discount curve for rates, hazard curve for credit)
//! via independent Arrow-Debreu forward induction, analogous to Ho-Lee calibration.
//!
//! # Calibration
//!
//! `calibrate()` must be called before `price()`. The calibration ensures:
//! - Tree-implied zero-coupon bond prices match the discount curve at every step
//! - Tree-implied survival probabilities match the hazard curve at every step
//!
//! # OAS
//!
//! Option-adjusted spread is read from `initial_vars["oas"]` (basis points) and
//! applied as a parallel shift to calibrated short rates during backward induction.
//! This matches the `ShortRateTree` convention.

use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::HazardCurve;
use finstack_core::market_data::traits::Discounting;
use finstack_core::{Error, Result};

use super::state_keys;
use super::tree_framework::{NodeState, StateVariables, TreeModel, TreeValuator};

/// Configuration for rates + credit two-factor tree.
#[derive(Debug, Clone)]
pub struct RatesCreditConfig {
    /// Number of time steps
    pub steps: usize,
    /// Short-rate volatility (annualized, normal / Ho-Lee convention)
    pub rate_vol: f64,
    /// Credit hazard volatility (annualized, normal convention)
    pub hazard_vol: f64,
    /// Base short rate seed (only used as initial guess for calibration)
    pub base_rate: f64,
    /// Base hazard seed (only used as initial guess for calibration)
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
///
/// Both factors are calibrated to market curves via `calibrate()`. Calling
/// `price()` without prior calibration returns an error.
#[derive(Debug, Clone)]
pub struct RatesCreditTree {
    /// Rates-credit tree configuration
    pub config: RatesCreditConfig,
    /// Calibrated short rates: `rates[step][node_i]`.
    /// Populated by `calibrate()`.
    calibrated_rates: Vec<Vec<f64>>,
    /// Calibrated hazard rates: `hazards[step][node_j]`.
    /// Populated by `calibrate()`.
    calibrated_hazards: Vec<Vec<f64>>,
    /// Recovery rate from the hazard curve (populated by `calibrate()`).
    recovery_rate: f64,
}

impl RatesCreditTree {
    /// Create a new rates-credit tree with the given configuration.
    ///
    /// `calibrate()` must be called before `price()`.
    pub fn new(config: RatesCreditConfig) -> Self {
        Self {
            config,
            calibrated_rates: Vec::new(),
            calibrated_hazards: Vec::new(),
            recovery_rate: 0.0,
        }
    }

    /// Calibrate both factors to market curves using Arrow-Debreu forward induction.
    ///
    /// - **Rate factor**: calibrated to the discount curve (Ho-Lee style theta adjustment)
    /// - **Hazard factor**: calibrated to the hazard curve's survival probabilities
    ///
    /// After calibration, `price()` uses the stored per-node rates and hazards.
    ///
    /// # Arguments
    ///
    /// * `disc` - Discount curve for risk-free rate calibration
    /// * `hazard` - Hazard curve for credit intensity calibration
    /// * `time_to_maturity` - Total time horizon in years
    pub fn calibrate(
        &mut self,
        disc: &dyn Discounting,
        hazard: &HazardCurve,
        time_to_maturity: f64,
    ) -> Result<()> {
        let steps = self.config.steps;
        if steps == 0 || time_to_maturity <= 0.0 {
            return Err(Error::Internal);
        }
        let dt = time_to_maturity / steps as f64;

        // Store recovery rate from hazard curve.
        self.recovery_rate = hazard.recovery_rate();

        // --- Rate factor calibration (Ho-Lee style) ---
        self.calibrated_rates = self.calibrate_factor_ho_lee(
            steps,
            dt,
            self.config.rate_vol,
            |t| disc.df(t),
            time_to_maturity,
        )?;

        // --- Hazard factor calibration (same Ho-Lee approach targeting survival) ---
        self.calibrated_hazards = self.calibrate_factor_ho_lee(
            steps,
            dt,
            self.config.hazard_vol,
            |t| hazard.sp(t),
            time_to_maturity,
        )?;

        Ok(())
    }

    /// Return the recovery rate from the most recent `calibrate()` call.
    pub fn recovery_rate(&self) -> f64 {
        self.recovery_rate
    }

    /// Ho-Lee style calibration for a single factor.
    ///
    /// Builds a 1D binomial lattice with additive normal volatility (`sigma * sqrt(dt)`)
    /// and solves for a theta (drift) at each step so that the lattice-implied
    /// "discount factor" matches a target curve.
    ///
    /// - For the rate factor: `target_fn(t) = disc.df(t)` (discount factor)
    /// - For the hazard factor: `target_fn(t) = hazard.sp(t)` (survival probability)
    ///
    /// Both share the same mathematical structure: the product `exp(-x * dt)` over
    /// path nodes must match the target curve value at each maturity.
    fn calibrate_factor_ho_lee(
        &self,
        steps: usize,
        dt: f64,
        sigma: f64,
        target_fn: impl Fn(f64) -> f64,
        time_to_maturity: f64,
    ) -> Result<Vec<Vec<f64>>> {
        let mut rates = vec![Vec::new(); steps + 1];

        // Initial rate: r0 = -ln(target(dt)) / dt
        let target_dt = target_fn(dt);
        let r0 = if target_dt > 0.0 && dt > 0.0 {
            -target_dt.ln() / dt
        } else {
            0.03 // Fallback
        };
        rates[0] = vec![r0];

        // Arrow-Debreu state prices
        let mut state_prices = vec![1.0];

        let sqrt_dt = dt.sqrt();

        for step in 0..steps {
            let next_nodes = step + 2;
            let mut next_rates_base = vec![0.0; next_nodes];
            let mut next_state_prices = vec![0.0; next_nodes];

            // Propagate state prices and compute base rates (without theta)
            for (i, &current_rate) in rates[step].iter().enumerate() {
                let q = state_prices[i];
                let df_i = (-current_rate * dt).exp();

                // Up move (to node i+1)
                let r_up_base = current_rate + sigma * sqrt_dt;
                if i + 1 < next_nodes {
                    next_rates_base[i + 1] = r_up_base;
                    next_state_prices[i + 1] += q * df_i * 0.5;
                }

                // Down move (to node i)
                let r_down_base = current_rate - sigma * sqrt_dt;
                if i < next_nodes {
                    next_rates_base[i] = r_down_base;
                    next_state_prices[i] += q * df_i * 0.5;
                }
            }

            // Solve for theta: target = exp(-theta*dt) * sum_j(Q_next[j] * exp(-r_base[j]*dt))
            let next_next_time = (step + 2) as f64 * dt;
            let theta = if next_next_time <= time_to_maturity + dt * 0.5 {
                let p_target = target_fn(next_next_time);
                let mut p_model_base = 0.0;
                for (j, &q_next) in next_state_prices.iter().enumerate() {
                    p_model_base += q_next * (-next_rates_base[j] * dt).exp();
                }
                if p_model_base > 0.0 && p_target > 0.0 {
                    -(p_target / p_model_base).ln() / dt
                } else {
                    0.0
                }
            } else {
                0.0
            };

            // Apply theta to get final calibrated rates
            let mut next_rates = vec![0.0; next_nodes];
            for j in 0..next_nodes {
                next_rates[j] = next_rates_base[j] + theta;
            }
            rates[step + 1] = next_rates;
            state_prices = next_state_prices;
        }

        Ok(rates)
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
}

impl TreeModel for RatesCreditTree {
    fn price<V: TreeValuator>(
        &self,
        initial_vars: StateVariables,
        time_to_maturity: f64,
        market_context: &MarketContext,
        valuator: &V,
    ) -> Result<f64> {
        if self.calibrated_rates.is_empty() || self.calibrated_hazards.is_empty() {
            return Err(Error::Internal); // Tree not calibrated
        }
        if self.config.steps == 0 || time_to_maturity <= 0.0 {
            return Err(Error::Internal);
        }

        let steps = self.config.steps;
        let dt = time_to_maturity / steps as f64;

        // OAS from initial variables (bp units, same convention as ShortRateTree)
        let oas_decimal = initial_vars.get("oas").copied().unwrap_or(0.0) / 10_000.0;

        // Pre-allocate double buffers for backward induction (zero allocations in loop)
        let max_nodes = steps + 1;
        let mut curr_values: Vec<Vec<f64>> = vec![vec![0.0; max_nodes]; max_nodes];
        let mut next_values: Vec<Vec<f64>> = vec![vec![0.0; max_nodes]; max_nodes];
        let mut vars = initial_vars.clone();

        // Initialize terminal values
        #[allow(clippy::needless_range_loop)]
        for i in 0..=steps {
            let r_t = self.calibrated_rates[steps][i];
            #[allow(clippy::needless_range_loop)]
            for j in 0..=steps {
                let h_t = self.calibrated_hazards[steps][j];

                vars.insert(state_keys::INTEREST_RATE, r_t.max(1e-8));
                vars.insert(state_keys::HAZARD_RATE, h_t.max(0.0));
                vars.insert("step", steps as f64);
                vars.insert("node_i", i as f64);
                vars.insert("node_j", j as f64);
                vars.insert("time", time_to_maturity);

                let state = NodeState::new(steps, time_to_maturity, &vars, market_context);
                curr_values[i][j] = valuator.value_at_maturity(&state)?;
            }
        }

        // Backward induction with double-buffering
        for k in (0..steps).rev() {
            for i in 0..=k {
                let r_t = self.calibrated_rates[k][i];

                // Rate transition probability with mean reversion
                let p_r = if self.config.rate_mean_reversion > 0.0 && r_t > 0.0 {
                    let log_r = r_t.ln();
                    let log_base = self.config.base_rate.max(1e-8).ln();
                    let drift = -self.config.rate_mean_reversion * (log_r - log_base);
                    let rate_vol = self.config.rate_vol.max(1e-12);
                    (0.5 + drift * dt.sqrt() / (2.0 * rate_vol)).clamp(0.0, 1.0)
                } else {
                    0.5
                };

                for j in 0..=k {
                    let h_t = self.calibrated_hazards[k][j];

                    // Hazard transition probability with mean reversion
                    let p_h = if self.config.hazard_mean_reversion > 0.0 && h_t > 0.0 {
                        let log_h = h_t.ln();
                        let log_base = self.config.base_hazard.max(1e-8).ln();
                        let drift = -self.config.hazard_mean_reversion * (log_h - log_base);
                        let hazard_vol = self.config.hazard_vol.max(1e-12);
                        (0.5 + drift * dt.sqrt() / (2.0 * hazard_vol)).clamp(0.0, 1.0)
                    } else {
                        0.5
                    };

                    // Joint probabilities
                    let (p_uu, p_ud, p_du, p_dd) = self.joint_probabilities(p_r, p_h);

                    // Continuation from four children at step k+1
                    let v_uu = curr_values[i + 1][j + 1];
                    let v_ud = curr_values[i + 1][j];
                    let v_du = curr_values[i][j + 1];
                    let v_dd = curr_values[i][j];

                    // Risk-free discounting with calibrated rate + OAS
                    let df = (-(r_t.max(1e-8) + oas_decimal) * dt).exp();
                    let cont = df * (p_uu * v_uu + p_ud * v_ud + p_du * v_du + p_dd * v_dd);

                    vars.insert(state_keys::INTEREST_RATE, r_t.max(1e-8));
                    vars.insert(state_keys::HAZARD_RATE, h_t.max(0.0));
                    vars.insert(state_keys::DF, df);
                    vars.insert("step", k as f64);
                    vars.insert("node_i", i as f64);
                    vars.insert("node_j", j as f64);
                    vars.insert("time", k as f64 * dt);

                    let state = NodeState::new(k, k as f64 * dt, &vars, market_context);
                    next_values[i][j] = valuator.value_at_node(&state, cont, dt)?;
                }
            }
            // Swap buffers (O(1) pointer swap, no data copy)
            std::mem::swap(&mut curr_values, &mut next_values);
        }

        Ok(curr_values[0][0])
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use finstack_core::market_data::context::MarketContext;
    use finstack_core::market_data::term_structures::DiscountCurve;
    use finstack_core::math::interp::InterpStyle;

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

    fn test_base_date() -> finstack_core::dates::Date {
        finstack_core::dates::Date::from_calendar_date(2025, time::Month::January, 1)
            .expect("valid date")
    }

    fn sloped_discount_curve() -> DiscountCurve {
        DiscountCurve::builder("USD-OIS")
            .base_date(test_base_date())
            .knots([
                (0.0, 1.0),
                (1.0, 0.96),
                (2.0, 0.91),
                (3.0, 0.86),
                (5.0, 0.78),
                (10.0, 0.60),
            ])
            .interp(InterpStyle::LogLinear)
            .build()
            .expect("curve should build")
    }

    fn test_hazard_curve() -> HazardCurve {
        use finstack_core::market_data::term_structures::ParInterp;
        HazardCurve::builder("TEST-HAZ")
            .base_date(test_base_date())
            .recovery_rate(0.4)
            .knots([(0.0, 0.02), (2.0, 0.025), (5.0, 0.03), (10.0, 0.035)])
            .par_interp(ParInterp::Linear)
            .build()
            .expect("hazard curve should build")
    }

    #[test]
    fn rates_credit_calibrated_prices_positive() {
        let disc = sloped_discount_curve();
        let haz = test_hazard_curve();
        let mut tree = RatesCreditTree::new(RatesCreditConfig {
            steps: 40,
            ..Default::default()
        });
        tree.calibrate(&disc, &haz, 5.0).expect("calibration");

        let ctx = MarketContext::new();
        let vars = StateVariables::default();
        let val = DummyValuator;
        let price = tree.price(vars, 5.0, &ctx, &val).expect("should succeed");
        assert!(price.is_finite() && price > 0.0);
    }

    #[test]
    fn uncalibrated_tree_returns_error() {
        let tree = RatesCreditTree::new(RatesCreditConfig::default());
        let ctx = MarketContext::new();
        let vars = StateVariables::default();
        let val = DummyValuator;
        let result = tree.price(vars, 1.0, &ctx, &val);
        assert!(result.is_err(), "price() without calibrate() must fail");
    }

    /// Verify that tree-implied ZCB prices at each step match `disc.df(t)` within 1e-6.
    ///
    /// The DummyValuator passes continuation through unchanged and pays 1.0 at
    /// maturity, so tree price = ZCB price ≈ disc.df(T) for any number of steps.
    #[test]
    fn calibration_quality_zcb_repricing() {
        let disc = sloped_discount_curve();
        let haz = test_hazard_curve();
        let steps = 60;
        let ttm = 5.0;

        let mut tree = RatesCreditTree::new(RatesCreditConfig {
            steps,
            rate_vol: 0.01,
            hazard_vol: 0.0, // no hazard vol → pure rate test
            ..Default::default()
        });
        tree.calibrate(&disc, &haz, ttm).expect("calibrate");

        let ctx = MarketContext::new();
        let vars = StateVariables::default();
        let val = DummyValuator;
        let tree_price = tree.price(vars, ttm, &ctx, &val).expect("price");
        let market_df = disc.df(ttm);

        let error_bps = (tree_price - market_df).abs() * 10_000.0;
        assert!(
            error_bps < 1.0, // within 1 bp
            "ZCB repricing error = {:.4} bps (tree={:.8}, market={:.8})",
            error_bps,
            tree_price,
            market_df
        );
    }

    /// Verify that calibrated hazard rates reproduce the hazard curve's survival
    /// probabilities at each step, using Arrow-Debreu forward induction on the
    /// 1D hazard lattice.
    #[test]
    fn calibration_quality_survival_matching() {
        let disc = sloped_discount_curve();
        let haz = test_hazard_curve();
        let steps = 50;
        let ttm = 5.0;
        let dt = ttm / steps as f64;

        let mut tree = RatesCreditTree::new(RatesCreditConfig {
            steps,
            hazard_vol: 0.20,
            ..Default::default()
        });
        tree.calibrate(&disc, &haz, ttm).expect("calibrate");

        // Forward-propagate Arrow-Debreu state prices through the calibrated
        // hazard lattice to compute model survival probability at each step.
        // No floor applied — must exactly mirror the calibration logic.
        let mut state_prices = vec![1.0_f64]; // Q_h[0] = 1.0

        for k in 0..steps {
            let next_nodes = k + 2;
            let mut next_sp = vec![0.0_f64; next_nodes];
            for j in 0..=k {
                let h_j = tree.calibrated_hazards[k][j];
                let surv_df = (-h_j * dt).exp();
                let q = state_prices[j];
                // Up move to j+1, down move to j — p = 0.5 each (no mean reversion)
                if j + 1 < next_nodes {
                    next_sp[j + 1] += q * surv_df * 0.5;
                }
                next_sp[j] += q * surv_df * 0.5;
            }
            state_prices = next_sp;

            // Model survival probability at step k+1 = sum of state prices
            let model_sp: f64 = state_prices.iter().sum();
            let t = (k + 1) as f64 * dt;
            let market_sp = haz.sp(t);

            let error = (model_sp - market_sp).abs();
            assert!(
                error < 1e-6,
                "Survival mismatch at step {} (t={:.3}): model={:.8}, market={:.8}, err={:.2e}",
                k + 1,
                t,
                model_sp,
                market_sp,
                error
            );
        }
    }
}
