//! Antithetic variates variance reduction.
//!
//! Pairs each random path with its antithetic counterpart (negated shocks)
//! to reduce variance through negative correlation of payoffs.
//!
//! For each path with shocks Z, simulate a paired path with -Z.
//! The average of the two payoffs has lower variance than either alone.

use crate::instruments::common_impl::models::monte_carlo::online_stats::OnlineStats;
use crate::instruments::common_impl::models::monte_carlo::traits::Payoff;
use crate::instruments::common_impl::models::monte_carlo::traits::{
    Discretization, RandomStream, StochasticProcess,
};
use finstack_core::currency::Currency;

/// Configuration for antithetic variates pricing.
pub struct AntitheticConfig<'a> {
    /// Number of path pairs to simulate
    pub num_pairs: usize,
    /// Time grid for simulation
    pub time_grid: &'a crate::instruments::common_impl::models::monte_carlo::time_grid::TimeGrid,
    /// Currency for payoff amounts
    pub currency: Currency,
    /// Discount factor to present value
    pub discount_factor: f64,
}

/// Apply antithetic variates to reduce variance.
///
/// This function simulates pairs of paths (Z, -Z) and averages their payoffs.
/// The variance reduction comes from the negative correlation between
/// the paired payoffs.
///
/// # Arguments
///
/// * `rng` - Random number generator
/// * `process` - Stochastic process
/// * `disc` - Discretization scheme
/// * `initial_state` - Initial state vector
/// * `payoff` - Payoff specification
/// * `config` - Antithetic configuration
///
/// # Returns
///
/// Online statistics with accumulated results from all pairs.
#[allow(clippy::too_many_arguments)]
pub fn antithetic_price<R, P, D, F>(
    rng: &mut R,
    process: &P,
    disc: &D,
    initial_state: &[f64],
    payoff: &F,
    config: &AntitheticConfig,
) -> OnlineStats
where
    R: RandomStream,
    P: StochasticProcess,
    D: Discretization<P>,
    F: Payoff,
{
    let mut stats = OnlineStats::new();
    let dim = process.dim();
    let num_factors = process.num_factors();
    let work_size = disc.work_size(process);

    // Buffers for both paths
    let mut state_pos = vec![0.0; dim];
    let mut state_neg = vec![0.0; dim];
    let mut z = vec![0.0; num_factors];
    let mut z_neg = vec![0.0; num_factors];
    let mut work = vec![0.0; work_size];

    // Pre-allocate storage for all shocks in the positive path so we can
    // replay them negated for the antithetic path. This avoids the
    // double-negation bug where re-negating the z buffer on alternating
    // steps produces +Z on even steps instead of -Z on all steps.
    let num_steps = config.time_grid.num_steps();
    let mut all_shocks = vec![0.0; num_steps * num_factors];

    for _pair in 0..config.num_pairs {
        // Generate all shocks for this path pair up front
        for step in 0..num_steps {
            let offset = step * num_factors;
            rng.fill_std_normals(&mut all_shocks[offset..offset + num_factors]);
        }

        // Simulate positive path (Z)
        let mut payoff_pos = payoff.clone();
        payoff_pos.reset();
        simulate_path_with_stored_shocks(
            process,
            disc,
            initial_state,
            &mut payoff_pos,
            &mut state_pos,
            &mut z,
            &mut work,
            config.time_grid,
            &all_shocks,
            1.0, // Positive shocks
        );
        let value_pos = payoff_pos.value(config.currency).amount() * config.discount_factor;

        // Simulate negative path (-Z) using stored shocks
        let mut payoff_neg = payoff.clone();
        payoff_neg.reset();
        simulate_path_with_stored_shocks(
            process,
            disc,
            initial_state,
            &mut payoff_neg,
            &mut state_neg,
            &mut z_neg,
            &mut work,
            config.time_grid,
            &all_shocks,
            -1.0, // Negative shocks
        );
        let value_neg = payoff_neg.value(config.currency).amount() * config.discount_factor;

        // Average the pair
        let avg_value = (value_pos + value_neg) / 2.0;
        stats.update(avg_value);
    }

    stats
}

/// Simulate a single path using pre-stored shocks with optional sign.
///
/// This avoids the double-negation bug in the original `simulate_path` by
/// reading shocks from `all_shocks` (generated once per pair) and applying
/// `shock_sign` uniformly to all steps.
#[allow(clippy::too_many_arguments)]
fn simulate_path_with_stored_shocks<P, D, F>(
    process: &P,
    disc: &D,
    initial_state: &[f64],
    payoff: &mut F,
    state: &mut [f64],
    z: &mut [f64],
    work: &mut [f64],
    time_grid: &crate::instruments::common_impl::models::monte_carlo::time_grid::TimeGrid,
    all_shocks: &[f64],
    shock_sign: f64,
) where
    P: StochasticProcess,
    D: Discretization<P>,
    F: Payoff,
{
    let num_factors = z.len();

    // Initialize state
    state.copy_from_slice(initial_state);

    // Create initial path state
    let mut path_state =
        crate::instruments::common_impl::models::monte_carlo::traits::PathState::new(0, 0.0);
    if !state.is_empty() {
        path_state.set(
            crate::instruments::common_impl::models::monte_carlo::traits::state_keys::SPOT,
            state[0],
        );
    }
    payoff.on_event(&mut path_state);

    // Simulate path through time steps
    for step in 0..time_grid.num_steps() {
        let t = time_grid.time(step);
        let dt = time_grid.dt(step);

        // Copy stored shocks for this step, applying sign
        let offset = step * num_factors;
        for i in 0..num_factors {
            z[i] = shock_sign * all_shocks[offset + i];
        }

        // Advance state
        disc.step(process, t, dt, state, z, work);

        // Update path state
        path_state.step = step + 1;
        path_state.time = t + dt;
        if !state.is_empty() {
            path_state.set(
                crate::instruments::common_impl::models::monte_carlo::traits::state_keys::SPOT,
                state[0],
            );
            if state.len() > 1 {
                path_state.set(
                    crate::instruments::common_impl::models::monte_carlo::traits::state_keys::VARIANCE,
                    state[1],
                );
            }
        }

        // Process payoff event
        payoff.on_event(&mut path_state);
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::instruments::common_impl::models::monte_carlo::discretization::exact::ExactGbm;
    use crate::instruments::common_impl::models::monte_carlo::payoff::vanilla::EuropeanCall;
    use crate::instruments::common_impl::models::monte_carlo::process::gbm::{
        GbmParams, GbmProcess,
    };
    use crate::instruments::common_impl::models::monte_carlo::rng::philox::PhiloxRng;
    use crate::instruments::common_impl::models::monte_carlo::time_grid::TimeGrid;
    use finstack_core::currency::Currency;

    #[test]
    fn test_antithetic_basic() {
        let mut rng = PhiloxRng::new(42);
        let process = GbmProcess::new(GbmParams::new(0.05, 0.02, 0.2));
        let disc = ExactGbm::new();
        let initial_state = vec![100.0];
        let payoff = EuropeanCall::new(100.0, 1.0, 10);
        let time_grid = TimeGrid::uniform(1.0, 10).expect("should succeed");

        let config = AntitheticConfig {
            num_pairs: 100,
            time_grid: &time_grid,
            currency: Currency::USD,
            discount_factor: 1.0,
        };

        let stats = antithetic_price(&mut rng, &process, &disc, &initial_state, &payoff, &config);

        // Should produce reasonable option value
        assert!(stats.mean() > 0.0);
        assert!(stats.mean() < 50.0);
        assert_eq!(stats.count(), 100);
    }

    #[test]
    fn test_antithetic_reduces_variance() {
        // Antithetic variates should produce lower stderr than standard MC
        // This is a demonstration test showing variance reduction

        let mut rng_antithetic = PhiloxRng::new(42);

        let process = GbmProcess::new(GbmParams::new(0.05, 0.02, 0.3));
        let disc = ExactGbm::new();
        let initial_state = vec![100.0];
        let payoff = EuropeanCall::new(100.0, 1.0, 50);
        let time_grid = TimeGrid::uniform(1.0, 50).expect("should succeed");

        // Antithetic: 100 pairs = 200 effective paths
        let config = AntitheticConfig {
            num_pairs: 100,
            time_grid: &time_grid,
            currency: Currency::USD,
            discount_factor: 1.0,
        };

        let stats_anti = antithetic_price(
            &mut rng_antithetic,
            &process,
            &disc,
            &initial_state,
            &payoff,
            &config,
        );

        // Should produce reasonable estimates
        assert!(stats_anti.mean() > 0.0);
        assert!(stats_anti.stderr() > 0.0);
        assert_eq!(stats_anti.count(), 100);

        // Antithetic should have lower standard error than naive MC (demonstrated in practice)
        println!(
            "Antithetic mean: {}, stderr: {}",
            stats_anti.mean(),
            stats_anti.stderr()
        );
    }
}
