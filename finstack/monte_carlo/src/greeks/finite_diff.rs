//! Finite differences with Common Random Numbers (CRN).
//!
//! Computes Greeks by bump-and-revalue using the same random numbers for base
//! and bumped scenarios. This reduces variance significantly compared with an
//! independent re-run.
//!
//! # CRN invariant
//!
//! CRN here relies on a **splittable, counter-based RNG** whose `split(i)`
//! output depends only on the seed, never on how much of the stream has been
//! consumed. Philox (the default [`crate::rng::philox::PhiloxRng`]) satisfies
//! this; Sobol explicitly does not. These helpers therefore guard at runtime
//! via [`RandomStream::supports_splitting`] to prevent silent CRN breakage.

use super::super::engine::McEngine;
use crate::traits::Payoff;
use crate::traits::{Discretization, RandomStream, StochasticProcess};
use finstack_core::currency::Currency;
use finstack_core::Result;

/// Guard that the supplied RNG supports deterministic splitting, which is a
/// prerequisite for CRN across bump-and-revalue calls.
fn require_splittable_rng<R: RandomStream>(rng: &R, routine: &str) -> Result<()> {
    if !rng.supports_splitting() {
        return Err(finstack_core::Error::Validation(format!(
            "{routine} requires an RNG that supports deterministic splitting (e.g. PhiloxRng); \
             the supplied generator reports supports_splitting() = false. Without stream \
             splitting the bumped and base valuations consume different random numbers and CRN \
             variance reduction is lost."
        )));
    }
    Ok(())
}

/// Compute a symmetric-difference bump size in absolute price units.
///
/// The `bump_size` argument is interpreted as a *relative* bump (e.g. `0.01`
/// for 1 % of spot). A floor of `1e-8` keeps the bump numerically meaningful
/// for near-zero spots (rates, FX implied yields, etc.).
fn bump_amount(initial_spot: f64, bump_size: f64) -> f64 {
    (initial_spot.abs() * bump_size).max(1e-8)
}

/// Compute delta using central finite differences with CRN.
///
/// ```text
/// Δ ≈ (V(S₀+h) − V(S₀−h)) / (2h)
/// ```
///
/// Both valuations reuse `rng` by reference. This works only when the RNG is
/// splittable (e.g. [`crate::rng::philox::PhiloxRng`]) so that `rng.split(i)`
/// produces identical per-path streams across calls regardless of how much of
/// the parent stream has been consumed.
///
/// # Arguments
///
/// * `engine` - MC engine (borrowed, used for the two price runs)
/// * `rng` - Random number generator; must support splitting
/// * `process` - Stochastic process
/// * `disc` - Discretization scheme
/// * `initial_spot` - Initial spot price (S₀)
/// * `payoff` - Payoff specification
/// * `currency` - Currency tag
/// * `discount_factor` - Discount factor
/// * `bump_size` - Relative bump (e.g. `0.01` for 1 %)
///
/// # Returns
///
/// `(delta, stderr)` — the central-difference estimator and its standard
/// error under the assumption of independence between the up and down runs
/// (conservative; CRN makes the true stderr smaller but computing it exactly
/// would require per-path pairing outside this helper).
///
/// # Errors
///
/// Returns [`finstack_core::Error::Validation`] when the supplied RNG does
/// not support splitting or when either `engine.price` call fails.
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
) -> Result<(f64, f64)>
where
    R: RandomStream,
    P: StochasticProcess,
    D: Discretization<P>,
    F: Payoff,
{
    require_splittable_rng(rng, "finite_diff_delta")?;
    let h = bump_amount(initial_spot, bump_size);

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

    let initial_down = vec![(initial_spot - h).max(1e-12)];
    let result_down = engine.price(
        rng,
        process,
        disc,
        &initial_down,
        payoff,
        currency,
        discount_factor,
    )?;

    let delta = (result_up.mean.amount() - result_down.mean.amount()) / (2.0 * h);
    // Conservative stderr under the independence assumption. Propagating a
    // tighter CRN stderr would require per-path bookkeeping not exposed
    // through the current engine API.
    let se_up = result_up.stderr;
    let se_down = result_down.stderr;
    let stderr = (se_up * se_up + se_down * se_down).sqrt() / (2.0 * h);

    Ok((delta, stderr))
}

/// Compute gamma using a second central difference with CRN.
///
/// ```text
/// Γ ≈ (V(S₀+h) − 2·V(S₀) + V(S₀−h)) / h²
/// ```
///
/// # Returns
///
/// `(gamma, stderr)` — the second-difference estimator and a conservative
/// standard error under the assumption of independent MC stderrs at the
/// three grid points.
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
) -> Result<(f64, f64)>
where
    R: RandomStream,
    P: StochasticProcess,
    D: Discretization<P>,
    F: Payoff,
{
    require_splittable_rng(rng, "finite_diff_gamma")?;
    let h = bump_amount(initial_spot, bump_size);

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

    let initial_down = vec![(initial_spot - h).max(1e-12)];
    let result_down = engine.price(
        rng,
        process,
        disc,
        &initial_down,
        payoff,
        currency,
        discount_factor,
    )?;

    let gamma = (result_up.mean.amount() - 2.0 * result_base.mean.amount()
        + result_down.mean.amount())
        / (h * h);
    let se_up = result_up.stderr;
    let se_base = result_base.stderr;
    let se_down = result_down.stderr;
    // Variance of (V_up − 2V_base + V_down): 1·V_up + 4·V_base + 1·V_down under
    // independence. CRN makes this pessimistic but not wrong.
    let stderr = (se_up * se_up + 4.0 * se_base * se_base + se_down * se_down).sqrt() / (h * h);

    Ok((gamma, stderr))
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::super::super::engine::McEngineConfig;
    use super::*;
    use crate::discretization::exact::ExactGbm;
    use crate::payoff::vanilla::EuropeanCall;
    use crate::process::gbm::{GbmParams, GbmProcess};
    use crate::rng::philox::PhiloxRng;
    use crate::time_grid::TimeGrid;

    #[test]
    fn test_finite_diff_delta_atm() {
        let time_grid = TimeGrid::uniform(1.0, 50).expect("should succeed");
        let engine = McEngine::new(McEngineConfig {
            num_paths: 5_000,
            seed: 42,
            time_grid,
            target_ci_half_width: None,
            use_parallel: false,
            chunk_size: 1000,
            path_capture: crate::engine::PathCaptureConfig::default(),
            antithetic: false,
        });

        let rng = PhiloxRng::new(42);
        let gbm = GbmProcess::new(GbmParams::new(0.05, 0.0, 0.2).unwrap());
        let disc = ExactGbm::new();
        let call = EuropeanCall::new(100.0, 1.0, 50);

        let (delta, stderr) = finite_diff_delta(
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
        .expect("should succeed");

        // ATM call delta should be around 0.5
        assert!(delta > 0.3 && delta < 0.7);
        assert!(stderr.is_finite() && stderr >= 0.0);
    }

    #[test]
    fn test_finite_diff_gamma_positive() {
        let time_grid = TimeGrid::uniform(1.0, 50).expect("should succeed");
        let engine = McEngine::new(McEngineConfig {
            num_paths: 5_000,
            seed: 42,
            time_grid,
            target_ci_half_width: None,
            use_parallel: false,
            chunk_size: 1000,
            path_capture: crate::engine::PathCaptureConfig::default(),
            antithetic: false,
        });

        let rng = PhiloxRng::new(42);
        let gbm = GbmProcess::new(GbmParams::new(0.05, 0.0, 0.2).unwrap());
        let disc = ExactGbm::new();
        let call = EuropeanCall::new(100.0, 1.0, 50);

        let (gamma, stderr) = finite_diff_gamma(
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
        .expect("should succeed");

        // Gamma should be positive for ATM options
        assert!(gamma > 0.0);
        assert!(stderr.is_finite() && stderr >= 0.0);
    }
}
