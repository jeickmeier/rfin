//! Payoff trait definitions.
//!
//! The core Payoff trait is already defined in super::super::traits.
//! This module provides additional utilities and helper traits.

use crate::instruments::common_impl::mc::traits::PathState;
use crate::instruments::common_impl::models::monte_carlo::traits::Payoff;
use finstack_core::currency::Currency;
use finstack_core::money::Money;

/// Payoff that tracks a specific time step for evaluation.
///
/// This is a common pattern for European-style payoffs that only
/// depend on the state at maturity.
pub trait TerminalPayoff: Payoff {
    /// Get the maturity step index.
    fn maturity_step(&self) -> usize;

    /// Check if this step is the maturity step.
    fn is_maturity(&self, state: &PathState) -> bool {
        state.step == self.maturity_step()
    }
}

/// Payoff builder for fluent construction.
pub struct PayoffBuilder<P> {
    payoff: P,
}

impl<P> PayoffBuilder<P> {
    /// Create a new builder with the given payoff.
    pub fn new(payoff: P) -> Self {
        Self { payoff }
    }

    /// Build the payoff.
    pub fn build(self) -> P {
        self.payoff
    }
}

/// Helper for creating simple terminal payoffs.
#[derive(Debug, Clone)]
pub struct SimpleTerminalPayoff<F>
where
    F: Fn(f64) -> f64 + Send + Sync,
{
    /// Maturity step
    pub maturity_step: usize,
    /// Payoff function
    pub payoff_fn: F,
    /// Terminal spot value
    pub terminal_spot: f64,
    /// Notional amount
    pub notional: f64,
}

impl<F> SimpleTerminalPayoff<F>
where
    F: Fn(f64) -> f64 + Send + Sync,
{
    /// Create a new simple terminal payoff.
    pub fn new(maturity_step: usize, notional: f64, payoff_fn: F) -> Self {
        Self {
            maturity_step,
            payoff_fn,
            terminal_spot: 0.0,
            notional,
        }
    }
}

impl<F> Payoff for SimpleTerminalPayoff<F>
where
    F: Fn(f64) -> f64 + Send + Sync + Clone,
{
    fn on_event(&mut self, state: &mut PathState) {
        if state.step == self.maturity_step {
            self.terminal_spot = state.spot().unwrap_or(0.0);
        }
    }

    fn value(&self, currency: Currency) -> Money {
        let payoff = (self.payoff_fn)(self.terminal_spot);
        Money::new(payoff * self.notional, currency)
    }

    fn reset(&mut self) {
        self.terminal_spot = 0.0;
    }
}

impl<F> TerminalPayoff for SimpleTerminalPayoff<F>
where
    F: Fn(f64) -> f64 + Send + Sync + Clone,
{
    fn maturity_step(&self) -> usize {
        self.maturity_step
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::instruments::common_impl::mc::traits::state_keys;

    #[test]
    fn test_simple_terminal_payoff() {
        let payoff_fn = |s: f64| (s - 100.0).max(0.0);
        let mut payoff = SimpleTerminalPayoff::new(10, 1.0, payoff_fn);

        // Before maturity
        let mut state = PathState::new(5, 0.5);
        state.set(state_keys::SPOT, 110.0);
        payoff.on_event(&mut state);
        assert_eq!(payoff.terminal_spot, 0.0);

        // At maturity
        let mut state_mat = PathState::new(10, 1.0);
        state_mat.set(state_keys::SPOT, 110.0);
        payoff.on_event(&mut state_mat);
        assert_eq!(payoff.terminal_spot, 110.0);

        // Get value
        let value = payoff.value(Currency::USD);
        assert_eq!(value.amount(), 10.0); // max(110 - 100, 0)
    }

    #[test]
    fn test_terminal_payoff_trait() {
        let payoff_fn = |s: f64| s;
        let payoff = SimpleTerminalPayoff::new(10, 1.0, payoff_fn);

        assert_eq!(payoff.maturity_step(), 10);

        let state = PathState::new(10, 1.0);
        assert!(payoff.is_maturity(&state));

        let state_before = PathState::new(9, 0.9);
        assert!(!payoff.is_maturity(&state_before));
    }
}
