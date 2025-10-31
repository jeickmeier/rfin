//! Finite differences with Common Random Numbers (CRN).
//!
//! Computes Greeks by bump-and-revalue using the same random numbers
//! for base and bumped scenarios. This reduces variance significantly.

use super::super::engine::McEngine;
use super::super::traits::{Discretization, Payoff, RandomStream, StochasticProcess};
use finstack_core::currency::Currency;
use finstack_core::Result;

/// Compute delta using finite differences with CRN.
///
/// Delta ≈ (V(S₀ + h) - V(S₀ - h)) / (2h)
///
/// Uses the same random numbers for both valuations to reduce variance.
///
/// # Arguments
///
/// * `engine` - MC engine  
/// * `rng` - Random number generator
/// * `process` - Stochastic process
/// * `disc` - Discretization scheme
/// * `initial_spot` - Initial spot price
/// * `payoff` - Payoff specification
/// * `currency` - Currency
/// * `discount_factor` - Discount factor
/// * `bump_size` - Relative bump size (e.g., 0.01 for 1%)
///
/// # Returns
///
/// Delta estimate
#[allow(clippy::too_many_arguments)]
pub fn finite_diff_delta<R, P, D, F>(
    engine: &McEngine,
    rng: &R,
    process: &P,
    disc: &D,
    initial_spot: f64,
    payoff: &F,
    currency: Currency,
    discount_factor: f64,
    bump_size: f64,
) -> Result<f64>
where
    R: RandomStream,
    P: StochasticProcess,
    D: Discretization<P>,
    F: Payoff,
{
    let h = initial_spot * bump_size;

    // Price with S₀ + h
    let initial_up = vec![initial_spot + h];
    let result_up = engine.price(
        rng,
        process,
        disc,
        &initial_up,
        payoff,
        currency,
        discount_factor,
    )?;

    // Price with S₀ - h (using same RNG seed via split)
    let initial_down = vec![initial_spot - h];
    let result_down = engine.price(
        rng,
        process,
        disc,
        &initial_down,
        payoff,
        currency,
        discount_factor,
    )?;

    // Central difference
    let delta = (result_up.mean.amount() - result_down.mean.amount()) / (2.0 * h);

    Ok(delta)
}

/// Compute gamma using finite differences.
///
/// Gamma ≈ (V(S₀+h) - 2V(S₀) + V(S₀-h)) / h²
#[allow(clippy::too_many_arguments)]
pub fn finite_diff_gamma<R, P, D, F>(
    engine: &McEngine,
    rng: &R,
    process: &P,
    disc: &D,
    initial_spot: f64,
    payoff: &F,
    currency: Currency,
    discount_factor: f64,
    bump_size: f64,
) -> Result<f64>
where
    R: RandomStream,
    P: StochasticProcess,
    D: Discretization<P>,
    F: Payoff,
{
    let h = initial_spot * bump_size;

    // Price at base
    let initial_base = vec![initial_spot];
    let result_base = engine.price(
        rng,
        process,
        disc,
        &initial_base,
        payoff,
        currency,
        discount_factor,
    )?;

    // Price up
    let initial_up = vec![initial_spot + h];
    let result_up = engine.price(
        rng,
        process,
        disc,
        &initial_up,
        payoff,
        currency,
        discount_factor,
    )?;

    // Price down
    let initial_down = vec![initial_spot - h];
    let result_down = engine.price(
        rng,
        process,
        disc,
        &initial_down,
        payoff,
        currency,
        discount_factor,
    )?;

    // Second derivative
    let gamma = (result_up.mean.amount() - 2.0 * result_base.mean.amount()
        + result_down.mean.amount())
        / (h * h);

    Ok(gamma)
}

#[cfg(test)]
mod tests {
    use super::super::super::discretization::exact::ExactGbm;
    use super::super::super::engine::McEngineConfig;
    use super::super::super::payoff::vanilla::EuropeanCall;
    use super::super::super::process::gbm::{GbmParams, GbmProcess};
    use super::super::super::rng::philox::PhiloxRng;
    use super::super::super::time_grid::TimeGrid;
    use super::*;

    #[test]
    fn test_finite_diff_delta_atm() {
        let time_grid = TimeGrid::uniform(1.0, 50).unwrap();
        let engine = McEngine::new(McEngineConfig {
            num_paths: 5_000,
            seed: 42,
            time_grid,
            target_ci_half_width: None,
            use_parallel: false,
            chunk_size: 1000,
            path_capture: crate::instruments::common::mc::engine::PathCaptureConfig::default(),
        });

        let rng = PhiloxRng::new(42);
        let gbm = GbmProcess::new(GbmParams::new(0.05, 0.0, 0.2));
        let disc = ExactGbm::new();
        let call = EuropeanCall::new(100.0, 1.0, 50);

        let delta = finite_diff_delta(
            &engine,
            &rng,
            &gbm,
            &disc,
            100.0,
            &call,
            Currency::USD,
            1.0,
            0.01,
        )
        .unwrap();

        // ATM call delta should be around 0.5
        assert!(delta > 0.3 && delta < 0.7);
    }

    #[test]
    fn test_finite_diff_gamma_positive() {
        let time_grid = TimeGrid::uniform(1.0, 50).unwrap();
        let engine = McEngine::new(McEngineConfig {
            num_paths: 5_000,
            seed: 42,
            time_grid,
            target_ci_half_width: None,
            use_parallel: false,
            chunk_size: 1000,
            path_capture: crate::instruments::common::mc::engine::PathCaptureConfig::default(),
        });

        let rng = PhiloxRng::new(42);
        let gbm = GbmProcess::new(GbmParams::new(0.05, 0.0, 0.2));
        let disc = ExactGbm::new();
        let call = EuropeanCall::new(100.0, 1.0, 50);

        let gamma = finite_diff_gamma(
            &engine,
            &rng,
            &gbm,
            &disc,
            100.0,
            &call,
            Currency::USD,
            1.0,
            0.01,
        )
        .unwrap();

        // Gamma should be positive for ATM options
        assert!(gamma > 0.0);
    }
}
