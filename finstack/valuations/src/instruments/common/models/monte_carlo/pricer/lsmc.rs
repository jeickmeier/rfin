//! Longstaff-Schwartz Monte Carlo (LSMC) for American/Bermudan options.
//!
//! Implements backward induction with least-squares regression to price
//! options with early exercise features.
//!
//! Reference: Longstaff & Schwartz (2001) - "Valuing American Options by Simulation"

use crate::instruments::common::mc::discretization::exact::ExactGbm;
use crate::instruments::common::mc::process::gbm::GbmProcess;
use super::super::results::MoneyEstimate;
use crate::instruments::common::mc::rng::philox::PhiloxRng;
use crate::instruments::common::mc::stats::OnlineStats;
use crate::instruments::common::mc::time_grid::TimeGrid;
use crate::instruments::common::mc::traits::{Discretization, RandomStream};
use crate::instruments::common::mc::results::Estimate;
use super::lsq::solve_least_squares;
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
#[derive(Clone, Debug)]
pub struct AmericanPut {
    pub strike: f64,
}

impl ImmediateExercise for AmericanPut {
    fn exercise_value(&self, spot: f64) -> f64 {
        (self.strike - spot).max(0.0)
    }
}

/// American call option immediate exercise.
#[derive(Clone, Debug)]
pub struct AmericanCall {
    pub strike: f64,
}

impl ImmediateExercise for AmericanCall {
    fn exercise_value(&self, spot: f64) -> f64 {
        (spot - self.strike).max(0.0)
    }
}

/// Basis functions for LSMC regression.
pub trait BasisFunctions: Send + Sync {
    /// Number of basis functions.
    fn num_basis(&self) -> usize;

    /// Evaluate all basis functions at given spot.
    fn evaluate(&self, spot: f64, out: &mut [f64]);
}

/// Polynomial basis functions: {1, S, S², ...}.
#[derive(Clone, Debug)]
pub struct PolynomialBasis {
    degree: usize,
}

impl PolynomialBasis {
    /// Create polynomial basis of given degree.
    ///
    /// degree=2 gives: {1, S, S²}
    pub fn new(degree: usize) -> Self {
        assert!(degree > 0, "Degree must be positive");
        Self { degree }
    }
}

impl BasisFunctions for PolynomialBasis {
    fn num_basis(&self) -> usize {
        self.degree + 1
    }

    fn evaluate(&self, spot: f64, out: &mut [f64]) {
        debug_assert_eq!(
            out.len(),
            self.num_basis(),
            "Buffer size mismatch: expected {}, got {}",
            self.num_basis(),
            out.len()
        );
        out[0] = 1.0;
        for i in 1..=self.degree {
            out[i] = out[i - 1] * spot;
        }
    }
}

/// Laguerre basis functions (better for options).
///
/// Uses Laguerre polynomials normalized by strike price for proper scaling
/// across different strike levels.
#[derive(Clone, Debug)]
pub struct LaguerreBasis {
    degree: usize,
    strike: f64,
}

impl LaguerreBasis {
    /// Create Laguerre basis of given degree with strike normalization.
    ///
    /// # Arguments
    ///
    /// * `degree` - Polynomial degree (1-4)
    /// * `strike` - Strike price for normalization (must be > 0)
    ///
    /// # Panics
    ///
    /// Panics if `degree` is not in [1, 4] or `strike <= 0`.
    pub fn new(degree: usize, strike: f64) -> Self {
        assert!(degree > 0 && degree <= 4, "Degree must be 1-4");
        assert!(strike > 0.0, "Strike must be positive");
        Self { degree, strike }
    }

    /// Get the strike price used for normalization.
    pub fn strike(&self) -> f64 {
        self.strike
    }
}

impl BasisFunctions for LaguerreBasis {
    fn num_basis(&self) -> usize {
        self.degree + 1
    }

    fn evaluate(&self, spot: f64, out: &mut [f64]) {
        debug_assert_eq!(
            out.len(),
            self.num_basis(),
            "Buffer size mismatch: expected {}, got {}",
            self.num_basis(),
            out.len()
        );
        // Laguerre polynomials: L_0(x)=1, L_1(x)=1-x, L_2(x)=1-2x+x²/2, ...
        // Applied to x = S/K (normalized spot by strike)
        let x = spot / self.strike;

        out[0] = 1.0;
        if self.degree >= 1 {
            out[1] = 1.0 - x;
        }
        if self.degree >= 2 {
            out[2] = 1.0 - 2.0 * x + x * x / 2.0;
        }
        if self.degree >= 3 {
            out[3] = 1.0 - 3.0 * x + 3.0 * x * x / 2.0 - x * x * x / 6.0;
        }
        if self.degree >= 4 {
            out[4] = 1.0 - 4.0 * x + 3.0 * x * x - 2.0 * x * x * x / 3.0 + x * x * x * x / 24.0;
        }
    }
}

/// LSMC configuration.
#[derive(Clone, Debug)]
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
    /// Create a new LSMC configuration.
    pub fn new(num_paths: usize, exercise_dates: Vec<usize>) -> Self {
        Self {
            num_paths,
            seed: 42,
            exercise_dates,
            use_parallel: false, // LSMC is complex, default to serial
        }
    }

    /// Set random seed.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
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
        B: BasisFunctions,
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
            stats.ci_95(),
            values.len(),
        )
        .with_std_dev(stats.std_dev());

        Ok(MoneyEstimate::from_estimate(estimate, currency))
    }

    /// Generate Monte Carlo paths.
    fn generate_paths(
        &self,
        process: &GbmProcess,
        initial_spot: f64,
        time_to_maturity: f64,
        num_steps: usize,
    ) -> Result<Vec<Vec<f64>>> {
        let time_grid = TimeGrid::uniform(time_to_maturity, num_steps)?;
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

    /// Perform backward induction with regression.
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
        B: BasisFunctions,
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

        for &exercise_step in &sorted_exercise_dates {
            if exercise_step >= num_steps {
                continue;
            }

            let t = exercise_step as f64 * dt;

            // Collect in-the-money paths for regression
            let mut regression_x = Vec::new();
            let mut regression_y = Vec::new();
            let mut regression_indices = Vec::new();

            for (i, path) in paths.iter().enumerate() {
                let spot = path[exercise_step];
                let immediate = exercise.exercise_value(spot);

                // Only regress on ITM paths
                if immediate > 1e-6 {
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
                let continuation_values = self.regression(&regression_x, &regression_y, basis)?;

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
        }

        // Discount all cashflows to present
        let mut present_values = vec![0.0; num_paths];
        for i in 0..num_paths {
            present_values[i] = cashflows[i] * (-discount_rate * exercise_times[i]).exp();
        }

        Ok(present_values)
    }

    /// Perform least-squares regression.
    ///
    /// Fits continuation value = Σ β_i φ_i(S) using ordinary least squares.
    fn regression<B>(&self, x: &[f64], y: &[f64], basis: &B) -> Result<Vec<f64>>
    where
        B: BasisFunctions,
    {
        let n = x.len();
        let k = basis.num_basis();

        // Build design matrix X (n x k)
        let mut design = vec![0.0; n * k];
        let mut basis_vals = vec![0.0; k];

        for (i, &spot) in x.iter().enumerate() {
            basis.evaluate(spot, &mut basis_vals);
            for j in 0..k {
                design[i * k + j] = basis_vals[j];
            }
        }

        // Solve least squares using SVD (numerically stable for ill-conditioned systems)
        let coeffs = solve_least_squares(&design, y, n, k)?;

        // Predict continuation values
        let mut predictions = vec![0.0; n];
        for (i, &spot) in x.iter().enumerate() {
            basis.evaluate(spot, &mut basis_vals);
            let mut pred = 0.0;
            for j in 0..k {
                pred += coeffs[j] * basis_vals[j];
            }
            predictions[i] = pred;
        }

        Ok(predictions)
    }
}

#[cfg(test)]
mod tests {
    use crate::instruments::common::mc::process::gbm::GbmParams;
    use super::*;

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
        let config = LsmcConfig::new(1_000, exercise_dates).with_seed(42);
        let pricer = LsmcPricer::new(config);

        let gbm = GbmProcess::new(GbmParams::new(0.05, 0.0, 0.3));
        let put = AmericanPut { strike: 100.0 };
        let basis = PolynomialBasis::new(2);

        let result = pricer
            .price(&gbm, 100.0, 1.0, 100, &put, &basis, Currency::USD, 0.05)
            .unwrap();

        // American put should have positive value
        assert!(result.mean.amount() > 0.0);
        assert!(result.mean.amount() < 50.0); // Sanity check
    }


    #[test]
    fn test_lsmc_high_degree_polynomial() {
        // Test with degree-5 polynomial (can be ill-conditioned)
        // This tests QR robustness vs Cholesky
        let exercise_dates = vec![25, 50, 75, 100];
        let config = LsmcConfig::new(5_000, exercise_dates).with_seed(42);
        let pricer = LsmcPricer::new(config);

        let gbm = GbmProcess::new(GbmParams::new(0.05, 0.0, 0.3));
        let put = AmericanPut { strike: 100.0 };

        // High-degree polynomial basis (more prone to ill-conditioning)
        let basis = PolynomialBasis::new(5);

        let result = pricer.price(&gbm, 80.0, 1.0, 100, &put, &basis, Currency::USD, 0.05);

        // Should not panic or produce NaN
        assert!(result.is_ok());
        let price = result.unwrap();
        assert!(price.mean.amount().is_finite());
        assert!(price.mean.amount() > 0.0);

        println!("High-degree poly LSMC (deep ITM): {}", price.mean);
    }

    #[test]
    fn test_lsmc_extreme_spot_ranges() {
        // Test with paths spanning wide spot range (10 to 1000)
        // This can cause numerical issues with polynomial basis
        let exercise_dates = vec![50, 100];
        let config = LsmcConfig::new(5_000, exercise_dates).with_seed(123);
        let pricer = LsmcPricer::new(config);

        // High volatility to get wide spot range
        let gbm = GbmProcess::new(GbmParams::new(0.05, 0.0, 1.0));
        let put = AmericanPut { strike: 100.0 };
        let basis = PolynomialBasis::new(3);

        let result = pricer.price(&gbm, 100.0, 1.0, 100, &put, &basis, Currency::USD, 0.05);

        // Should remain stable even with extreme paths
        assert!(result.is_ok());
        let price = result.unwrap();
        assert!(price.mean.amount().is_finite());
        assert!(price.mean.amount() >= 0.0);

        println!("Extreme spot ranges LSMC: {}", price.mean);
    }

    #[test]
    fn test_lsmc_few_itm_paths() {
        // Deep OTM put with few ITM paths
        // Tests regression fallback when insufficient data
        let exercise_dates = vec![50, 100];
        let config = LsmcConfig::new(1_000, exercise_dates).with_seed(456);
        let pricer = LsmcPricer::new(config);

        // Low volatility, deep OTM
        let gbm = GbmProcess::new(GbmParams::new(0.05, 0.0, 0.05));
        let put = AmericanPut { strike: 50.0 };
        let basis = PolynomialBasis::new(2);

        // Start well above strike
        let result = pricer.price(&gbm, 150.0, 0.5, 100, &put, &basis, Currency::USD, 0.05);

        // Should handle gracefully (very small value expected)
        assert!(result.is_ok());
        let price = result.unwrap();
        assert!(price.mean.amount().is_finite());
        assert!(price.mean.amount() >= 0.0);
        assert!(price.mean.amount() < 0.1); // Should be near zero

        println!("Few ITM paths LSMC: {}", price.mean);
    }

}
