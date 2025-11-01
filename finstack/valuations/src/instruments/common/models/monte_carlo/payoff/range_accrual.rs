//! Range accrual payoff for Monte Carlo pricing.
//!
//! Range accrual products pay coupons based on the number of days the underlying
//! stays within a specified range.

use crate::instruments::common::mc::traits::PathState;
use crate::instruments::common::models::monte_carlo::traits::Payoff;
use finstack_core::currency::Currency;
use finstack_core::money::Money;

/// Range accrual payoff.
///
/// Accumulates coupon payments based on how many observation dates the
/// underlying spot price stays within a specified range [lower_bound, upper_bound].
///
/// # Payoff Structure
///
/// Payoff = coupon_rate * (days_in_range / total_days) * notional
///
/// where days_in_range is the count of observation dates where lower_bound <= S_t <= upper_bound.
#[derive(Clone, Debug)]
pub struct RangeAccrualPayoff {
    /// Observation dates (time in years, must be sorted)
    pub observation_dates: Vec<f64>,
    /// Lower bound of the range
    pub lower_bound: f64,
    /// Upper bound of the range
    pub upper_bound: f64,
    /// Coupon rate (e.g., 0.08 for 8% annual)
    pub coupon_rate: f64,
    /// Notional amount
    pub notional: f64,
    /// Currency
    pub currency: Currency,

    // State variables (tracked during path simulation)
    /// Number of observation dates where spot was in range
    days_in_range: usize,
    /// Total number of observation dates checked
    total_observations: usize,
    /// Index of next observation date to check
    next_obs_idx: usize,
}

impl RangeAccrualPayoff {
    /// Create a new range accrual payoff.
    ///
    /// # Arguments
    ///
    /// * `observation_dates` - Dates when range check occurs (must be sorted)
    /// * `lower_bound` - Lower bound of the range
    /// * `upper_bound` - Upper bound of the range (must be > lower_bound)
    /// * `coupon_rate` - Annual coupon rate
    /// * `notional` - Notional amount
    /// * `currency` - Currency
    pub fn new(
        observation_dates: Vec<f64>,
        lower_bound: f64,
        upper_bound: f64,
        coupon_rate: f64,
        notional: f64,
        currency: Currency,
    ) -> Self {
        assert!(
            lower_bound < upper_bound,
            "Lower bound must be less than upper bound"
        );
        assert!(coupon_rate >= 0.0, "Coupon rate must be non-negative");

        // Verify observation dates are sorted
        for i in 1..observation_dates.len() {
            assert!(
                observation_dates[i - 1] < observation_dates[i],
                "Observation dates must be sorted"
            );
        }

        Self {
            observation_dates,
            lower_bound,
            upper_bound,
            coupon_rate,
            notional,
            currency,
            days_in_range: 0,
            total_observations: 0,
            next_obs_idx: 0,
        }
    }

    /// Check if spot is within range.
    fn is_in_range(&self, spot: f64) -> bool {
        spot >= self.lower_bound && spot <= self.upper_bound
    }
}

impl Payoff for RangeAccrualPayoff {
    fn on_event(&mut self, state: &PathState) {
        if self.next_obs_idx < self.observation_dates.len() {
            let target_date = self.observation_dates[self.next_obs_idx];

            // Check if we're at an observation date
            if (state.time - target_date).abs() < 1e-6 || state.time >= target_date {
                if let Some(spot) = state.spot() {
                    if self.is_in_range(spot) {
                        self.days_in_range += 1;
                    }
                    self.total_observations += 1;
                    self.next_obs_idx += 1;
                }
            }
        }
    }

    fn value(&self, currency: Currency) -> Money {
        if self.total_observations == 0 {
            return Money::new(0.0, currency);
        }

        // Compute accrual fraction: days_in_range / total_days
        let accrual_fraction = self.days_in_range as f64 / self.total_observations as f64;

        // Payoff = coupon_rate * accrual_fraction * notional
        let payoff = self.coupon_rate * accrual_fraction * self.notional;

        Money::new(payoff, currency)
    }

    fn reset(&mut self) {
        self.days_in_range = 0;
        self.total_observations = 0;
        self.next_obs_idx = 0;
    }
}

#[cfg(test)]
mod tests {
    use crate::instruments::common::mc::traits::state_keys;
    use super::*;

    #[test]
    fn test_range_accrual_creation() {
        let observation_dates = vec![0.25, 0.5, 0.75, 1.0];
        let accrual = RangeAccrualPayoff::new(
            observation_dates,
            95.0,  // Lower bound
            105.0, // Upper bound
            0.08,  // 8% coupon
            100_000.0,
            Currency::USD,
        );

        assert_eq!(accrual.observation_dates.len(), 4);
        assert_eq!(accrual.lower_bound, 95.0);
        assert_eq!(accrual.upper_bound, 105.0);
    }

    #[test]
    fn test_range_accrual_in_range() {
        let observation_dates = vec![0.25, 0.5];
        let mut accrual = RangeAccrualPayoff::new(
            observation_dates,
            95.0,
            105.0,
            0.08,
            100_000.0,
            Currency::USD,
        );

        // Both observations in range
        let mut state1 = PathState::new(10, 0.25);
        state1.set(state_keys::SPOT, 100.0);
        accrual.on_event(&state1);

        let mut state2 = PathState::new(20, 0.5);
        state2.set(state_keys::SPOT, 98.0);
        accrual.on_event(&state2);

        let value = accrual.value(Currency::USD);
        // 2 days in range / 2 total = 1.0 fraction
        // Payoff = 0.08 * 1.0 * 100_000 = 8_000
        assert!((value.amount() - 8_000.0).abs() < 1e-6);
    }

    #[test]
    fn test_range_accrual_partial() {
        let observation_dates = vec![0.25, 0.5, 0.75];
        let mut accrual = RangeAccrualPayoff::new(
            observation_dates,
            95.0,
            105.0,
            0.08,
            100_000.0,
            Currency::USD,
        );

        // Only 2 out of 3 in range
        let mut state1 = PathState::new(10, 0.25);
        state1.set(state_keys::SPOT, 100.0); // In range
        accrual.on_event(&state1);

        let mut state2 = PathState::new(20, 0.5);
        state2.set(state_keys::SPOT, 110.0); // Out of range
        accrual.on_event(&state2);

        let mut state3 = PathState::new(30, 0.75);
        state3.set(state_keys::SPOT, 98.0); // In range
        accrual.on_event(&state3);

        let value = accrual.value(Currency::USD);
        // 2 days in range / 3 total = 2/3 fraction
        // Payoff = 0.08 * (2/3) * 100_000 = 5_333.33...
        assert!((value.amount() - 5_333.333333).abs() < 1.0);
    }

    #[test]
    fn test_range_accrual_boundary() {
        let observation_dates = vec![0.25];
        let mut accrual = RangeAccrualPayoff::new(
            observation_dates,
            95.0,
            105.0,
            0.08,
            100_000.0,
            Currency::USD,
        );

        // Exactly at lower boundary (should be in range)
        let mut state = PathState::new(10, 0.25);
        state.set(state_keys::SPOT, 95.0);
        accrual.on_event(&state);

        assert_eq!(accrual.days_in_range, 1);

        // Exactly at upper boundary (should be in range)
        accrual.reset();
        state.set(state_keys::SPOT, 105.0);
        accrual.on_event(&state);

        assert_eq!(accrual.days_in_range, 1);
    }

    #[test]
    fn test_range_accrual_reset() {
        let observation_dates = vec![0.25];
        let mut accrual = RangeAccrualPayoff::new(
            observation_dates,
            95.0,
            105.0,
            0.08,
            100_000.0,
            Currency::USD,
        );

        let mut state = PathState::new(10, 0.25);
        state.set(state_keys::SPOT, 100.0);
        accrual.on_event(&state);

        assert_eq!(accrual.days_in_range, 1);
        assert_eq!(accrual.total_observations, 1);

        accrual.reset();

        assert_eq!(accrual.days_in_range, 0);
        assert_eq!(accrual.total_observations, 0);
        assert_eq!(accrual.next_obs_idx, 0);
    }
}
