//! Pricing-specific traits for Monte Carlo (payoffs and observers).
//!
//! This module houses `Payoff` and related observer traits that are specific
//! to instrument pricing. Generic MC traits such as `RandomStream`,
//! `StochasticProcess`, `Discretization`, and `PathState` remain under
//! `instruments::common::mc::traits`.

use crate::instruments::common_impl::mc::traits::PathState;
use finstack_core::currency::Currency;
use finstack_core::money::Money;

/// Payoff computation with currency safety.
///
/// Payoffs accumulate path information via `on_event` calls and
/// return a final `Money` value. This ensures all results carry
/// explicit currency information.
pub trait Payoff: Send + Sync + Clone {
    /// Process a path event (fixing, barrier check, etc.).
    ///
    /// The PathState is mutable to allow payoffs to record cashflows
    /// using `state.add_cashflow()`. These cashflows will be transferred
    /// to PathPoint during path capture.
    fn on_event(&mut self, state: &mut PathState);

    /// Compute final payoff value in the specified currency (undiscounted).
    fn value(&self, currency: Currency) -> Money;

    /// Reset payoff state for next path.
    fn reset(&mut self);

    /// Optional: discount factor to apply; default is 1.0 (no discounting).
    fn discount_factor(&self) -> f64 {
        1.0
    }

    /// Optional hook invoked at the start of each path with access to RNG.
    ///
    /// Useful to draw per-path random variables (e.g., default threshold E ~ Exp(1)).
    fn on_path_start<R: crate::instruments::common_impl::mc::traits::RandomStream>(
        &mut self,
        _rng: &mut R,
    ) {
    }
}

/// Path observer for collecting statistics along paths.
///
/// This trait enables extracting intermediate path information
/// beyond just the final payoff (useful for debugging, Greeks, etc.).
pub trait PathObserver: Send + Sync {
    /// Observe a path state.
    fn observe(&mut self, state: &mut PathState);

    /// Reset observer for next path.
    fn reset(&mut self);

    /// Extract collected data (format depends on observer).
    fn data(&self) -> Vec<f64> {
        Vec::new()
    }
}

// models/monte_carlo/traits.rs placeholder
