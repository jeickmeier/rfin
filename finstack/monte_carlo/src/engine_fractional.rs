//! Fractional noise integration for rough volatility Monte Carlo simulation.
//!
//! Provides helper functions to run Monte Carlo paths with pre-generated
//! fractional Brownian motion increments. Used by rBergomi and Cheyette rough
//! vol discretizations.
//!
//! The standard [`crate::engine::McEngine`] simulation loop draws i.i.d.
//! normals at each time step. Rough volatility models instead need
//! correlated noise across *all* steps, generated upfront via an
//! implementor of [`crate::rng::fbm::FractionalNoiseGenerator`]. The
//! functions in this module wrap the per-path simulation loop to handle
//! that pre-generation and injection.
//!
//! # Usage
//!
//! ```rust,ignore
//! use finstack_monte_carlo::engine_fractional::simulate_path_fractional;
//!
//! // Pre-generate fBM increments for the full path…
//! rng.fill_std_normals(&mut fbm_normals);
//! fbm_gen.generate(&fbm_normals, &mut fbm_increments);
//!
//! // …then simulate with injection into z[fbm_z_index].
//! let pv = simulate_path_fractional(
//!     &mut path_rng, time_grid, process, disc, initial_state,
//!     &mut payoff, currency, &fbm_increments, fbm_z_index,
//!     &mut state, &mut z, &mut work,
//! )?;
//! ```

use crate::time_grid::TimeGrid;
use crate::traits::{Discretization, PathState, Payoff, RandomStream, StochasticProcess};
use finstack_core::currency::Currency;
use finstack_core::Result;

/// Simulate a single Monte Carlo path with fractional noise injection.
///
/// At each time step, standard normals are drawn from `rng` for all factors.
/// Then `z[fbm_z_index]` is overwritten with the pre-generated fBM increment
/// for that step.
///
/// # Arguments
///
/// * `rng` — random stream for standard normals and uniform draws
/// * `time_grid` — simulation time grid
/// * `process` — stochastic process
/// * `disc` — discretization scheme
/// * `initial_state` — initial state vector (length = `process.dim()`)
/// * `payoff` — payoff accumulator (must be pre-reset by caller)
/// * `currency` — result currency
/// * `fbm_increments` — pre-generated fBM increments (length = `num_steps`)
/// * `fbm_z_index` — index in `z` where the fBM increment is injected
/// * `state` — reusable state buffer (length = `process.dim()`)
/// * `z` — reusable noise buffer (length = `process.num_factors()`)
/// * `work` — reusable work buffer (length = `disc.work_size(process)`)
///
/// # Errors
///
/// Returns an error if the payoff value computation fails.
#[allow(clippy::too_many_arguments)]
pub fn simulate_path_fractional<R, P, D, F>(
    rng: &mut R,
    time_grid: &TimeGrid,
    process: &P,
    disc: &D,
    initial_state: &[f64],
    payoff: &mut F,
    currency: Currency,
    fbm_increments: &[f64],
    fbm_z_index: usize,
    state: &mut [f64],
    z: &mut [f64],
    work: &mut [f64],
) -> Result<f64>
where
    R: RandomStream,
    P: StochasticProcess,
    D: Discretization<P>,
    F: Payoff,
{
    state.copy_from_slice(initial_state);

    // Zero work buffer at the start of each path so discretizations don't
    // need to detect path boundaries via fragile float comparisons.
    for w in work.iter_mut() {
        *w = 0.0;
    }

    let mut path_state = PathState::new(0, 0.0);
    process.populate_path_state(state, &mut path_state);
    path_state.set_uniform_random(rng.next_u01());
    payoff.on_event(&mut path_state);

    for step in 0..time_grid.num_steps() {
        let t = time_grid.time(step);
        let dt = time_grid.dt(step);

        rng.fill_std_normals(z);

        // Inject fBM increment, replacing the standard normal at z_index.
        debug_assert!(
            step < fbm_increments.len(),
            "fBM increments shorter than time grid: step {step} >= len {}",
            fbm_increments.len()
        );
        debug_assert!(
            fbm_z_index < z.len(),
            "fbm_z_index {fbm_z_index} out of bounds for z len {}",
            z.len()
        );
        z[fbm_z_index] = fbm_increments[step];

        disc.step(process, t, dt, state, z, work);

        path_state.set_step_time(step + 1, t + dt);
        process.populate_path_state(state, &mut path_state);
        path_state.set_uniform_random(rng.next_u01());
        payoff.on_event(&mut path_state);
    }

    Ok(payoff.value(currency).amount())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::fbm::{CholeskyFbm, FractionalNoiseGenerator};
    use crate::rng::philox::PhiloxRng;
    use finstack_core::currency::Currency;
    use finstack_core::money::Money;

    // --- Minimal test process: 1D spot with 2 factors (spot + fBM slot) ---

    #[derive(Debug, Clone)]
    struct TwoFactorSpot;

    impl StochasticProcess for TwoFactorSpot {
        fn dim(&self) -> usize {
            1
        }

        fn num_factors(&self) -> usize {
            2 // z[0] = spot noise, z[1] = fBM slot
        }

        fn drift(&self, _t: f64, _x: &[f64], out: &mut [f64]) {
            out[0] = 0.0;
        }

        fn diffusion(&self, _t: f64, _x: &[f64], out: &mut [f64]) {
            out[0] = 1.0;
            if out.len() > 1 {
                out[1] = 0.0;
            }
        }

        fn populate_path_state(&self, x: &[f64], state: &mut PathState) {
            state.set(crate::traits::state_keys::SPOT, x[0]);
        }
    }

    // --- Simple discretization that records z[1] into x[0] ---

    #[derive(Debug, Clone)]
    struct RecordFbmDisc;

    impl Discretization<TwoFactorSpot> for RecordFbmDisc {
        fn step(
            &self,
            _process: &TwoFactorSpot,
            _t: f64,
            _dt: f64,
            x: &mut [f64],
            z: &[f64],
            _work: &mut [f64],
        ) {
            // Accumulate z[1] (the fBM slot) into x[0]
            x[0] += z[1];
        }

        fn work_size(&self, _process: &TwoFactorSpot) -> usize {
            0
        }
    }

    // --- Trivial payoff that returns final spot ---

    #[derive(Debug, Clone, Default)]
    struct TerminalSpotPayoff {
        terminal: f64,
    }

    impl Payoff for TerminalSpotPayoff {
        fn on_event(&mut self, state: &mut PathState) {
            if let Some(s) = state.spot() {
                self.terminal = s;
            }
        }

        fn value(&self, currency: Currency) -> Money {
            Money::new(self.terminal, currency)
        }

        fn reset(&mut self) {
            self.terminal = 0.0;
        }
    }

    #[test]
    fn test_simulate_path_fractional_injects_fbm() {
        let process = TwoFactorSpot;
        let disc = RecordFbmDisc;
        let time_grid = TimeGrid::uniform(1.0, 4).expect("valid grid");
        let mut rng = PhiloxRng::new(42);

        let fbm_increments = vec![0.1, 0.2, 0.3, 0.4];
        let mut payoff = TerminalSpotPayoff::default();
        let mut state = vec![0.0];
        let mut z = vec![0.0, 0.0];
        let mut work = vec![];

        let pv = simulate_path_fractional(
            &mut rng,
            &time_grid,
            &process,
            &disc,
            &[0.0],
            &mut payoff,
            Currency::USD,
            &fbm_increments,
            1,
            &mut state,
            &mut z,
            &mut work,
        )
        .expect("simulation should succeed");

        // The disc accumulates z[1] into x[0], so final = sum of fBM increments
        let expected: f64 = fbm_increments.iter().sum();
        assert!(
            (pv - expected).abs() < 1e-12,
            "Expected {expected}, got {pv}"
        );
    }

    #[test]
    fn test_simulate_with_fbm_generator_round_trip() {
        let times = vec![0.0, 0.25, 0.5, 0.75, 1.0];
        let hurst = 0.1;
        let fbm_gen = CholeskyFbm::new(&times, hurst).expect("valid fBM generator");
        let num_steps = fbm_gen.num_steps();

        let process = TwoFactorSpot;
        let disc = RecordFbmDisc;
        let time_grid = TimeGrid::uniform(1.0, 4).expect("valid grid");
        let mut rng = PhiloxRng::new(99);

        let mut payoff = TerminalSpotPayoff::default();
        let mut state = vec![0.0];
        let mut z = vec![0.0, 0.0];
        let mut work = vec![];
        let mut fbm_normals = vec![0.0; num_steps];
        let mut fbm_increments = vec![0.0; num_steps];

        // Generate fBM increments then simulate — the pattern callers use
        rng.fill_std_normals(&mut fbm_normals);
        fbm_gen.generate(&fbm_normals, &mut fbm_increments);

        let pv = simulate_path_fractional(
            &mut rng,
            &time_grid,
            &process,
            &disc,
            &[0.0],
            &mut payoff,
            Currency::USD,
            &fbm_increments,
            1,
            &mut state,
            &mut z,
            &mut work,
        )
        .expect("simulation should succeed");

        // pv should be the sum of correlated fBM increments (finite, non-NaN)
        assert!(pv.is_finite(), "payoff must be finite, got {pv}");
    }
}
