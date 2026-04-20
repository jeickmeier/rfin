//! Longstaff-Schwartz Monte Carlo (LSMC) for American/Bermudan options.
//!
//! Implements backward induction with least-squares regression to price
//! options with early exercise features.
//!
//! Reference: Longstaff & Schwartz (2001) - "Valuing American Options by Simulation"
//!
//! # In-sample upward bias
//!
//! This implementation estimates the continuation-value regression and the
//! resulting option price on the **same set of simulated paths** ("in-sample"
//! LSMC). The exercise policy is therefore fit to the noise of those paths,
//! which systematically biases the reported price *upward* relative to the
//! true American value. The magnitude of the bias is typically small (a few
//! basis points for smooth payoffs with well-chosen basis functions and
//! `num_paths ≳ 10⁴`) but grows with:
//!
//! - richer basis families (over-fitting is easier);
//! - fewer paths (less regression stability);
//! - payoff kinks near at-the-money states.
//!
//! For mission-critical pricing the standard remedy is to fit the regression
//! on one independent path set ("training") and apply the frozen exercise
//! policy to a separate path set ("pricing"). That two-pass pattern is not
//! implemented here; consumers who need an unbiased estimate should run the
//! pricer twice with disjoint seeds and manually apply the policy from the
//! first run to the second run's paths, or complement this estimator with
//! an Andersen-Broadie dual upper bound to bracket the true value.

use super::super::results::MoneyEstimate;
use super::lsq::regression_with_basis;
use crate::discretization::exact::ExactGbm;
use crate::estimate::Estimate;
use crate::online_stats::OnlineStats;
use crate::pricer::basis::BasisFunctions;
use crate::process::gbm::GbmProcess;
use crate::rng::philox::PhiloxRng;
use crate::time_grid::TimeGrid;
use crate::traits::{Discretization, RandomStream};
use finstack_core::currency::Currency;
use finstack_core::Result;

/// Immediate exercise payoff function.
///
/// Returns the payoff from exercising immediately at the given state.
pub trait ImmediateExercise: Send + Sync + Clone {
    /// Compute immediate exercise value.
    fn exercise_value(&self, spot: f64) -> f64;
}

/// American put option immediate exercise.
#[derive(Debug, Clone)]
pub struct AmericanPut {
    /// Strike price for American put option
    pub strike: f64,
}

impl AmericanPut {
    /// Create a validated American put with a positive strike.
    pub fn new(strike: f64) -> finstack_core::Result<Self> {
        if strike <= 0.0 {
            return Err(finstack_core::Error::Validation(
                "strike must be positive".to_string(),
            ));
        }
        Ok(Self { strike })
    }
}

impl ImmediateExercise for AmericanPut {
    fn exercise_value(&self, spot: f64) -> f64 {
        (self.strike - spot).max(0.0)
    }
}

/// American call option immediate exercise.
#[derive(Debug, Clone)]
pub struct AmericanCall {
    /// Strike price for American call option
    pub strike: f64,
}

impl AmericanCall {
    /// Create a validated American call with a positive strike.
    pub fn new(strike: f64) -> finstack_core::Result<Self> {
        if strike <= 0.0 {
            return Err(finstack_core::Error::Validation(
                "strike must be positive".to_string(),
            ));
        }
        Ok(Self { strike })
    }
}

impl ImmediateExercise for AmericanCall {
    fn exercise_value(&self, spot: f64) -> f64 {
        (spot - self.strike).max(0.0)
    }
}

/// LSMC configuration.
#[derive(Debug, Clone)]
pub struct LsmcConfig {
    /// Number of paths
    pub num_paths: usize,
    /// Random seed
    pub seed: u64,
    /// Exercise dates (step indices)
    pub exercise_dates: Vec<usize>,
    /// Use parallel execution
    pub use_parallel: bool,
}

impl LsmcConfig {
    /// Create a validated LSMC configuration.
    ///
    /// Verifies that `num_paths > 0`, `exercise_dates` is non-empty with
    /// strictly positive step indices, and every date satisfies
    /// `0 < date <= num_steps`. An index of `num_steps` corresponds to the
    /// terminal exercise (European boundary condition); any index strictly
    /// greater is a caller bug.
    pub fn new(
        num_paths: usize,
        exercise_dates: Vec<usize>,
        num_steps: usize,
    ) -> finstack_core::Result<Self> {
        if num_paths == 0 {
            return Err(finstack_core::Error::Validation(
                "num_paths must be positive".to_string(),
            ));
        }
        if exercise_dates.is_empty() {
            return Err(finstack_core::Error::Validation(
                "exercise_dates must have at least one element".to_string(),
            ));
        }
        if let Some(pos) = exercise_dates.iter().position(|&d| d == 0) {
            return Err(finstack_core::Error::Validation(format!(
                "exercise_dates must be strictly positive step indices (exercise_dates[{pos}] = 0 \
                 implies exercise before the first simulated step)"
            )));
        }
        if let Some(&bad) = exercise_dates.iter().find(|&&d| d > num_steps) {
            return Err(finstack_core::Error::Validation(format!(
                "exercise_dates contain {bad} which exceeds num_steps={num_steps}; each date \
                 must satisfy 0 < date <= num_steps"
            )));
        }
        Ok(Self {
            num_paths,
            seed: 42,
            exercise_dates,
            use_parallel: false,
        })
    }

    /// Set random seed.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// Enable or disable parallel path generation.
    ///
    /// Path generation is the dominant cost for large `num_paths`; when
    /// `enabled` is `true` the pricer uses a rayon par-iter and each path
    /// derives its own RNG via [`crate::rng::philox::PhiloxRng::split`] keyed
    /// on the path index, which keeps results bit-identical to the serial
    /// run.
    pub fn with_parallel(mut self, enabled: bool) -> Self {
        self.use_parallel = enabled;
        self
    }
}

/// LSMC pricer for American/Bermudan options.
///
/// Uses backward induction with least-squares regression to estimate
/// continuation values and optimal exercise decisions.
pub struct LsmcPricer {
    config: LsmcConfig,
}

impl LsmcPricer {
    /// Create a new LSMC pricer.
    pub fn new(config: LsmcConfig) -> Self {
        Self { config }
    }

    /// Price an American-style option.
    ///
    /// # Arguments
    ///
    /// * `process` - Stochastic process
    /// * `initial_spot` - Initial spot price
    /// * `time_to_maturity` - Time to maturity
    /// * `num_steps` - Number of time steps
    /// * `exercise` - Immediate exercise payoff
    /// * `basis` - Basis functions for regression
    /// * `currency` - Currency for result
    /// * `discount_rate` - Risk-free rate for discounting
    ///
    /// # Returns
    ///
    /// Statistical estimate of American option value
    #[allow(clippy::too_many_arguments)]
    pub fn price<E, B>(
        &self,
        process: &GbmProcess,
        initial_spot: f64,
        time_to_maturity: f64,
        num_steps: usize,
        exercise: &E,
        basis: &B,
        currency: Currency,
        discount_rate: f64,
    ) -> Result<MoneyEstimate>
    where
        E: ImmediateExercise,
        B: BasisFunctions + ?Sized,
    {
        // Step 1: Generate all paths
        let paths = self.generate_paths(process, initial_spot, time_to_maturity, num_steps)?;

        // Step 2: Backward induction with regression
        let values = self.backward_induction(
            &paths,
            exercise,
            basis,
            discount_rate,
            time_to_maturity,
            num_steps,
        )?;

        // Step 3: Compute statistics
        let mut stats = OnlineStats::new();
        for &value in &values {
            stats.update(value);
        }

        let estimate = Estimate::new(
            stats.mean(),
            stats.stderr(),
            stats.confidence_interval(0.05),
            values.len(),
        )
        .with_std_dev(stats.std_dev());

        Ok(MoneyEstimate::from_estimate(estimate, currency))
    }

    /// Generate Monte Carlo paths (serial or parallel depending on config).
    fn generate_paths(
        &self,
        process: &GbmProcess,
        initial_spot: f64,
        time_to_maturity: f64,
        num_steps: usize,
    ) -> Result<Vec<Vec<f64>>> {
        let time_grid = TimeGrid::uniform(time_to_maturity, num_steps)?;

        if self.config.use_parallel {
            return self.generate_paths_parallel(process, initial_spot, &time_grid, num_steps);
        }

        self.generate_paths_serial(process, initial_spot, &time_grid, num_steps)
    }

    /// Serial path generation.
    fn generate_paths_serial(
        &self,
        process: &GbmProcess,
        initial_spot: f64,
        time_grid: &TimeGrid,
        num_steps: usize,
    ) -> Result<Vec<Vec<f64>>> {
        let disc = ExactGbm::new();
        let rng = PhiloxRng::new(self.config.seed);
        let mut paths = Vec::with_capacity(self.config.num_paths);

        for path_id in 0..self.config.num_paths {
            let mut path_rng = rng.split(path_id as u64);
            let mut spot_path = Vec::with_capacity(num_steps + 1);
            let mut state = vec![initial_spot];
            let mut z = vec![0.0];
            let mut work = vec![];

            spot_path.push(initial_spot);

            for step in 0..num_steps {
                let t = time_grid.time(step);
                let dt = time_grid.dt(step);

                path_rng.fill_std_normals(&mut z);
                disc.step(process, t, dt, &mut state, &z, &mut work);

                spot_path.push(state[0]);
            }

            paths.push(spot_path);
        }

        Ok(paths)
    }

    /// Parallel path generation using rayon with deterministic per-path RNG.
    fn generate_paths_parallel(
        &self,
        process: &GbmProcess,
        initial_spot: f64,
        time_grid: &TimeGrid,
        num_steps: usize,
    ) -> Result<Vec<Vec<f64>>> {
        use rayon::prelude::*;

        let rng = PhiloxRng::new(self.config.seed);
        let disc = ExactGbm::new();

        let paths: Vec<Vec<f64>> = (0..self.config.num_paths)
            .into_par_iter()
            .map(|path_id| {
                let mut path_rng = rng.split(path_id as u64);
                let mut spot_path = Vec::with_capacity(num_steps + 1);
                let mut state = vec![initial_spot];
                let mut z = vec![0.0];
                let mut work = vec![];

                spot_path.push(initial_spot);

                for step in 0..num_steps {
                    let t = time_grid.time(step);
                    let dt = time_grid.dt(step);

                    path_rng.fill_std_normals(&mut z);
                    disc.step(process, t, dt, &mut state, &z, &mut work);

                    spot_path.push(state[0]);
                }

                spot_path
            })
            .collect();

        Ok(paths)
    }

    /// Perform backward induction with regression.
    ///
    /// # Discounting Convention
    ///
    /// This pricer uses **exponential discounting with a flat rate**: `exp(-r * t)`.
    /// This is appropriate when `discount_rate` represents a constant risk-free rate.
    ///
    /// **Contrast with Swaption LSMC**: The swaption pricer uses discount factors from
    /// a yield curve (`df_t / df_0`) to handle term structure. Both approaches produce
    /// present values at time 0, but differ in their input assumptions:
    /// - **American LSMC**: Flat rate input → exponential discounting
    /// - **Swaption LSMC**: Discount curve input → ratio of discount factors
    ///
    /// See `swaption_lsmc.rs` for the curve-based discounting approach.
    #[allow(clippy::too_many_arguments)]
    fn backward_induction<E, B>(
        &self,
        paths: &[Vec<f64>],
        exercise: &E,
        basis: &B,
        discount_rate: f64,
        time_to_maturity: f64,
        num_steps: usize,
    ) -> Result<Vec<f64>>
    where
        E: ImmediateExercise,
        B: BasisFunctions + ?Sized,
    {
        let num_paths = paths.len();
        let dt = time_to_maturity / num_steps as f64;

        // Cashflow matrix: when each path exercises
        let mut cashflows = vec![0.0; num_paths];
        let mut exercise_times = vec![time_to_maturity; num_paths];

        // Initialize with terminal values
        for (i, path) in paths.iter().enumerate() {
            let terminal_spot = path[num_steps];
            cashflows[i] = exercise.exercise_value(terminal_spot);
        }

        // Backward induction through exercise dates
        let mut sorted_exercise_dates = self.config.exercise_dates.clone();
        sorted_exercise_dates.sort_unstable();
        sorted_exercise_dates.reverse(); // Go backward

        let valid_exercise_count = sorted_exercise_dates
            .iter()
            .filter(|&&step| step > 0 && step < num_steps)
            .count();
        if valid_exercise_count == 0 {
            tracing::warn!(
                num_steps,
                exercise_dates = ?self.config.exercise_dates,
                "No exercise date is inside the simulated horizon (0 < step < num_steps); \
                 option priced as European (terminal exercise only)"
            );
        }

        // Pre-allocate regression buffers to avoid reallocations
        let mut regression_x = Vec::with_capacity(paths.len() / 2);
        let mut regression_y = Vec::with_capacity(paths.len() / 2);
        let mut regression_indices = Vec::with_capacity(paths.len() / 2);

        for &exercise_step in &sorted_exercise_dates {
            // Drop guards against:
            //   - exercise_step == 0: pre-simulation exercise, nonsensical.
            //   - exercise_step >= num_steps: past/at the terminal where the
            //     European payoff is already seeded in `cashflows`.
            if exercise_step == 0 || exercise_step >= num_steps {
                continue;
            }

            let t = exercise_step as f64 * dt;

            // Clear buffers for this exercise date (reuse capacity)
            regression_x.clear();
            regression_y.clear();
            regression_indices.clear();

            for (i, path) in paths.iter().enumerate() {
                let spot = path[exercise_step];
                let immediate = exercise.exercise_value(spot);

                // Only regress on ITM paths
                if immediate > 0.0 {
                    // Discount cashflow to this exercise date
                    let time_to_cashflow = exercise_times[i] - t;
                    let discounted_cf = cashflows[i] * (-discount_rate * time_to_cashflow).exp();

                    regression_x.push(spot);
                    regression_y.push(discounted_cf);
                    regression_indices.push(i);
                }
            }

            // Perform regression if we have enough ITM paths
            if regression_x.len() > basis.num_basis() + 10 {
                match regression_with_basis(&regression_x, &regression_y, basis) {
                    Ok(continuation_values) => {
                        // Exercise decision
                        for (j, &i) in regression_indices.iter().enumerate() {
                            let spot = paths[i][exercise_step];
                            let immediate = exercise.exercise_value(spot);
                            let continuation = continuation_values[j];

                            // Exercise if immediate value > continuation value
                            if immediate > continuation {
                                cashflows[i] = immediate;
                                exercise_times[i] = t;
                            }
                        }
                    }
                    Err(err) => {
                        tracing::warn!(
                            exercise_step,
                            itm_paths = regression_x.len(),
                            "LSMC regression failed, preserving continuation cashflows: {err}"
                        );
                    }
                }
            } else {
                // Fallback: too few ITM paths for stable regression.
                // Preserve existing continuation cashflows instead of forcing early exercise.
                tracing::debug!(
                    exercise_step,
                    itm_paths = regression_x.len(),
                    min_required = basis.num_basis() + 10,
                    "LSMC: insufficient ITM paths for regression, preserving continuation values"
                );
            }
        }

        // Discount all cashflows to present
        let mut present_values = vec![0.0; num_paths];
        for i in 0..num_paths {
            present_values[i] = cashflows[i] * (-discount_rate * exercise_times[i]).exp();
        }

        Ok(present_values)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::prelude::{LaguerreBasis, PolynomialBasis};
    use crate::process::gbm::GbmParams;

    #[test]
    fn test_polynomial_basis() {
        let basis = PolynomialBasis::new(2);
        let mut out = vec![0.0; 3];

        basis.evaluate(100.0, &mut out);

        assert_eq!(out[0], 1.0);
        assert_eq!(out[1], 100.0);
        assert_eq!(out[2], 10000.0);
    }

    #[test]
    fn test_laguerre_basis() {
        let basis = LaguerreBasis::new(2, 100.0);
        let mut out = vec![0.0; 3];

        basis.evaluate(100.0, &mut out);

        assert_eq!(out[0], 1.0);
        // L_1(1) = 1 - 1 = 0
        assert_eq!(out[1], 0.0);
    }

    #[test]
    fn test_laguerre_basis_non_standard_strikes() {
        // Test that normalization works for non-standard strikes
        let basis_low = LaguerreBasis::new(2, 1.0);
        let basis_high = LaguerreBasis::new(2, 1000.0);
        let mut out_low = vec![0.0; 3];
        let mut out_high = vec![0.0; 3];

        // Both should normalize to x=1.0 when spot equals strike
        basis_low.evaluate(1.0, &mut out_low);
        basis_high.evaluate(1000.0, &mut out_high);

        // L_1(1) = 0 for both
        assert_eq!(out_low[1], 0.0);
        assert_eq!(out_high[1], 0.0);

        // Verify strike accessor
        assert_eq!(basis_low.strike(), 1.0);
        assert_eq!(basis_high.strike(), 1000.0);
    }

    #[test]
    fn test_american_put_exercise() {
        let put = AmericanPut { strike: 100.0 };

        assert_eq!(put.exercise_value(90.0), 10.0);
        assert_eq!(put.exercise_value(110.0), 0.0);
    }

    #[test]
    fn test_american_call_exercise() {
        let call = AmericanCall { strike: 100.0 };

        assert_eq!(call.exercise_value(110.0), 10.0);
        assert_eq!(call.exercise_value(90.0), 0.0);
    }

    #[test]
    fn test_lsmc_basic() {
        // Basic test of LSMC infrastructure
        let exercise_dates = vec![50, 100];
        let config = LsmcConfig::new(1_000, exercise_dates, 100)
            .unwrap()
            .with_seed(42);
        let pricer = LsmcPricer::new(config);

        let gbm = GbmProcess::new(GbmParams::new(0.05, 0.0, 0.3).unwrap());
        let put = AmericanPut { strike: 100.0 };
        let basis = PolynomialBasis::new(2);

        let result = pricer
            .price(&gbm, 100.0, 1.0, 100, &put, &basis, Currency::USD, 0.05)
            .expect("LSMC pricing should succeed in test");

        // American put should have positive value
        assert!(result.mean.amount() > 0.0);
        assert!(result.mean.amount() < 50.0); // Sanity check
    }

    #[test]
    fn test_lsmc_high_degree_polynomial() {
        // Test with degree-5 polynomial (can be ill-conditioned)
        // This tests QR robustness vs Cholesky
        let exercise_dates = vec![25, 50, 75, 100];
        let config = LsmcConfig::new(5_000, exercise_dates, 100)
            .unwrap()
            .with_seed(42);
        let pricer = LsmcPricer::new(config);

        let gbm = GbmProcess::new(GbmParams::new(0.05, 0.0, 0.3).unwrap());
        let put = AmericanPut { strike: 100.0 };

        // High-degree polynomial basis (more prone to ill-conditioning)
        let basis = PolynomialBasis::new(5);

        let result = pricer.price(&gbm, 80.0, 1.0, 100, &put, &basis, Currency::USD, 0.05);

        // Should not panic or produce NaN
        assert!(result.is_ok());
        let price = result.expect("LSMC pricing should succeed in test");
        assert!(price.mean.amount().is_finite());
        assert!(price.mean.amount() > 0.0);

        println!("High-degree poly LSMC (deep ITM): {}", price.mean);
    }

    #[test]
    fn test_lsmc_extreme_spot_ranges() {
        // Test with paths spanning wide spot range (10 to 1000)
        // This can cause numerical issues with polynomial basis
        let exercise_dates = vec![50, 100];
        let config = LsmcConfig::new(5_000, exercise_dates, 100)
            .unwrap()
            .with_seed(123);
        let pricer = LsmcPricer::new(config);

        // High volatility to get wide spot range
        let gbm = GbmProcess::new(GbmParams::new(0.05, 0.0, 1.0).unwrap());
        let put = AmericanPut { strike: 100.0 };
        let basis = PolynomialBasis::new(3);

        let result = pricer.price(&gbm, 100.0, 1.0, 100, &put, &basis, Currency::USD, 0.05);

        // Should remain stable even with extreme paths
        assert!(result.is_ok());
        let price = result.expect("LSMC pricing should succeed in test");
        assert!(price.mean.amount().is_finite());
        assert!(price.mean.amount() >= 0.0);

        println!("Extreme spot ranges LSMC: {}", price.mean);
    }

    #[test]
    fn test_lsmc_few_itm_paths() {
        // Deep OTM put with few ITM paths
        // Tests regression fallback when insufficient data
        let exercise_dates = vec![50, 100];
        let config = LsmcConfig::new(1_000, exercise_dates, 100)
            .unwrap()
            .with_seed(456);
        let pricer = LsmcPricer::new(config);

        // Low volatility, deep OTM
        let gbm = GbmProcess::new(GbmParams::new(0.05, 0.0, 0.05).unwrap());
        let put = AmericanPut { strike: 50.0 };
        let basis = PolynomialBasis::new(2);

        // Start well above strike
        let result = pricer.price(&gbm, 150.0, 0.5, 100, &put, &basis, Currency::USD, 0.05);

        // Should handle gracefully (very small value expected)
        assert!(result.is_ok());
        let price = result.expect("LSMC pricing should succeed in test");
        assert!(price.mean.amount().is_finite());
        assert!(price.mean.amount() >= 0.0);
        assert!(price.mean.amount() < 0.1); // Should be near zero

        println!("Few ITM paths LSMC: {}", price.mean);
    }

    #[test]
    fn test_lsmc_insufficient_itm_paths_preserves_continuation() {
        let config = LsmcConfig::new(1, vec![1], 2).unwrap();
        let pricer = LsmcPricer::new(config);
        let exercise = AmericanCall { strike: 100.0 };
        let basis = PolynomialBasis::new(2);
        let paths = vec![vec![100.0, 110.0, 130.0]];

        let present_values = pricer
            .backward_induction(&paths, &exercise, &basis, 0.05, 1.0, 2)
            .expect("backward induction should succeed");

        let expected = 30.0 * (-0.05_f64).exp();
        assert!((present_values[0] - expected).abs() < 1e-12);
    }

    #[test]
    fn test_lsmc_config_rejects_zero_exercise_date() {
        let err = LsmcConfig::new(100, vec![0, 10, 20], 100)
            .expect_err("should reject zero step")
            .to_string();
        assert!(
            err.contains("strictly positive"),
            "unexpected error message: {err}"
        );
    }

    #[test]
    fn test_lsmc_config_rejects_date_beyond_num_steps() {
        let err = LsmcConfig::new(100, vec![5, 15, 42], 20)
            .expect_err("should reject date > num_steps")
            .to_string();
        assert!(
            err.contains("42") && err.contains("num_steps=20"),
            "unexpected error message: {err}"
        );
    }

    #[test]
    fn test_lsmc_config_accepts_terminal_date() {
        let cfg =
            LsmcConfig::new(100, vec![5, 10, 20], 20).expect("terminal date should be accepted");
        assert_eq!(cfg.exercise_dates, vec![5, 10, 20]);
    }

    #[test]
    fn test_lsmc_tiny_positive_intrinsic_values_are_treated_as_itm() {
        let config = LsmcConfig::new(16, vec![1], 2).unwrap();
        let pricer = LsmcPricer::new(config);
        let exercise = AmericanCall { strike: 100.0 };
        let basis = PolynomialBasis::new(1);
        let paths = vec![vec![100.0, 100.0 + 1.0e-8, 100.0]; 16];

        let present_values = pricer
            .backward_induction(&paths, &exercise, &basis, 0.0, 1.0, 2)
            .expect("backward induction should succeed");

        for value in present_values {
            assert!(
                (value - 1.0e-8).abs() < 1.0e-14,
                "tiny intrinsic value should trigger exercise instead of being dropped: {value}"
            );
        }
    }
}
