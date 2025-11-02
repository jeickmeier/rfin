//! Lookback option payoffs.
//!
//! Lookback options depend on the maximum or minimum spot price
//! observed over the life of the option.

use crate::instruments::common::mc::traits::PathState;
use crate::instruments::common::models::monte_carlo::traits::Payoff;
use finstack_core::currency::Currency;
use finstack_core::money::Money;

/// Lookback call option.
///
/// Payoff: max(S_max - K, 0) × N
///
/// where S_max is the maximum spot observed over the path.
#[derive(Clone, Debug)]
pub struct LookbackCall {
    /// Strike price
    pub strike: f64,
    /// Notional
    pub notional: f64,
    /// Maturity step
    pub maturity_step: usize,

    // State
    max_spot: f64,
}

impl LookbackCall {
    /// Create a new lookback call option.
    pub fn new(strike: f64, notional: f64, maturity_step: usize) -> Self {
        Self {
            strike,
            notional,
            maturity_step,
            max_spot: f64::NEG_INFINITY,
        }
    }
}

impl Payoff for LookbackCall {
    fn on_event(&mut self, state: &PathState) {
        if let Some(spot) = state.spot() {
            self.max_spot = self.max_spot.max(spot);
        }
    }

    fn value(&self, currency: Currency) -> Money {
        let intrinsic = (self.max_spot - self.strike).max(0.0);
        Money::new(intrinsic * self.notional, currency)
    }

    fn reset(&mut self) {
        self.max_spot = f64::NEG_INFINITY;
    }
}

/// Lookback put option.
///
/// Payoff: max(K - S_min, 0) × N
///
/// where S_min is the minimum spot observed over the path.
#[derive(Clone, Debug)]
pub struct LookbackPut {
    pub strike: f64,
    pub notional: f64,
    pub maturity_step: usize,

    min_spot: f64,
}

impl LookbackPut {
    /// Create a new lookback put option.
    pub fn new(strike: f64, notional: f64, maturity_step: usize) -> Self {
        Self {
            strike,
            notional,
            maturity_step,
            min_spot: f64::INFINITY,
        }
    }
}

impl Payoff for LookbackPut {
    fn on_event(&mut self, state: &PathState) {
        if let Some(spot) = state.spot() {
            self.min_spot = self.min_spot.min(spot);
        }
    }

    fn value(&self, currency: Currency) -> Money {
        let intrinsic = (self.strike - self.min_spot).max(0.0);
        Money::new(intrinsic * self.notional, currency)
    }

    fn reset(&mut self) {
        self.min_spot = f64::INFINITY;
    }
}

/// Floating strike lookback call.
///
/// Payoff: (S_T - S_min) × N
///
/// The strike "floats" to the minimum observed price.
#[derive(Clone, Debug)]
pub struct FloatingStrikeLookbackCall {
    pub notional: f64,
    pub maturity_step: usize,

    terminal_spot: f64,
    min_spot: f64,
}

impl FloatingStrikeLookbackCall {
    /// Create a new floating strike lookback call.
    pub fn new(notional: f64, maturity_step: usize) -> Self {
        Self {
            notional,
            maturity_step,
            terminal_spot: 0.0,
            min_spot: f64::INFINITY,
        }
    }
}

impl Payoff for FloatingStrikeLookbackCall {
    fn on_event(&mut self, state: &PathState) {
        if let Some(spot) = state.spot() {
            self.min_spot = self.min_spot.min(spot);
            if state.step == self.maturity_step {
                self.terminal_spot = spot;
            }
        }
    }

    fn value(&self, currency: Currency) -> Money {
        let payoff = self.terminal_spot - self.min_spot;
        Money::new(payoff * self.notional, currency)
    }

    fn reset(&mut self) {
        self.terminal_spot = 0.0;
        self.min_spot = f64::INFINITY;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruments::common::mc::traits::state_keys;

    fn create_state(step: usize, spot: f64) -> PathState {
        let mut state = PathState::new(step, step as f64 * 0.1);
        state.set(state_keys::SPOT, spot);
        state
    }

    #[test]
    fn test_lookback_call() {
        let mut lookback = LookbackCall::new(100.0, 1.0, 10);

        // Simulate path: max = 120
        lookback.on_event(&create_state(0, 100.0));
        lookback.on_event(&create_state(5, 120.0));
        lookback.on_event(&create_state(10, 110.0));

        let value = lookback.value(Currency::USD);
        // max(120 - 100, 0) = 20
        assert_eq!(value.amount(), 20.0);
    }

    #[test]
    fn test_lookback_put() {
        let mut lookback = LookbackPut::new(100.0, 1.0, 10);

        // Simulate path: min = 80
        lookback.on_event(&create_state(0, 100.0));
        lookback.on_event(&create_state(5, 80.0));
        lookback.on_event(&create_state(10, 90.0));

        let value = lookback.value(Currency::USD);
        // max(100 - 80, 0) = 20
        assert_eq!(value.amount(), 20.0);
    }

    #[test]
    fn test_floating_strike_lookback() {
        let mut lookback = FloatingStrikeLookbackCall::new(1.0, 10);

        // Path: starts 100, min 90, ends 110
        lookback.on_event(&create_state(0, 100.0));
        lookback.on_event(&create_state(5, 90.0));
        lookback.on_event(&create_state(10, 110.0));

        let value = lookback.value(Currency::USD);
        // 110 - 90 = 20
        assert_eq!(value.amount(), 20.0);
    }

    #[test]
    fn test_lookback_reset() {
        let mut lookback = LookbackCall::new(100.0, 1.0, 10);

        lookback.on_event(&create_state(0, 100.0));
        lookback.on_event(&create_state(5, 120.0));
        assert_eq!(lookback.max_spot, 120.0);

        lookback.reset();
        assert_eq!(lookback.max_spot, f64::NEG_INFINITY);
    }
}
