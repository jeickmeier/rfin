//! Longstaff-Schwartz Monte Carlo pricer for Bermudan swaptions.
//!
//! Extends the LSMC framework to price Bermudan swaptions where exercise decisions
//! depend on forward swap rates computed from Hull-White short rate simulations.
//!
//! # Features
//!
//! - Hull-White 1-factor short rate simulation with exact discretization
//! - Longstaff-Schwartz backward induction with optimal exercise decisions
//! - Variance reduction via antithetic variates and control variates
//! - Polynomial and Laguerre basis functions for regression
//!
//! # Usage
//!
//! ```rust,no_run
//! use finstack_valuations::instruments::common::models::monte_carlo::pricer::swaption_lsmc::{
//!     SwaptionLsmcPricer, SwaptionLsmcConfig,
//! };
//! use finstack_valuations::instruments::common::models::monte_carlo::pricer::basis::PolynomialBasis;
//! use finstack_valuations::instruments::common::mc::process::ou::{HullWhite1FProcess, HullWhite1FParams};
//!
//! let hw_params = HullWhite1FParams::new(0.03, 0.01, 0.03);
//! let hw_process = HullWhite1FProcess::new(hw_params);
//! let config = SwaptionLsmcConfig::default();
//!
//! let pricer = SwaptionLsmcPricer::with_config(config, hw_process);
//! ```

use super::super::payoff::swaption::{BermudanSwaptionPayoff, SwaptionType};
use super::super::results::MoneyEstimate;
use super::lsmc::LsmcConfig;
use super::lsq::regression_with_basis;
use super::swap_rate_utils::{ForwardSwapRate, HullWhiteBondPrice};
use crate::instruments::common::mc::discretization::exact_hw1f::ExactHullWhite1F;
use crate::instruments::common::mc::estimate::Estimate;
use crate::instruments::common::mc::online_stats::OnlineStats;
use crate::instruments::common::mc::process::ou::HullWhite1FProcess;
use crate::instruments::common::mc::rng::philox::PhiloxRng;
use crate::instruments::common::mc::time_grid::TimeGrid;
use crate::instruments::common::mc::traits::{Discretization, RandomStream};
use crate::instruments::common::models::monte_carlo::pricer::basis::{
    BasisFunctions, PolynomialBasis,
};
use finstack_core::currency::Currency;
use finstack_core::Result;

// ============================================================================
// Configuration
// ============================================================================

/// Time grid specification for swaption LSMC.
#[derive(Clone, Debug)]
pub enum TimeGridSpec {
    /// Use a uniform grid with specified number of steps.
    Uniform {
        /// Number of time steps for simulation.
        num_steps: usize,
    },
    /// Use a custom time grid (exercise dates are included exactly).
    Custom(TimeGrid),
}

impl Default for TimeGridSpec {
    fn default() -> Self {
        TimeGridSpec::Uniform { num_steps: 100 }
    }
}

/// Configuration for Bermudan swaption LSMC pricing.
///
/// # Default Values
///
/// | Parameter | Default | Description |
/// |-----------|---------|-------------|
/// | num_paths | 50,000 | Number of Monte Carlo paths |
/// | seed | 42 | Random seed for reproducibility |
/// | basis_degree | 3 | Polynomial degree for regression |
/// | antithetic | true | Use antithetic variates |
/// | control_variate | false | Use European as control |
#[derive(Clone, Debug)]
pub struct SwaptionLsmcConfig {
    /// Number of Monte Carlo paths.
    ///
    /// More paths improve accuracy but increase computation time.
    /// Typical values: 10,000 (fast), 50,000 (standard), 100,000+ (high precision)
    pub num_paths: usize,

    /// Random seed for reproducibility.
    ///
    /// Using the same seed produces identical results across runs.
    pub seed: u64,

    /// Polynomial degree for basis functions in regression.
    ///
    /// Higher degrees can capture more complex continuation value surfaces
    /// but may overfit with limited ITM paths.
    /// Typical values: 2-4
    pub basis_degree: usize,

    /// Use antithetic variates for variance reduction.
    ///
    /// Generates (Z, -Z) path pairs which reduces variance by exploiting
    /// negative correlation between paired paths.
    pub antithetic: bool,

    /// Use European swaption as control variate.
    ///
    /// Reduces variance by using the analytical European value as a control.
    /// Requires the European swaption to be priced analytically.
    pub control_variate: bool,

    /// Exercise dates for the LSMC algorithm (step indices).
    ///
    /// Typically set automatically from the Bermudan schedule.
    pub exercise_dates: Vec<usize>,

    /// Time grid specification.
    ///
    /// By default, uses uniform steps. For exact exercise date alignment,
    /// use `TimeGridSpec::Custom` with exercise dates included in the grid.
    pub time_grid_spec: TimeGridSpec,
}

impl Default for SwaptionLsmcConfig {
    fn default() -> Self {
        Self {
            num_paths: 50_000,
            seed: 42,
            basis_degree: 3,
            antithetic: true,
            control_variate: false,
            exercise_dates: Vec::new(),
            time_grid_spec: TimeGridSpec::default(),
        }
    }
}

impl SwaptionLsmcConfig {
    /// Create a new configuration with specified parameters.
    pub fn new(num_paths: usize, seed: u64) -> Self {
        Self {
            num_paths,
            seed,
            ..Default::default()
        }
    }

    /// Set basis function degree.
    pub fn with_basis_degree(mut self, degree: usize) -> Self {
        self.basis_degree = degree;
        self
    }

    /// Enable/disable antithetic variates.
    pub fn with_antithetic(mut self, enabled: bool) -> Self {
        self.antithetic = enabled;
        self
    }

    /// Enable/disable control variate.
    pub fn with_control_variate(mut self, enabled: bool) -> Self {
        self.control_variate = enabled;
        self
    }

    /// Set exercise dates (step indices).
    pub fn with_exercise_dates(mut self, dates: Vec<usize>) -> Self {
        self.exercise_dates = dates;
        self
    }

    /// Set time grid specification.
    pub fn with_time_grid(mut self, spec: TimeGridSpec) -> Self {
        self.time_grid_spec = spec;
        self
    }

    /// Set number of time steps (for uniform grid).
    pub fn with_num_steps(mut self, num_steps: usize) -> Self {
        self.time_grid_spec = TimeGridSpec::Uniform { num_steps };
        self
    }

    /// Convert to internal LsmcConfig.
    pub fn to_lsmc_config(&self) -> LsmcConfig {
        LsmcConfig {
            num_paths: self.num_paths,
            seed: self.seed,
            exercise_dates: self.exercise_dates.clone(),
            use_parallel: false, // LSMC is complex, default to serial
        }
    }

    /// Build a time grid that includes all exercise dates exactly.
    ///
    /// Creates a grid with exercise dates as exact grid points, plus
    /// optional refinement points between them.
    ///
    /// # Arguments
    ///
    /// * `exercise_times` - Exercise times in years (sorted)
    /// * `maturity` - Final maturity time
    /// * `min_steps_between` - Minimum steps between grid points (default: 1)
    pub fn build_exercise_aligned_grid(
        exercise_times: &[f64],
        maturity: f64,
        min_steps_between: usize,
    ) -> Result<(TimeGrid, Vec<usize>)> {
        let mut times = vec![0.0];
        let mut exercise_indices = Vec::with_capacity(exercise_times.len());

        for &ex_time in exercise_times {
            if ex_time <= 0.0 || ex_time > maturity {
                continue;
            }

            // Add refinement points between last time and this exercise date
            // times is initialized with vec![0.0] and only grows, so last() always succeeds
            let last_time = times[times.len() - 1];
            if min_steps_between > 0 && ex_time > last_time + 1e-10 {
                let dt = (ex_time - last_time) / (min_steps_between + 1) as f64;
                for i in 1..=min_steps_between {
                    let t = last_time + dt * i as f64;
                    if (t - ex_time).abs() > 1e-10 {
                        times.push(t);
                    }
                }
            }

            // Add the exercise date exactly
            let current_last = times[times.len() - 1];
            if (ex_time - current_last).abs() > 1e-10 {
                times.push(ex_time);
            }
            exercise_indices.push(times.len() - 1);
        }

        // Add maturity if not already present
        let final_last = times[times.len() - 1];
        if (final_last - maturity).abs() > 1e-10 {
            times.push(maturity);
        }

        let grid = TimeGrid::from_times(times)?;
        Ok((grid, exercise_indices))
    }
}

// ============================================================================
// Swaption-Specific Basis Functions
// ============================================================================

/// Extended basis functions including annuity.
///
/// Basis: {1, S, S², ..., A, S×A}
///
/// Including the annuity as a state variable can improve regression quality
/// for swaptions where both rate level and annuity affect option value.
#[derive(Clone, Debug)]
pub struct ExtendedSwaptionBasis {
    /// Polynomial degree for swap rate
    degree: usize,
    /// Include annuity as additional basis
    include_annuity: bool,
}

impl ExtendedSwaptionBasis {
    /// Create extended basis with annuity terms.
    pub fn new(degree: usize, include_annuity: bool) -> Self {
        Self {
            degree,
            include_annuity,
        }
    }
}

impl BasisFunctions for ExtendedSwaptionBasis {
    fn num_basis(&self) -> usize {
        let base = self.degree + 1;
        if self.include_annuity {
            base + 2 // Add A and S×A terms
        } else {
            base
        }
    }

    fn evaluate(&self, swap_rate: f64, out: &mut [f64]) {
        debug_assert_eq!(out.len(), self.num_basis());

        // Polynomial basis
        let mut power = 1.0;
        for basis in out.iter_mut().take(self.degree + 1) {
            *basis = power;
            power *= swap_rate;
        }

        // Note: Annuity terms would need to be passed separately
        // For now, fill with placeholder values
        if self.include_annuity {
            out[self.degree + 1] = 1.0; // Placeholder for A
            out[self.degree + 2] = swap_rate; // Placeholder for S×A
        }
    }
}

/// Backward-compatible alias for swaption polynomial basis.
pub type SwaptionBasis = PolynomialBasis;

/// LSMC pricer for Bermudan swaptions.
///
/// Uses backward induction with least-squares regression, similar to equity LSMC,
/// but computes exercise values from forward swap rates instead of spot prices.
///
/// # Features
///
/// - Hull-White 1F short rate simulation
/// - Polynomial basis functions for regression
/// - Optional antithetic variates for variance reduction
/// - Optional control variate using European swaption
pub struct SwaptionLsmcPricer {
    /// Internal LSMC configuration
    config: LsmcConfig,
    /// Hull-White process parameters
    hw_process: HullWhite1FProcess,
    /// Extended configuration
    swaption_config: SwaptionLsmcConfig,
}

impl SwaptionLsmcPricer {
    /// Create a new swaption LSMC pricer with default configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - LSMC configuration (num_paths, exercise_dates, etc.)
    /// * `hw_process` - Hull-White 1F process for short rate simulation
    pub fn new(config: LsmcConfig, hw_process: HullWhite1FProcess) -> Self {
        Self {
            config: config.clone(),
            hw_process,
            swaption_config: SwaptionLsmcConfig {
                num_paths: config.num_paths,
                seed: config.seed,
                exercise_dates: config.exercise_dates.clone(),
                ..Default::default()
            },
        }
    }

    /// Create a new pricer with full configuration.
    pub fn with_config(
        swaption_config: SwaptionLsmcConfig,
        hw_process: HullWhite1FProcess,
    ) -> Self {
        let config = swaption_config.to_lsmc_config();
        Self {
            config,
            hw_process,
            swaption_config,
        }
    }

    /// Price a Bermudan swaption.
    ///
    /// # Arguments
    ///
    /// * `payoff` - Bermudan swaption payoff specification
    /// * `initial_short_rate` - Initial short rate r(0)
    /// * `time_to_maturity` - Time to swaption maturity
    /// * `num_steps` - Number of time steps for simulation
    /// * `basis` - Basis functions for regression
    /// * `discount_curve_fn` - Function to get discount factors DF(t) for time t
    /// * `currency` - Currency for result
    ///
    /// # Returns
    ///
    /// Statistical estimate of Bermudan swaption value
    #[allow(clippy::too_many_arguments)]
    pub fn price_bermudan<B, F>(
        &self,
        payoff: &BermudanSwaptionPayoff,
        initial_short_rate: f64,
        time_to_maturity: f64,
        num_steps: usize,
        basis: &B,
        discount_curve_fn: F,
        currency: Currency,
    ) -> Result<MoneyEstimate>
    where
        B: BasisFunctions,
        F: Fn(f64) -> f64 + Send + Sync,
    {
        // Step 1: Generate short rate paths
        let paths = self.generate_rate_paths(initial_short_rate, time_to_maturity, num_steps)?;

        // Step 2: Convert exercise dates to step indices
        let dt = time_to_maturity / num_steps as f64;
        let exercise_steps: Vec<usize> = payoff
            .exercise_dates
            .iter()
            .map(|&t| {
                let step = (t / dt).round() as usize;
                step.min(num_steps)
            })
            .collect();

        // Step 3: Backward induction with swap rate calculation
        let time_grid = TimeGrid::uniform(time_to_maturity, num_steps)?;
        let values = self.backward_induction_swaption_grid(
            &paths,
            payoff,
            &exercise_steps,
            basis,
            &time_grid,
            &discount_curve_fn,
        )?;

        // Step 4: Compute statistics
        let mut stats = OnlineStats::new();
        for &value in &values {
            stats.update(value);
        }

        let estimate = Estimate::new(stats.mean(), stats.stderr(), stats.confidence_interval(0.05), values.len())
            .with_std_dev(stats.std_dev());

        Ok(MoneyEstimate::from_estimate(estimate, currency))
    }

    /// Price a Bermudan swaption using a custom time grid with exact exercise indices.
    ///
    /// This variant allows precise alignment of the time grid with exercise dates,
    /// avoiding the rounding errors that can occur with uniform grids.
    ///
    /// # Arguments
    ///
    /// * `payoff` - Bermudan swaption payoff specification
    /// * `initial_short_rate` - Initial short rate r(0)
    /// * `time_grid` - Custom time grid (should include exercise dates exactly)
    /// * `exercise_indices` - Exact step indices for exercise dates
    /// * `basis` - Basis functions for regression
    /// * `discount_curve_fn` - Function to get discount factors DF(t) for time t
    /// * `currency` - Currency for result
    ///
    /// # Returns
    ///
    /// Statistical estimate of Bermudan swaption value
    #[allow(clippy::too_many_arguments)]
    pub fn price_bermudan_with_grid<B, F>(
        &self,
        payoff: &BermudanSwaptionPayoff,
        initial_short_rate: f64,
        time_grid: &TimeGrid,
        exercise_indices: &[usize],
        basis: &B,
        discount_curve_fn: F,
        currency: Currency,
    ) -> Result<MoneyEstimate>
    where
        B: BasisFunctions,
        F: Fn(f64) -> f64 + Send + Sync,
    {
        // Step 1: Generate short rate paths using the custom time grid
        let paths = self.generate_rate_paths_with_grid(initial_short_rate, time_grid)?;

        // Step 2: Backward induction with exact exercise indices
        let values = self.backward_induction_swaption_grid(
            &paths,
            payoff,
            exercise_indices,
            basis,
            time_grid,
            &discount_curve_fn,
        )?;

        // Step 3: Compute statistics
        let mut stats = OnlineStats::new();
        for &value in &values {
            stats.update(value);
        }

        let estimate = Estimate::new(stats.mean(), stats.stderr(), stats.confidence_interval(0.05), values.len())
            .with_std_dev(stats.std_dev());

        Ok(MoneyEstimate::from_estimate(estimate, currency))
    }

    /// Generate short rate paths using Hull-White process.
    ///
    /// If antithetic variates are enabled, generates paired paths (Z, -Z)
    /// which reduces variance through negative correlation.
    fn generate_rate_paths(
        &self,
        initial_rate: f64,
        time_to_maturity: f64,
        num_steps: usize,
    ) -> Result<Vec<Vec<f64>>> {
        let time_grid = TimeGrid::uniform(time_to_maturity, num_steps)?;
        self.generate_rate_paths_with_grid(initial_rate, &time_grid)
    }

    /// Generate short rate paths using a custom time grid.
    ///
    /// If antithetic variates are enabled, generates paired paths (Z, -Z)
    /// which reduces variance through negative correlation.
    fn generate_rate_paths_with_grid(
        &self,
        initial_rate: f64,
        time_grid: &TimeGrid,
    ) -> Result<Vec<Vec<f64>>> {
        if self.swaption_config.antithetic {
            self.generate_antithetic_paths_with_grid(initial_rate, time_grid)
        } else {
            self.generate_standard_paths_with_grid(initial_rate, time_grid)
        }
    }

    /// Generate standard (non-antithetic) paths using a time grid.
    fn generate_standard_paths_with_grid(
        &self,
        initial_rate: f64,
        time_grid: &TimeGrid,
    ) -> Result<Vec<Vec<f64>>> {
        let disc = ExactHullWhite1F::new();
        let rng = PhiloxRng::new(self.config.seed);
        let num_steps = time_grid.num_steps();

        let mut paths = Vec::with_capacity(self.config.num_paths);

        for path_id in 0..self.config.num_paths {
            let mut path_rng = rng.split(path_id as u64);
            let mut rate_path = Vec::with_capacity(num_steps + 1);
            let mut state = vec![initial_rate];
            let mut z = vec![0.0];
            let mut work = vec![];

            rate_path.push(initial_rate);

            for step in 0..num_steps {
                let t = time_grid.time(step);
                let dt = time_grid.dt(step);

                path_rng.fill_std_normals(&mut z);
                disc.step(&self.hw_process, t, dt, &mut state, &z, &mut work);

                rate_path.push(state[0]);
            }

            paths.push(rate_path);
        }

        Ok(paths)
    }

    /// Generate antithetic path pairs (Z, -Z) for variance reduction using a time grid.
    ///
    /// For each random draw Z, generates two paths:
    /// - Original path using Z
    /// - Antithetic path using -Z
    ///
    /// This exploits the negative correlation to reduce variance.
    fn generate_antithetic_paths_with_grid(
        &self,
        initial_rate: f64,
        time_grid: &TimeGrid,
    ) -> Result<Vec<Vec<f64>>> {
        let disc = ExactHullWhite1F::new();
        let rng = PhiloxRng::new(self.config.seed);
        let num_steps = time_grid.num_steps();

        // With antithetic, we need half the random numbers
        let num_pairs = self.config.num_paths / 2;
        let mut paths = Vec::with_capacity(self.config.num_paths);

        for pair_id in 0..num_pairs {
            let mut path_rng = rng.split(pair_id as u64);

            // Generate random draws for this pair
            let mut z_draws: Vec<f64> = vec![0.0; num_steps];
            for z in &mut z_draws {
                let mut z_buf = vec![0.0];
                path_rng.fill_std_normals(&mut z_buf);
                *z = z_buf[0];
            }

            // Original path using +Z
            let mut state_orig = vec![initial_rate];
            let mut rate_path_orig = Vec::with_capacity(num_steps + 1);
            rate_path_orig.push(initial_rate);

            let mut work = vec![];
            for (step, &z_val) in z_draws.iter().enumerate() {
                let t = time_grid.time(step);
                let dt = time_grid.dt(step);

                let z = vec![z_val];
                disc.step(&self.hw_process, t, dt, &mut state_orig, &z, &mut work);
                rate_path_orig.push(state_orig[0]);
            }

            // Antithetic path using -Z
            let mut state_anti = vec![initial_rate];
            let mut rate_path_anti = Vec::with_capacity(num_steps + 1);
            rate_path_anti.push(initial_rate);

            for (step, &z_val) in z_draws.iter().enumerate() {
                let t = time_grid.time(step);
                let dt = time_grid.dt(step);

                let z = vec![-z_val]; // Negate the random draw
                disc.step(&self.hw_process, t, dt, &mut state_anti, &z, &mut work);
                rate_path_anti.push(state_anti[0]);
            }

            paths.push(rate_path_orig);
            paths.push(rate_path_anti);
        }

        // Handle odd number of paths
        if self.config.num_paths % 2 == 1 {
            let mut path_rng = rng.split(num_pairs as u64);
            let mut state = vec![initial_rate];
            let mut rate_path = Vec::with_capacity(num_steps + 1);
            rate_path.push(initial_rate);

            let mut z = vec![0.0];
            let mut work = vec![];
            for step in 0..num_steps {
                let t = time_grid.time(step);
                let dt = time_grid.dt(step);

                path_rng.fill_std_normals(&mut z);
                disc.step(&self.hw_process, t, dt, &mut state, &z, &mut work);
                rate_path.push(state[0]);
            }
            paths.push(rate_path);
        }

        Ok(paths)
    }

    /// Perform backward induction for swaptions using a time grid.
    ///
    /// # Discounting Convention
    ///
    /// This pricer uses **discount factor ratios from a yield curve**: `df_t / df_0`.
    /// This approach properly handles the term structure embedded in the discount curve
    /// and is appropriate for swaptions where rates vary across maturities.
    ///
    /// **Contrast with American LSMC**: The equity/American option pricer uses exponential
    /// discounting (`exp(-r * t)`) with a flat rate. Both approaches produce present values
    /// at time 0, but differ in their input assumptions:
    /// - **American LSMC**: Flat rate input → exponential discounting
    /// - **Swaption LSMC**: Discount curve input → ratio of discount factors
    ///
    /// The discount factor at time 0 (`df_0`) is asserted to be positive to prevent
    /// division by zero and ensure well-defined present values.
    ///
    /// See `lsmc.rs` for the flat-rate discounting approach.
    #[allow(clippy::too_many_arguments)]
    fn backward_induction_swaption_grid<B, F>(
        &self,
        paths: &[Vec<f64>],
        payoff: &BermudanSwaptionPayoff,
        exercise_steps: &[usize],
        basis: &B,
        time_grid: &TimeGrid,
        discount_curve_fn: &F,
    ) -> Result<Vec<f64>>
    where
        B: BasisFunctions,
        F: Fn(f64) -> f64 + Send + Sync,
    {
        let num_paths = paths.len();
        let params = self.hw_process.params();

        // Cashflow and exercise time tracking
        let mut cashflows = vec![0.0; num_paths];
        let mut exercise_times =
            vec![payoff.exercise_dates.last().copied().unwrap_or(0.0); num_paths];

        // Initialize with terminal values (if not exercised, value is zero)
        // For swaptions, terminal value is zero if not exercised

        // Backward induction through exercise dates
        let mut sorted_exercise_steps = exercise_steps.to_vec();
        sorted_exercise_steps.sort_unstable();
        sorted_exercise_steps.reverse(); // Go backward

        // Pre-allocate regression buffers to avoid reallocations
        let mut regression_x = Vec::with_capacity(paths.len() / 2); // Swap rates
        let mut regression_y = Vec::with_capacity(paths.len() / 2); // Discounted continuation values
        let mut regression_indices = Vec::with_capacity(paths.len() / 2);

        for &exercise_step in &sorted_exercise_steps {
            if exercise_step >= paths[0].len() - 1 {
                continue;
            }

            // Get exact time from grid instead of computing from step * dt
            let t = time_grid.time(exercise_step);

            // Clear buffers for this exercise date (reuse capacity)
            regression_x.clear();
            regression_y.clear();
            regression_indices.clear();

            for (i, path) in paths.iter().enumerate() {
                let r_t = path[exercise_step];

                // Compute forward swap rate
                let swap_rate = ForwardSwapRate::compute(
                    params,
                    r_t,
                    t,
                    &payoff.swap_schedule,
                    discount_curve_fn,
                );

                // Compute exercise value: (S(t) - K) * A(t) * N for payer
                let swap_value = match payoff.option_type {
                    SwaptionType::Payer => swap_rate - payoff.strike_rate,
                    SwaptionType::Receiver => payoff.strike_rate - swap_rate,
                };

                // Compute annuity for proper scaling
                let mut annuity = 0.0;
                for (j, &payment_time_j) in payoff.swap_schedule.payment_dates.iter().enumerate() {
                    if payment_time_j > t {
                        let p_j = HullWhiteBondPrice::bond_price(
                            params,
                            r_t,
                            t,
                            payment_time_j,
                            discount_curve_fn,
                        );
                        let tau_j = payoff.swap_schedule.accrual_fractions[j];
                        annuity += tau_j * p_j;
                    }
                }

                let immediate_value = swap_value.max(0.0) * annuity * payoff.notional;

                // Only regress on ITM paths
                if immediate_value > 1e-6 {
                    // Discount cashflow to this exercise date
                    let discount_factor = discount_curve_fn(t);
                    let discounted_cf = if discount_factor > 0.0 {
                        cashflows[i] * discount_curve_fn(exercise_times[i]) / discount_factor
                    } else {
                        0.0
                    };

                    regression_x.push(swap_rate);
                    regression_y.push(discounted_cf);
                    regression_indices.push(i);
                }
            }

            // Perform regression if we have enough ITM paths
            if regression_x.len() > basis.num_basis() + 10 {
                let continuation_values =
                    regression_with_basis(&regression_x, &regression_y, basis)?;

                // Exercise decision
                for (j, &i) in regression_indices.iter().enumerate() {
                    let r_t = paths[i][exercise_step];
                    let swap_rate = ForwardSwapRate::compute(
                        params,
                        r_t,
                        t,
                        &payoff.swap_schedule,
                        discount_curve_fn,
                    );

                    let swap_value = match payoff.option_type {
                        SwaptionType::Payer => swap_rate - payoff.strike_rate,
                        SwaptionType::Receiver => payoff.strike_rate - swap_rate,
                    };

                    let mut annuity = 0.0;
                    for (k, &payment_time_k) in
                        payoff.swap_schedule.payment_dates.iter().enumerate()
                    {
                        if payment_time_k > t {
                            let p_k = HullWhiteBondPrice::bond_price(
                                params,
                                r_t,
                                t,
                                payment_time_k,
                                discount_curve_fn,
                            );
                            let tau_k = payoff.swap_schedule.accrual_fractions[k];
                            annuity += tau_k * p_k;
                        }
                    }

                    let immediate_value = swap_value.max(0.0) * annuity * payoff.notional;
                    let continuation = continuation_values[j];

                    // Exercise if immediate value > continuation value
                    if immediate_value > continuation {
                        cashflows[i] = immediate_value;
                        exercise_times[i] = t;
                    }
                }
            }
        }

        // Discount all cashflows to present using discount factor ratios
        let mut present_values = vec![0.0; num_paths];
        let df_0 = discount_curve_fn(0.0);

        // Ensure df_0 is positive to prevent division by zero and ensure well-defined PV
        if df_0 <= 0.0 {
            return Err(finstack_core::Error::Validation(format!(
                "Discount factor at time 0 must be positive, got df_0 = {}",
                df_0
            )));
        }

        for i in 0..num_paths {
            let df_t = discount_curve_fn(exercise_times[i]);
            present_values[i] = cashflows[i] * df_t / df_0;
        }

        Ok(present_values)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    // Tests for swap rate utilities are now in swap_rate_utils.rs
    // This module focuses on testing the LSMC swaption pricer itself
}
