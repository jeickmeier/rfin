//! Autocallable structured product payoffs for Monte Carlo pricing.
//!
//! Autocallable products have early redemption features where the option
//! is automatically called (redeemed) if certain barrier conditions are met
//! at observation dates.

use super::super::traits::{PathState, Payoff};
use finstack_core::currency::Currency;
use finstack_core::money::Money;

/// Final payoff type for autocallable products.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FinalPayoffType {
    /// Capital protection: max(floor, participation * min(S_T/S_0, cap))
    CapitalProtection {
        /// Floor level (e.g., 0.9 for 90% protection)
        floor: f64,
    },
    /// Participation: 1 + participation_rate * max(0, S_T/S_0 - 1)
    Participation {
        /// Participation rate (e.g., 1.0 for 100% participation)
        rate: f64,
    },
    /// Knock-in put: Put option if barrier breached, otherwise return principal
    KnockInPut {
        /// Put strike price
        strike: f64,
    },
}

/// Autocallable structured product payoff.
///
/// At each observation date, if spot >= autocall_barrier, the product
/// is redeemed early with coupon + principal.
///
/// If not autocalled, final payoff depends on FinalPayoffType and barriers.
#[derive(Clone, Debug)]
pub struct AutocallablePayoff {
    /// Observation dates (time in years from valuation)
    pub observation_dates: Vec<f64>,
    /// Autocall barrier levels at each observation date
    pub autocall_barriers: Vec<f64>,
    /// Coupon payments if autocalled at each date
    pub coupons: Vec<f64>,
    /// Final barrier level (for knock-in/knock-out)
    pub final_barrier: f64,
    /// Final payoff structure
    pub final_payoff_type: FinalPayoffType,
    /// Participation rate for final payoff
    pub participation_rate: f64,
    /// Cap level for returns (e.g., 1.2 for 20% cap)
    pub cap_level: f64,
    /// Notional amount
    pub notional: f64,
    /// Currency
    pub currency: Currency,
    /// Initial spot price
    pub initial_spot: f64,

    // State variables (tracked during path simulation)
    /// Index of observation date when autocalled (None if not autocalled)
    autocalled_at: Option<usize>,
    /// Minimum spot observed (for knock-in barriers)
    min_spot_observed: f64,
    /// Maximum spot observed (for knock-out barriers)
    max_spot_observed: f64,
    /// Final spot price
    final_spot: f64,
}

impl AutocallablePayoff {
    /// Create a new autocallable payoff.
    ///
    /// # Arguments
    ///
    /// * `observation_dates` - Dates when autocall barriers are checked (must be sorted)
    /// * `autocall_barriers` - Barrier levels at each observation date
    /// * `coupons` - Coupon payments if autocalled at each date
    /// * `final_barrier` - Barrier for final payoff (knock-in/knock-out)
    /// * `final_payoff_type` - Type of final payoff
    /// * `participation_rate` - Participation rate for final payoff
    /// * `cap_level` - Maximum return cap
    /// * `notional` - Notional amount
    /// * `currency` - Currency
    /// * `initial_spot` - Initial spot price S_0
    pub fn new(
        observation_dates: Vec<f64>,
        autocall_barriers: Vec<f64>,
        coupons: Vec<f64>,
        final_barrier: f64,
        final_payoff_type: FinalPayoffType,
        participation_rate: f64,
        cap_level: f64,
        notional: f64,
        currency: Currency,
        initial_spot: f64,
    ) -> Self {
        assert_eq!(
            observation_dates.len(),
            autocall_barriers.len(),
            "Observation dates and barriers must have same length"
        );
        assert_eq!(
            observation_dates.len(),
            coupons.len(),
            "Observation dates and coupons must have same length"
        );

        // Verify observation dates are sorted
        for i in 1..observation_dates.len() {
            assert!(
                observation_dates[i - 1] < observation_dates[i],
                "Observation dates must be sorted"
            );
        }

        Self {
            observation_dates,
            autocall_barriers,
            coupons,
            final_barrier,
            final_payoff_type,
            participation_rate,
            cap_level,
            notional,
            currency,
            initial_spot,
            autocalled_at: None,
            min_spot_observed: f64::INFINITY,
            max_spot_observed: f64::NEG_INFINITY,
            final_spot: 0.0,
        }
    }
}

impl Payoff for AutocallablePayoff {
    fn on_event(&mut self, state: &PathState) {
        let spot = state.spot().unwrap_or(0.0);

        // Track min/max for barrier checks
        self.min_spot_observed = self.min_spot_observed.min(spot);
        self.max_spot_observed = self.max_spot_observed.max(spot);

        // Check autocall at observation dates
        if self.autocalled_at.is_none() {
            for (idx, &obs_date) in self.observation_dates.iter().enumerate() {
                // Check if we're at or past this observation date and barrier is hit
                let is_at_obs_date = state.time >= obs_date || (state.time - obs_date).abs() < 1e-6;
                if is_at_obs_date && spot >= self.autocall_barriers[idx] {
                    self.autocalled_at = Some(idx);
                    break;
                }
            }
        }

        // Store final spot at maturity
        // Assume maturity is the last observation date (or can be set separately)
        if let Some(&last_date) = self.observation_dates.last() {
            if state.time >= last_date {
                self.final_spot = spot;
            }
        }
    }

    fn value(&self, currency: Currency) -> Money {
        // If autocalled early
        if let Some(idx) = self.autocalled_at {
            let coupon = self.coupons[idx];
            // Return coupon + principal
            return Money::new((coupon + 1.0) * self.notional, currency);
        }

        // Final payoff (not autocalled)
        let final_payoff = match self.final_payoff_type {
            FinalPayoffType::CapitalProtection { floor } => {
                let return_ratio = (self.final_spot / self.initial_spot).min(self.cap_level);
                floor.max(self.participation_rate * return_ratio)
            }
            FinalPayoffType::Participation { rate } => {
                1.0 + rate * ((self.final_spot / self.initial_spot - 1.0).max(0.0))
            }
            FinalPayoffType::KnockInPut { strike } => {
                if self.min_spot_observed <= self.final_barrier {
                    // Barrier breached, put option active
                    (strike - self.final_spot).max(0.0)
                } else {
                    1.0 // No barrier breach, return principal
                }
            }
        };

        Money::new(final_payoff * self.notional, currency)
    }

    fn reset(&mut self) {
        self.autocalled_at = None;
        self.min_spot_observed = f64::INFINITY;
        self.max_spot_observed = f64::NEG_INFINITY;
        self.final_spot = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::traits::state_keys;
    use super::*;

    #[test]
    fn test_autocallable_creation() {
        let observation_dates = vec![0.25, 0.5, 0.75, 1.0];
        let barriers = vec![1.05, 1.05, 1.05, 1.05];
        let coupons = vec![0.08, 0.08, 0.08, 0.10];

        let payoff = AutocallablePayoff::new(
            observation_dates,
            barriers,
            coupons,
            0.75, // Final barrier
            FinalPayoffType::CapitalProtection { floor: 0.9 },
            1.0,  // Participation rate
            1.2,  // Cap level
            100_000.0,
            Currency::USD,
            100.0, // Initial spot
        );

        assert_eq!(payoff.observation_dates.len(), 4);
        assert_eq!(payoff.initial_spot, 100.0);
        assert!(payoff.autocalled_at.is_none());
    }

    #[test]
    fn test_autocallable_early_exercise() {
        let observation_dates = vec![0.25, 0.5];
        let barriers = vec![1.05, 1.05];
        let coupons = vec![0.08, 0.10];

        let mut payoff = AutocallablePayoff::new(
            observation_dates,
            barriers,
            coupons,
            0.75,
            FinalPayoffType::CapitalProtection { floor: 0.9 },
            1.0,
            1.2,
            100_000.0,
            Currency::USD,
            100.0,
        );

        // Simulate first observation date with spot above barrier
        let mut state = PathState::new(10, 0.25);
        state.set(state_keys::SPOT, 106.0); // Above 105 barrier

        payoff.on_event(&state);

        assert_eq!(payoff.autocalled_at, Some(0));
        
        let value = payoff.value(Currency::USD);
        // Should be coupon (0.08) + principal (1.0) = 1.08 * notional
        assert!((value.amount() - 108_000.0).abs() < 1e-6);
    }

    #[test]
    fn test_autocallable_capital_protection() {
        let observation_dates = vec![1.0];
        let barriers = vec![1.20]; // Very high barrier, unlikely to hit
        let coupons = vec![0.0];

        let mut payoff = AutocallablePayoff::new(
            observation_dates,
            barriers,
            coupons,
            0.75,
            FinalPayoffType::CapitalProtection { floor: 0.9 },
            1.0,
            1.2,
            100_000.0,
            Currency::USD,
            100.0,
        );

        // Not autocalled, final spot is below initial
        let mut state = PathState::new(100, 1.0);
        state.set(state_keys::SPOT, 80.0); // Below initial

        payoff.on_event(&state);

        let value = payoff.value(Currency::USD);
        // Capital protection: max(0.9, 1.0 * 0.8) = 0.9
        assert!((value.amount() - 90_000.0).abs() < 1e-6);
    }

    #[test]
    fn test_autocallable_reset() {
        let observation_dates = vec![0.25];
        let barriers = vec![1.05];
        let coupons = vec![0.08];

        let mut payoff = AutocallablePayoff::new(
            observation_dates,
            barriers,
            coupons,
            0.75,
            FinalPayoffType::CapitalProtection { floor: 0.9 },
            1.0,
            1.2,
            100_000.0,
            Currency::USD,
            100.0,
        );

        let mut state = PathState::new(10, 0.25);
        state.set(state_keys::SPOT, 106.0);
        payoff.on_event(&state);
        assert!(payoff.autocalled_at.is_some());

        payoff.reset();
        assert!(payoff.autocalled_at.is_none());
        assert_eq!(payoff.min_spot_observed, f64::INFINITY);
    }
}

