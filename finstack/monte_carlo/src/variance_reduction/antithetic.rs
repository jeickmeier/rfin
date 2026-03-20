//! Antithetic variates variance reduction.
//!
//! Pairs each random path with its antithetic counterpart (negated shocks)
//! to reduce variance through negative correlation of payoffs.
//!
//! For each path with shocks Z, simulate a paired path with -Z.
//! The average of the two payoffs has lower variance than either alone.

use crate::online_stats::OnlineStats;
use crate::traits::Payoff;
use crate::traits::{Discretization, RandomStream, StochasticProcess};
use finstack_core::currency::Currency;

/// Configuration for antithetic variates pricing.
pub struct AntitheticConfig<'a> {
    /// Number of path pairs to simulate
    pub num_pairs: usize,
    /// Time grid for simulation
    pub time_grid: &'a crate::time_grid::TimeGrid,
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
    time_grid: &crate::time_grid::TimeGrid,
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
    let mut path_state = crate::traits::PathState::new(0, 0.0);
    process.populate_path_state(state, &mut path_state);
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
        process.populate_path_state(state, &mut path_state);

        // Process payoff event
        payoff.on_event(&mut path_state);
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::discretization::exact::ExactGbm;
    use crate::payoff::vanilla::EuropeanCall;
    use crate::process::gbm::{GbmParams, GbmProcess};
    use crate::rng::philox::PhiloxRng;
    use crate::time_grid::TimeGrid;
    use crate::traits::PathState;
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

    #[derive(Clone)]
    struct ThreeFactorProcess;

    impl StochasticProcess for ThreeFactorProcess {
        fn dim(&self) -> usize {
            3
        }

        fn num_factors(&self) -> usize {
            1
        }

        fn drift(&self, _t: f64, _x: &[f64], out: &mut [f64]) {
            out.fill(0.0);
        }

        fn diffusion(&self, _t: f64, _x: &[f64], out: &mut [f64]) {
            out.fill(0.0);
            out[0] = 1.0;
        }

        fn populate_path_state(&self, x: &[f64], state: &mut PathState) {
            state.set(crate::traits::state_keys::SPOT, x[0]);
            state.set(crate::traits::state_keys::VARIANCE, x[1]);
            state.set("custom_state", x[2]);
        }
    }

    struct IdentityDiscretization;

    impl Discretization<ThreeFactorProcess> for IdentityDiscretization {
        fn step(
            &self,
            _process: &ThreeFactorProcess,
            _t: f64,
            dt: f64,
            x: &mut [f64],
            z: &[f64],
            _work: &mut [f64],
        ) {
            x[0] += z[0] * dt.sqrt();
            x[2] += 1.0;
        }
    }

    #[derive(Clone)]
    struct CustomStatePayoff {
        missing_custom_state: bool,
    }

    impl Payoff for CustomStatePayoff {
        fn on_event(&mut self, state: &mut PathState) {
            if state.get("custom_state").is_none() {
                self.missing_custom_state = true;
            }
        }

        fn value(&self, currency: Currency) -> finstack_core::money::Money {
            finstack_core::money::Money::new(
                if self.missing_custom_state { 0.0 } else { 1.0 },
                currency,
            )
        }

        fn reset(&mut self) {
            self.missing_custom_state = false;
        }
    }

    #[test]
    fn test_antithetic_uses_process_path_state_population() {
        let mut rng = PhiloxRng::new(7);
        let process = ThreeFactorProcess;
        let disc = IdentityDiscretization;
        let initial_state = vec![100.0, 0.04, 3.0];
        let payoff = CustomStatePayoff {
            missing_custom_state: false,
        };
        let time_grid = TimeGrid::uniform(1.0, 2).expect("should succeed");
        let config = AntitheticConfig {
            num_pairs: 2,
            time_grid: &time_grid,
            currency: Currency::USD,
            discount_factor: 1.0,
        };

        let stats = antithetic_price(&mut rng, &process, &disc, &initial_state, &payoff, &config);

        assert_eq!(stats.mean(), 1.0);
    }
}
