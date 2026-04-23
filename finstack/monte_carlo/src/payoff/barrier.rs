//! Barrier option payoffs with Brownian bridge correction.
//!
//! Implements knock-in and knock-out barrier options with:
//! - Discrete monitoring with bridge correction
//! - Gobet-Miri barrier adjustment
//! - Up and down barriers
//! - Rebate support (paid at maturity)
//!
//! # Local volatility under stochastic-vol models
//!
//! The bridge correction and the Gobet-Miri adjustment both need an
//! instantaneous volatility to turn a (spot, next_spot) pair into a barrier
//! hit probability. When the [`PathState`] carries a stochastic variance
//! (e.g. Heston's `VARIANCE` key), [`BarrierOptionPayoff::on_event`]
//! substitutes `sqrt(variance)` for the configured flat [`BarrierOptionPayoff::sigma`].
//! This makes the reported payoff consistent with the path-level dynamics
//! under stochastic-vol models; for deterministic-vol processes the state
//! carries no variance entry and the configured sigma is used unchanged.

use super::super::barriers::bridge::{check_barrier_hit, BarrierDirection};
use super::super::barriers::corrections::gobet_miri_adjusted_barrier;
use crate::time_grid::TimeGrid;
use crate::traits::PathState;
use crate::traits::Payoff;
use finstack_core::currency::Currency;
use finstack_core::money::Money;

/// Vanilla option kind for barrier payoff evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum OptionKind {
    /// Call option: max(S-K, 0)
    Call,
    /// Put option: max(K-S, 0)
    Put,
}

/// Barrier option type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
#[derive(Debug, Clone)]
pub struct BarrierOptionPayoff {
    /// Strike price
    pub strike: f64,
    /// Barrier level
    pub barrier: f64,
    /// Barrier type
    pub barrier_type: BarrierType,
    /// Option type (Call/Put)
    pub option_type: OptionKind,
    /// Rebate amount (paid at maturity if option deactivated/not-activated)
    pub rebate: Option<f64>,
    /// Notional
    pub notional: f64,
    /// Maturity step
    pub maturity_step: usize,
    /// Volatility (for bridge correction)
    pub sigma: f64,
    /// Time steps for each monitoring interval (for bridge correction)
    pub step_dts: Vec<f64>,
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
        option_type: OptionKind,
        rebate: Option<f64>,
        notional: f64,
        maturity_step: usize,
        sigma: f64,
        time_grid: &TimeGrid,
        use_gobet_miri: bool,
    ) -> Self {
        Self {
            strike,
            barrier,
            barrier_type,
            option_type,
            rebate,
            notional,
            maturity_step,
            sigma,
            step_dts: time_grid.dts().to_vec(),
            use_gobet_miri,
            terminal_spot: 0.0,
            barrier_hit: false,
            previous_spot: 0.0,
        }
    }

    /// Get effective barrier level (with Gobet-Miri adjustment if enabled).
    ///
    /// `local_sigma` is the local per-step volatility to use in the adjustment.
    /// Under deterministic-vol models this equals [`Self::sigma`]; under
    /// stochastic-vol models [`on_event`](Payoff::on_event) substitutes
    /// `sqrt(state.variance())` when the path state exposes it.
    fn effective_barrier(&self, dt: f64, local_sigma: f64) -> f64 {
        if self.use_gobet_miri {
            gobet_miri_adjusted_barrier(self.barrier, local_sigma, dt, !self.barrier_type.is_up())
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

        if state.step == 0 {
            self.previous_spot = current_spot;

            let breached = match self.barrier_type.direction() {
                BarrierDirection::Up => current_spot >= self.barrier,
                BarrierDirection::Down => current_spot <= self.barrier,
            };
            if breached {
                self.barrier_hit = true;
            }
            return;
        }

        // Check barrier hit using bridge correction.
        if !self.barrier_hit {
            // Use independent uniform random from PathState for bridge sampling
            // so the barrier hit probability is statistically correct.
            let uniform_random = state.uniform_random();
            let dt = self.step_dts.get(state.step - 1).copied().unwrap_or(0.0);
            // Prefer the process's local instantaneous variance when available
            // (e.g. Heston). For deterministic-vol processes the state carries
            // no variance entry and we fall back to the configured flat sigma.
            let local_sigma = match state.variance() {
                Some(v) if v.is_finite() && v > 0.0 => v.sqrt(),
                _ => self.sigma,
            };
            let effective_barrier = self.effective_barrier(dt, local_sigma);

            let hit = check_barrier_hit(
                self.previous_spot,
                current_spot,
                effective_barrier,
                self.barrier_type.direction(),
                local_sigma,
                dt,
                uniform_random,
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
                OptionKind::Call => (self.terminal_spot - self.strike).max(0.0),
                OptionKind::Put => (self.strike - self.terminal_spot).max(0.0),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time_grid::TimeGrid;
    use crate::traits::state_keys;

    fn create_path_state(step: usize, time: f64, spot: f64, uniform_random: f64) -> PathState {
        let mut state = PathState::new(step, time);
        state.set(state_keys::SPOT, spot);
        state.set_uniform_random(uniform_random);
        state
    }

    #[test]
    fn test_barrier_put_payoff() {
        let mut barrier_put = BarrierOptionPayoff::new(
            100.0,
            120.0,
            BarrierType::UpAndOut,
            OptionKind::Put,
            None,
            1.0,
            10,
            0.2,
            &TimeGrid::uniform(1.0, 10).expect("grid should build"),
            false,
        );

        // Path that never hits barrier (active)
        for step in 0..=10 {
            let spot = 90.0; // Below barrier, below strike (ITM put)
            let mut state = create_path_state(step, step as f64 * 0.1, spot, 0.5);
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
            OptionKind::Call,
            Some(rebate),
            1.0,
            10,
            0.2,
            &TimeGrid::uniform(1.0, 10).expect("grid should build"),
            false,
        );

        // Hit barrier
        let mut s1 = create_path_state(0, 0.0, 105.0, 0.5);
        barrier_call.on_event(&mut s1);
        let mut s2 = create_path_state(1, 0.1, 125.0, 0.5); // Hit
        barrier_call.on_event(&mut s2);
        let mut s3 = create_path_state(10, 1.0, 130.0, 0.5); // Terminal
        barrier_call.on_event(&mut s3);

        // Should get rebate
        let value = barrier_call.value(Currency::USD);
        assert_eq!(value.amount(), 5.0);
    }

    #[test]
    fn test_barrier_uses_path_variance_when_present() {
        // Two barrier payoffs with identical parameters but different fallback
        // sigmas. When the path state carries a stochastic variance, both
        // must agree because the payoff consumes sqrt(variance) instead of
        // self.sigma.
        let time_grid = TimeGrid::uniform(1.0, 4).expect("grid should build");

        let build = |flat_sigma: f64| {
            BarrierOptionPayoff::new(
                100.0,
                120.0,
                BarrierType::UpAndOut,
                OptionKind::Call,
                None,
                1.0,
                4,
                flat_sigma,
                &time_grid,
                true,
            )
        };

        let mut p_low = build(0.01);
        let mut p_high = build(1.0);

        let stoch_var = 0.04_f64;
        let feed = |payoff: &mut BarrierOptionPayoff| {
            for (step, spot) in [
                (0usize, 95.0),
                (1, 105.0),
                (2, 115.0),
                (3, 110.0),
                (4, 118.0),
            ] {
                let mut state = PathState::new(step, step as f64 * 0.25);
                state.set(state_keys::SPOT, spot);
                state.set(state_keys::VARIANCE, stoch_var);
                state.set_uniform_random(0.5);
                payoff.on_event(&mut state);
            }
        };

        feed(&mut p_low);
        feed(&mut p_high);

        let v_low = p_low.value(Currency::USD).amount();
        let v_high = p_high.value(Currency::USD).amount();
        assert_eq!(
            v_low, v_high,
            "payoff must use sqrt(variance) from PathState when present, \
             ignoring the fallback self.sigma",
        );
    }

    #[test]
    fn test_barrier_falls_back_to_self_sigma_without_variance() {
        // When variance is absent from PathState, the payoff must use
        // self.sigma for the bridge/Gobet-Miri adjustment. Two payoffs with
        // different flat sigmas should behave differently.
        let time_grid = TimeGrid::uniform(1.0, 4).expect("grid should build");

        let build = |flat_sigma: f64| {
            BarrierOptionPayoff::new(
                100.0,
                120.0,
                BarrierType::UpAndOut,
                OptionKind::Call,
                None,
                1.0,
                4,
                flat_sigma,
                &time_grid,
                true,
            )
        };

        let mut p_low = build(0.01);
        let mut p_high = build(1.0);

        let feed = |payoff: &mut BarrierOptionPayoff| {
            for (step, spot) in [
                (0usize, 95.0),
                (1, 105.0),
                (2, 115.0),
                (3, 110.0),
                (4, 118.0),
            ] {
                let mut state = PathState::new(step, step as f64 * 0.25);
                state.set(state_keys::SPOT, spot);
                state.set_uniform_random(0.5);
                payoff.on_event(&mut state);
            }
        };

        feed(&mut p_low);
        feed(&mut p_high);

        let v_low = p_low.value(Currency::USD).amount();
        let v_high = p_high.value(Currency::USD).amount();
        // At least one of the two must differ — for a near-barrier path the
        // Gobet-Miri adjustment is very sensitive to sigma, so the payoff
        // should not be identical when the flat sigma is the only input.
        assert!(
            (v_low - v_high).abs() > 0.0 || v_low + v_high == 0.0,
            "without variance in PathState the payoff must respond to self.sigma; \
             got v_low={v_low}, v_high={v_high}"
        );
    }

    #[test]
    fn test_barrier_uses_step_dt_for_irregular_grid() {
        let time_grid = TimeGrid::from_times(vec![0.0, 0.2, 1.0]).expect("grid should build");
        let mut barrier_call = BarrierOptionPayoff::new(
            80.0,
            100.0,
            BarrierType::UpAndOut,
            OptionKind::Call,
            None,
            1.0,
            2,
            0.2,
            &time_grid,
            false,
        );

        let mut s0 = create_path_state(0, 0.0, 90.0, 0.65);
        barrier_call.on_event(&mut s0);
        let mut s1 = create_path_state(1, 0.2, 95.0, 0.65);
        barrier_call.on_event(&mut s1);
        let mut s2 = create_path_state(2, 1.0, 90.0, 0.65);
        barrier_call.on_event(&mut s2);

        assert_eq!(barrier_call.value(Currency::USD).amount(), 0.0);
    }
}
