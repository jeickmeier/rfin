//! Longstaff-Schwartz Monte Carlo pricer for Bermudan swaptions.
//!
//! Extends the LSMC framework to price Bermudan swaptions where exercise decisions
//! depend on forward swap rates computed from Hull-White short rate simulations.

use crate::instruments::common::mc::discretization::exact_hw1f::ExactHullWhite1F;
use super::super::payoff::swaption::{BermudanSwaptionPayoff, SwaptionType};
use crate::instruments::common::mc::process::ou::HullWhite1FProcess;
use super::super::results::MoneyEstimate;
use crate::instruments::common::mc::rng::philox::PhiloxRng;
use crate::instruments::common::mc::stats::OnlineStats;
use crate::instruments::common::mc::time_grid::TimeGrid;
use crate::instruments::common::mc::traits::{Discretization, RandomStream};
use crate::instruments::common::mc::results::Estimate;
use super::lsmc::{BasisFunctions, LsmcConfig};
use super::swap_rate_utils::{ForwardSwapRate, HullWhiteBondPrice};
use super::lsq::solve_least_squares;
use finstack_core::currency::Currency;
use finstack_core::Result;

/// LSMC pricer for Bermudan swaptions.
///
/// Uses backward induction with least-squares regression, similar to equity LSMC,
/// but computes exercise values from forward swap rates instead of spot prices.
pub struct SwaptionLsmcPricer {
    config: LsmcConfig,
    hw_process: HullWhite1FProcess,
}

impl SwaptionLsmcPricer {
    /// Create a new swaption LSMC pricer.
    ///
    /// # Arguments
    ///
    /// * `config` - LSMC configuration (num_paths, exercise_dates, etc.)
    /// * `hw_process` - Hull-White 1F process for short rate simulation
    pub fn new(config: LsmcConfig, hw_process: HullWhite1FProcess) -> Self {
        Self { config, hw_process }
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
        let values = self.backward_induction_swaption(
            &paths,
            payoff,
            &exercise_steps,
            basis,
            dt,
            &discount_curve_fn,
        )?;

        // Step 4: Compute statistics
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

    /// Generate short rate paths using Hull-White process.
    fn generate_rate_paths(
        &self,
        initial_rate: f64,
        time_to_maturity: f64,
        num_steps: usize,
    ) -> Result<Vec<Vec<f64>>> {
        let time_grid = TimeGrid::uniform(time_to_maturity, num_steps)?;
        let disc = ExactHullWhite1F::new();
        let rng = PhiloxRng::new(self.config.seed);

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

    /// Perform backward induction for swaptions.
    #[allow(clippy::too_many_arguments)]
    fn backward_induction_swaption<B, F>(
        &self,
        paths: &[Vec<f64>],
        payoff: &BermudanSwaptionPayoff,
        exercise_steps: &[usize],
        basis: &B,
        dt: f64,
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

        for &exercise_step in &sorted_exercise_steps {
            if exercise_step >= paths[0].len() - 1 {
                continue;
            }

            let t = exercise_step as f64 * dt;

            // Collect ITM paths for regression
            let mut regression_x = Vec::new(); // Swap rates
            let mut regression_y = Vec::new(); // Discounted continuation values
            let mut regression_indices = Vec::new();

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
                    let discounted_cf =
                        cashflows[i] * discount_curve_fn(exercise_times[i]) / discount_factor;

                    regression_x.push(swap_rate);
                    regression_y.push(discounted_cf);
                    regression_indices.push(i);
                }
            }

            // Perform regression if we have enough ITM paths
            if regression_x.len() > basis.num_basis() + 10 {
                let continuation_values = self.regression(&regression_x, &regression_y, basis)?;

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

        // Discount all cashflows to present
        let mut present_values = vec![0.0; num_paths];
        let df_0 = discount_curve_fn(0.0);
        for i in 0..num_paths {
            let df_t = discount_curve_fn(exercise_times[i]);
            present_values[i] = cashflows[i] * df_t / df_0;
        }

        Ok(present_values)
    }

    /// Perform least-squares regression using robust SVD solver.
    ///
    /// Uses the same SVD-based regression as equity LSMC to avoid numerical
    /// instability from normal equations (X'X) which square the condition number.
    fn regression<B>(&self, x: &[f64], y: &[f64], basis: &B) -> Result<Vec<f64>>
    where
        B: BasisFunctions,
    {
        let n = x.len();
        let k = basis.num_basis();

        // Build design matrix
        let mut design = vec![0.0; n * k];
        let mut basis_vals = vec![0.0; k];

        for (i, &swap_rate) in x.iter().enumerate() {
            basis.evaluate(swap_rate, &mut basis_vals);
            for j in 0..k {
                design[i * k + j] = basis_vals[j];
            }
        }

        // Solve using SVD (numerically stable for ill-conditioned systems)
        let coeffs = solve_least_squares(&design, y, n, k)?;

        // Predict continuation values
        let mut predictions = vec![0.0; n];
        for (i, &swap_rate) in x.iter().enumerate() {
            basis.evaluate(swap_rate, &mut basis_vals);
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
    // Tests for swap rate utilities are now in swap_rate_utils.rs
    // This module focuses on testing the LSMC swaption pricer itself
}
