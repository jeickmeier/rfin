//! Lookback option payoffs.
//!
//! Lookback options depend on the maximum or minimum spot price
//! observed over the life of the option.
//!
//! # Unified Implementation
//!
//! This module provides a unified [`Lookback`] struct that handles both call and put
//! lookback options via the [`LookbackDirection`] enum. This eliminates code duplication
//! while maintaining the same functionality.
//!
//! Legacy type aliases [`LookbackCall`] and [`LookbackPut`] are provided for backward
//! compatibility but are deprecated in favor of the unified struct.

use crate::instruments::common::mc::traits::PathState;
use crate::instruments::common::models::monte_carlo::traits::Payoff;
use finstack_core::currency::Currency;
use finstack_core::money::Money;

/// Direction of a fixed-strike lookback option.
///
/// Determines whether the option tracks the maximum (call) or minimum (put) spot price.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LookbackDirection {
    /// Call option: payoff = max(S_max - K, 0)
    Call,
    /// Put option: payoff = max(K - S_min, 0)
    Put,
}

/// Unified fixed-strike lookback option.
///
/// Supports both call and put lookback options through the [`LookbackDirection`] parameter.
///
/// # Payoffs
///
/// - **Call**: max(S_max - K, 0) × N, where S_max is the maximum spot observed
/// - **Put**: max(K - S_min, 0) × N, where S_min is the minimum spot observed
///
/// # Examples
///
/// ```
/// use finstack_valuations::instruments::common::models::monte_carlo::payoff::lookback::{Lookback, LookbackDirection};
///
/// // Create a lookback call
/// let call = Lookback::new(LookbackDirection::Call, 100.0, 1.0, 10);
///
/// // Create a lookback put
/// let put = Lookback::new(LookbackDirection::Put, 100.0, 1.0, 10);
/// ```
#[derive(Clone, Debug)]
pub struct Lookback {
    /// Direction (call or put)
    pub direction: LookbackDirection,
    /// Strike price
    pub strike: f64,
    /// Notional amount
    pub notional: f64,
    /// Time step index for maturity
    pub maturity_step: usize,

    /// Extreme spot price observed (max for Call, min for Put)
    extreme_spot: f64,
}

impl Lookback {
    /// Create a new fixed-strike lookback option.
    ///
    /// The `extreme_spot` is initialized to:
    /// - `NEG_INFINITY` for calls (to track maximum)
    /// - `INFINITY` for puts (to track minimum)
    pub fn new(direction: LookbackDirection, strike: f64, notional: f64, maturity_step: usize) -> Self {
        let extreme_spot = match direction {
            LookbackDirection::Call => f64::NEG_INFINITY,
            LookbackDirection::Put => f64::INFINITY,
        };

        Self {
            direction,
            strike,
            notional,
            maturity_step,
            extreme_spot,
        }
    }
}

impl Payoff for Lookback {
    fn on_event(&mut self, state: &mut PathState) {
        if let Some(spot) = state.spot() {
            self.extreme_spot = match self.direction {
                LookbackDirection::Call => self.extreme_spot.max(spot),
                LookbackDirection::Put => self.extreme_spot.min(spot),
            };
        }
    }

    fn value(&self, currency: Currency) -> Money {
        let intrinsic = match self.direction {
            LookbackDirection::Call => (self.extreme_spot - self.strike).max(0.0),
            LookbackDirection::Put => (self.strike - self.extreme_spot).max(0.0),
        };
        Money::new(intrinsic * self.notional, currency)
    }

    fn reset(&mut self) {
        self.extreme_spot = match self.direction {
            LookbackDirection::Call => f64::NEG_INFINITY,
            LookbackDirection::Put => f64::INFINITY,
        };
    }
}

/// Legacy type alias for lookback call option.
///
/// # Deprecated
///
/// Use [`Lookback::new(LookbackDirection::Call, ...)`] instead for new code.
/// This alias is maintained for backward compatibility.
#[deprecated(
    since = "0.1.0",
    note = "Use Lookback::new(LookbackDirection::Call, ...) instead"
)]
pub type LookbackCall = Lookback;

/// Legacy type alias for lookback put option.
///
/// # Deprecated
///
/// Use [`Lookback::new(LookbackDirection::Put, ...)`] instead for new code.
/// This alias is maintained for backward compatibility.
#[deprecated(
    since = "0.1.0",
    note = "Use Lookback::new(LookbackDirection::Put, ...) instead"
)]
pub type LookbackPut = Lookback;

/// Floating strike lookback call.
///
/// Payoff: (S_T - S_min) × N
///
/// The strike "floats" to the minimum observed price.
#[derive(Clone, Debug)]
pub struct FloatingStrikeLookbackCall {
    /// Notional amount
    pub notional: f64,
    /// Time step index for maturity
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
    fn on_event(&mut self, state: &mut PathState) {
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
    fn test_lookback_call_unified() {
        let mut lookback = Lookback::new(LookbackDirection::Call, 100.0, 1.0, 10);

        // Simulate path: max = 120
        lookback.on_event(&mut create_state(0, 100.0));
        lookback.on_event(&mut create_state(5, 120.0));
        lookback.on_event(&mut create_state(10, 110.0));

        let value = lookback.value(Currency::USD);
        // max(120 - 100, 0) = 20
        assert_eq!(value.amount(), 20.0);
        assert_eq!(lookback.extreme_spot, 120.0);
    }

    #[test]
    fn test_lookback_put_unified() {
        let mut lookback = Lookback::new(LookbackDirection::Put, 100.0, 1.0, 10);

        // Simulate path: min = 80
        lookback.on_event(&mut create_state(0, 100.0));
        lookback.on_event(&mut create_state(5, 80.0));
        lookback.on_event(&mut create_state(10, 90.0));

        let value = lookback.value(Currency::USD);
        // max(100 - 80, 0) = 20
        assert_eq!(value.amount(), 20.0);
        assert_eq!(lookback.extreme_spot, 80.0);
    }

    #[test]
    fn test_lookback_call_out_of_money() {
        let mut lookback = Lookback::new(LookbackDirection::Call, 150.0, 1.0, 10);

        // Path never exceeds strike
        lookback.on_event(&mut create_state(0, 100.0));
        lookback.on_event(&mut create_state(5, 120.0));
        lookback.on_event(&mut create_state(10, 110.0));

        let value = lookback.value(Currency::USD);
        // max(120 - 150, 0) = 0
        assert_eq!(value.amount(), 0.0);
    }

    #[test]
    fn test_lookback_put_out_of_money() {
        let mut lookback = Lookback::new(LookbackDirection::Put, 50.0, 1.0, 10);

        // Path never goes below strike
        lookback.on_event(&mut create_state(0, 100.0));
        lookback.on_event(&mut create_state(5, 80.0));
        lookback.on_event(&mut create_state(10, 90.0));

        let value = lookback.value(Currency::USD);
        // max(50 - 80, 0) = 0
        assert_eq!(value.amount(), 0.0);
    }

    #[test]
    fn test_lookback_call_reset() {
        let mut lookback = Lookback::new(LookbackDirection::Call, 100.0, 1.0, 10);

        lookback.on_event(&mut create_state(0, 100.0));
        lookback.on_event(&mut create_state(5, 120.0));
        assert_eq!(lookback.extreme_spot, 120.0);

        lookback.reset();
        assert_eq!(lookback.extreme_spot, f64::NEG_INFINITY);
    }

    #[test]
    fn test_lookback_put_reset() {
        let mut lookback = Lookback::new(LookbackDirection::Put, 100.0, 1.0, 10);

        lookback.on_event(&mut create_state(0, 100.0));
        lookback.on_event(&mut create_state(5, 80.0));
        assert_eq!(lookback.extreme_spot, 80.0);

        lookback.reset();
        assert_eq!(lookback.extreme_spot, f64::INFINITY);
    }

    #[test]
    fn test_lookback_with_notional() {
        let mut call = Lookback::new(LookbackDirection::Call, 100.0, 2.5, 10);
        let mut put = Lookback::new(LookbackDirection::Put, 100.0, 2.5, 10);

        // Call path: max = 120
        call.on_event(&mut create_state(0, 100.0));
        call.on_event(&mut create_state(5, 120.0));

        // Put path: min = 80
        put.on_event(&mut create_state(0, 100.0));
        put.on_event(&mut create_state(5, 80.0));

        // Call: (120 - 100) * 2.5 = 50
        assert_eq!(call.value(Currency::USD).amount(), 50.0);

        // Put: (100 - 80) * 2.5 = 50
        assert_eq!(put.value(Currency::USD).amount(), 50.0);
    }

    #[test]
    fn test_floating_strike_lookback() {
        let mut lookback = FloatingStrikeLookbackCall::new(1.0, 10);

        // Path: starts 100, min 90, ends 110
        lookback.on_event(&mut create_state(0, 100.0));
        lookback.on_event(&mut create_state(5, 90.0));
        lookback.on_event(&mut create_state(10, 110.0));

        let value = lookback.value(Currency::USD);
        // 110 - 90 = 20
        assert_eq!(value.amount(), 20.0);
    }

    #[test]
    #[allow(deprecated)]
    fn test_backward_compatibility_type_aliases() {
        // Test that legacy type aliases still work
        let mut call = LookbackCall::new(LookbackDirection::Call, 100.0, 1.0, 10);
        let mut put = LookbackPut::new(LookbackDirection::Put, 100.0, 1.0, 10);

        call.on_event(&mut create_state(0, 100.0));
        call.on_event(&mut create_state(5, 120.0));

        put.on_event(&mut create_state(0, 100.0));
        put.on_event(&mut create_state(5, 80.0));

        assert_eq!(call.value(Currency::USD).amount(), 20.0);
        assert_eq!(put.value(Currency::USD).amount(), 20.0);
    }
}
