//! Barrier option payoffs with Brownian bridge correction.
//!
//! Implements knock-in and knock-out barrier options with:
//! - Discrete monitoring with bridge correction
//! - Gobet-Miri barrier adjustment
//! - Up and down barriers
//! - Rebate support (paid at maturity)

use super::super::barriers::bridge::{check_barrier_hit, BarrierDirection};
use super::super::barriers::corrections::gobet_miri_adjusted_barrier;
use crate::instruments::common::mc::traits::PathState;
use crate::instruments::common::models::monte_carlo::traits::Payoff;
use crate::instruments::OptionType;
use finstack_core::currency::Currency;
use finstack_core::money::Money;

/// Barrier option type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BarrierType {
    /// Up-and-out: option knocked out if S >= B
    UpAndOut,
    /// Up-and-in: option activated if S >= B
    UpAndIn,
    /// Down-and-out: option knocked out if S <= B
    DownAndOut,
    /// Down-and-in: option activated if S <= B
    DownAndIn,
}

impl BarrierType {
    /// Check if this is a knock-out barrier.
    pub fn is_knock_out(&self) -> bool {
        matches!(self, BarrierType::UpAndOut | BarrierType::DownAndOut)
    }

    /// Check if this is a knock-in barrier.
    pub fn is_knock_in(&self) -> bool {
        !self.is_knock_out()
    }

    /// Get barrier direction.
    pub fn direction(&self) -> BarrierDirection {
        match self {
            BarrierType::UpAndOut | BarrierType::UpAndIn => BarrierDirection::Up,
            BarrierType::DownAndOut | BarrierType::DownAndIn => BarrierDirection::Down,
        }
    }

    /// Check if this is an up barrier.
    pub fn is_up(&self) -> bool {
        matches!(self, BarrierType::UpAndOut | BarrierType::UpAndIn)
    }
}

/// Barrier option payoff with bridge correction.
///
/// A generic barrier option (Call or Put) with a barrier that can knock in or out,
/// and an optional rebate paid at maturity if the option is not active (e.g. knocked out).
#[derive(Clone, Debug)]
pub struct BarrierOptionPayoff {
    /// Strike price
    pub strike: f64,
    /// Barrier level
    pub barrier: f64,
    /// Barrier type
    pub barrier_type: BarrierType,
    /// Option type (Call/Put)
    pub option_type: OptionType,
    /// Rebate amount (paid at maturity if option deactivated/not-activated)
    pub rebate: Option<f64>,
    /// Notional
    pub notional: f64,
    /// Maturity step
    pub maturity_step: usize,
    /// Volatility (for bridge correction)
    pub sigma: f64,
    /// Time step (for bridge correction)
    pub dt: f64,
    /// Use Gobet-Miri adjustment
    pub use_gobet_miri: bool,

    // State
    terminal_spot: f64,
    barrier_hit: bool,
    previous_spot: f64,
}

impl BarrierOptionPayoff {
    /// Create a new barrier option payoff.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        strike: f64,
        barrier: f64,
        barrier_type: BarrierType,
        option_type: OptionType,
        rebate: Option<f64>,
        notional: f64,
        maturity_step: usize,
        sigma: f64,
        time_to_maturity: f64,
        use_gobet_miri: bool,
    ) -> Self {
        let dt = time_to_maturity / maturity_step as f64;

        Self {
            strike,
            barrier,
            barrier_type,
            option_type,
            rebate,
            notional,
            maturity_step,
            sigma,
            dt,
            use_gobet_miri,
            terminal_spot: 0.0,
            barrier_hit: false,
            previous_spot: 0.0,
        }
    }

    /// Get effective barrier level (with Gobet-Miri adjustment if enabled).
    pub fn effective_barrier(&self) -> f64 {
        if self.use_gobet_miri {
            gobet_miri_adjusted_barrier(
                self.barrier,
                self.sigma,
                self.dt,
                !self.barrier_type.is_up(),
            )
        } else {
            self.barrier
        }
    }

    /// Check if option is active based on barrier status.
    fn is_active(&self) -> bool {
        match self.barrier_type {
            BarrierType::UpAndOut | BarrierType::DownAndOut => {
                // Knock-out: active if barrier NOT hit
                !self.barrier_hit
            }
            BarrierType::UpAndIn | BarrierType::DownAndIn => {
                // Knock-in: active if barrier WAS hit
                self.barrier_hit
            }
        }
    }
}

impl Payoff for BarrierOptionPayoff {
    fn on_event(&mut self, state: &mut PathState) {
        let current_spot = state.spot().unwrap_or(0.0);

        // Check barrier on first event
        if state.step == 0 {
            self.previous_spot = current_spot;
            return;
        }

        // Check barrier hit using bridge correction
        if !self.barrier_hit {
            // Generate pseudo-random for bridge check (use spot as seed - not ideal but simple)
            let pseudo_random = ((current_spot * 12345.0).fract()).abs();

            let effective_barrier = self.effective_barrier();

            let hit = check_barrier_hit(
                self.previous_spot,
                current_spot,
                effective_barrier,
                self.barrier_type.direction(),
                self.sigma,
                self.dt,
                pseudo_random,
            );

            if hit {
                self.barrier_hit = true;
            }
        }

        // Update state
        self.previous_spot = current_spot;

        // Capture terminal spot at maturity
        if state.step == self.maturity_step {
            self.terminal_spot = current_spot;
        }
    }

    fn value(&self, currency: Currency) -> Money {
        if self.is_active() {
            // Standard vanilla payoff
            let intrinsic = match self.option_type {
                OptionType::Call => (self.terminal_spot - self.strike).max(0.0),
                OptionType::Put => (self.strike - self.terminal_spot).max(0.0),
            };
            Money::new(intrinsic * self.notional, currency)
        } else {
            // Rebate payment
            let rebate_amount = self.rebate.unwrap_or(0.0);
            Money::new(rebate_amount * self.notional, currency)
        }
    }

    fn reset(&mut self) {
        self.terminal_spot = 0.0;
        self.barrier_hit = false;
        self.previous_spot = 0.0;
    }
}

/// Compatibility alias for tests or other modules (deprecated)
pub type BarrierCall = BarrierOptionPayoff;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruments::common::mc::traits::state_keys;

    fn create_path_state(step: usize, spot: f64) -> PathState {
        let mut state = PathState::new(step, step as f64 * 0.01);
        state.set(state_keys::SPOT, spot);
        state
    }

    #[test]
    fn test_barrier_put_payoff() {
        let mut barrier_put = BarrierOptionPayoff::new(
            100.0,
            120.0,
            BarrierType::UpAndOut,
            OptionType::Put,
            None,
            1.0,
            10,
            0.2,
            1.0,
            false,
        );

        // Path that never hits barrier (active)
        for step in 0..=10 {
            let spot = 90.0; // Below barrier, below strike (ITM put)
            let mut state = create_path_state(step, spot);
            barrier_put.on_event(&mut state);
        }

        // Should get put payoff (100 - 90 = 10)
        let value = barrier_put.value(Currency::USD);
        assert_eq!(value.amount(), 10.0);
    }

    #[test]
    fn test_barrier_rebate() {
        let rebate = 5.0;
        let mut barrier_call = BarrierOptionPayoff::new(
            100.0,
            120.0,
            BarrierType::UpAndOut,
            OptionType::Call,
            Some(rebate),
            1.0,
            10,
            0.2,
            1.0,
            false,
        );

        // Hit barrier
        let mut s1 = create_path_state(0, 105.0);
        barrier_call.on_event(&mut s1);
        let mut s2 = create_path_state(1, 125.0); // Hit
        barrier_call.on_event(&mut s2);
        let mut s3 = create_path_state(10, 130.0); // Terminal
        barrier_call.on_event(&mut s3);

        // Should get rebate
        let value = barrier_call.value(Currency::USD);
        assert_eq!(value.amount(), 5.0);
    }
}
