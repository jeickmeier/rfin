//! Barrier option payoffs with Brownian bridge correction.
//!
//! Implements knock-in and knock-out barrier options with:
//! - Discrete monitoring with bridge correction
//! - Gobet-Miri barrier adjustment
//! - Up and down barriers

use super::super::barriers::bridge::{check_barrier_hit, BarrierDirection};
use super::super::barriers::corrections::gobet_miri_adjusted_barrier;
use super::super::traits::{Payoff, PathState};
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

/// Barrier call option with bridge correction.
///
/// A call option with a barrier that can knock in or out.
#[derive(Clone, Debug)]
pub struct BarrierCall {
    /// Strike price
    pub strike: f64,
    /// Barrier level
    pub barrier: f64,
    /// Barrier type
    pub barrier_type: BarrierType,
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

impl BarrierCall {
    /// Create a new barrier call option.
    ///
    /// # Arguments
    ///
    /// * `strike` - Strike price
    /// * `barrier` - Barrier level
    /// * `barrier_type` - Type of barrier
    /// * `notional` - Notional amount
    /// * `maturity_step` - Maturity step index
    /// * `sigma` - Volatility (for corrections)
    /// * `time_to_maturity` - Time to maturity
    /// * `use_gobet_miri` - Apply Gobet-Miri adjustment
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        strike: f64,
        barrier: f64,
        barrier_type: BarrierType,
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

impl Payoff for BarrierCall {
    fn on_event(&mut self, state: &PathState) {
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
        // Only pay if option is active
        if !self.is_active() {
            return Money::new(0.0, currency);
        }

        // Standard call payoff
        let intrinsic = (self.terminal_spot - self.strike).max(0.0);
        Money::new(intrinsic * self.notional, currency)
    }

    fn reset(&mut self) {
        self.terminal_spot = 0.0;
        self.barrier_hit = false;
        self.previous_spot = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::super::traits::state_keys;

    fn create_path_state(step: usize, spot: f64) -> PathState {
        let mut state = PathState::new(step, step as f64 * 0.01);
        state.set(state_keys::SPOT, spot);
        state
    }

    #[test]
    fn test_barrier_call_no_hit() {
        let mut barrier_call = BarrierCall::new(
            100.0,
            120.0,
            BarrierType::UpAndOut,
            1.0,
            10,
            0.2,
            1.0,
            false,
        );

        // Simulate path that never hits barrier
        for step in 0..=10 {
            let spot = 105.0; // Below barrier
            let state = create_path_state(step, spot);
            barrier_call.on_event(&state);
        }

        // Should get standard call payoff (105 - 100 = 5)
        let value = barrier_call.value(Currency::USD);
        assert_eq!(value.amount(), 5.0);
    }

    #[test]
    fn test_barrier_call_knocked_out() {
        let mut barrier_call = BarrierCall::new(
            100.0,
            110.0,
            BarrierType::UpAndOut,
            1.0,
            10,
            0.2,
            1.0,
            false,
        );

        // Simulate path that hits barrier
        barrier_call.on_event(&create_path_state(0, 105.0));
        barrier_call.on_event(&create_path_state(1, 115.0)); // Hit barrier
        barrier_call.on_event(&create_path_state(10, 120.0)); // Terminal

        // Should get zero (knocked out)
        let value = barrier_call.value(Currency::USD);
        assert_eq!(value.amount(), 0.0);
    }

    #[test]
    fn test_barrier_call_knock_in() {
        let mut barrier_call = BarrierCall::new(
            100.0,
            110.0,
            BarrierType::UpAndIn,
            1.0,
            10,
            0.2,
            1.0,
            false,
        );

        // Path that never hits barrier
        for step in 0..=10 {
            let spot = 105.0;
            barrier_call.on_event(&create_path_state(step, spot));
        }

        // Should get zero (never knocked in)
        let value = barrier_call.value(Currency::USD);
        assert_eq!(value.amount(), 0.0);
    }

    #[test]
    fn test_barrier_call_knock_in_activated() {
        let mut barrier_call = BarrierCall::new(
            100.0,
            110.0,
            BarrierType::UpAndIn,
            1.0,
            10,
            0.2,
            1.0,
            false,
        );

        // Path that hits barrier
        barrier_call.on_event(&create_path_state(0, 105.0));
        barrier_call.on_event(&create_path_state(1, 115.0)); // Knock in
        barrier_call.on_event(&create_path_state(10, 120.0)); // Terminal

        // Should get call payoff (120 - 100 = 20)
        let value = barrier_call.value(Currency::USD);
        assert_eq!(value.amount(), 20.0);
    }

    #[test]
    fn test_gobet_miri_adjustment() {
        let barrier_no_adj = BarrierCall::new(
            100.0,
            110.0,
            BarrierType::UpAndOut,
            1.0,
            252,
            0.2,
            1.0,
            false,
        );

        let barrier_with_adj = BarrierCall::new(
            100.0,
            110.0,
            BarrierType::UpAndOut,
            1.0,
            252,
            0.2,
            1.0,
            true,
        );

        let eff_no_adj = barrier_no_adj.effective_barrier();
        let eff_with_adj = barrier_with_adj.effective_barrier();

        // With adjustment, up barrier should be higher
        assert!(eff_with_adj > eff_no_adj);
        assert_eq!(eff_no_adj, 110.0);
    }
}
