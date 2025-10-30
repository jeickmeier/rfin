//! Cliquet option payoffs for Monte Carlo pricing.
//!
//! Cliquet options have periodic resets where the strike is reset to the current
//! spot at reset dates, effectively creating a series of forward-starting options.

use super::super::traits::{PathState, Payoff};
use finstack_core::currency::Currency;
use finstack_core::money::Money;

/// Cliquet call option payoff.
///
/// A cliquet option accumulates returns over multiple periods with periodic resets.
/// At each reset date, the strike is reset to the current spot, and returns
/// are accumulated subject to local and global caps.
///
/// # Payoff Structure
///
/// Total payoff = Σ max(min(S_i/S_{i-1} - 1, local_cap), 0) capped at global_cap
///
/// where:
/// - S_i is spot at reset date i
/// - local_cap is maximum return per period
/// - global_cap is maximum total return
#[derive(Clone, Debug)]
pub struct CliquetCallPayoff {
    /// Reset dates (time in years, must be sorted)
    pub reset_dates: Vec<f64>,
    /// Local cap per period (e.g., 0.10 for 10% max per period)
    pub local_cap: f64,
    /// Global cap on total return (e.g., 0.30 for 30% max total)
    pub global_cap: f64,
    /// Notional amount
    pub notional: f64,
    /// Currency
    pub currency: Currency,
    /// Initial spot price S_0
    pub initial_spot: f64,

    // State variables (tracked during path simulation)
    /// Spot prices at reset dates
    reset_spots: Vec<f64>,
    /// Accumulated return so far
    accumulated_return: f64,
    /// Index of next reset date to check
    next_reset_idx: usize,
}

impl CliquetCallPayoff {
    /// Create a new cliquet call payoff.
    ///
    /// # Arguments
    ///
    /// * `reset_dates` - Dates when strike resets (must be sorted, includes initial date)
    /// * `local_cap` - Maximum return per period (e.g., 0.10 for 10%)
    /// * `global_cap` - Maximum total return (e.g., 0.30 for 30%)
    /// * `notional` - Notional amount
    /// * `currency` - Currency
    /// * `initial_spot` - Initial spot price S_0
    pub fn new(
        reset_dates: Vec<f64>,
        local_cap: f64,
        global_cap: f64,
        notional: f64,
        currency: Currency,
        initial_spot: f64,
    ) -> Self {
        // Verify reset dates are sorted
        for i in 1..reset_dates.len() {
            assert!(
                reset_dates[i - 1] < reset_dates[i],
                "Reset dates must be sorted"
            );
        }

        assert!(local_cap > 0.0, "Local cap must be positive");
        assert!(global_cap > 0.0, "Global cap must be positive");
        assert!(global_cap >= local_cap, "Global cap should be >= local cap");

        Self {
            reset_dates,
            local_cap,
            global_cap,
            notional,
            currency,
            initial_spot,
            reset_spots: Vec::new(),
            accumulated_return: 0.0,
            next_reset_idx: 0,
        }
    }

    /// Compute cliquet return from reset spots.
    ///
    /// Returns accumulated return subject to local and global caps.
    fn compute_return(&self) -> f64 {
        if self.reset_spots.is_empty() {
            return 0.0;
        }

        let mut total_return = 0.0;
        let mut prev_spot = self.initial_spot;

        for &spot in &self.reset_spots {
            // Period return: min(max(S_i/S_{i-1} - 1, 0), local_cap)
            let period_return = ((spot / prev_spot - 1.0).max(0.0)).min(self.local_cap);
            total_return += period_return;
            prev_spot = spot;
        }

        // Apply global cap
        total_return.min(self.global_cap)
    }
}

impl Payoff for CliquetCallPayoff {
    fn on_event(&mut self, state: &PathState) {
        if self.next_reset_idx < self.reset_dates.len() {
            let target_date = self.reset_dates[self.next_reset_idx];
            
            // Check if we're at a reset date
            if (state.time - target_date).abs() < 1e-6 || state.time >= target_date {
                if let Some(spot) = state.spot() {
                    self.reset_spots.push(spot);
                    self.next_reset_idx += 1;
                }
            }
        }
    }

    fn value(&self, currency: Currency) -> Money {
        // Compute total cliquet return
        let total_return = self.compute_return();
        
        // Payoff = total_return * notional
        Money::new(total_return * self.notional, currency)
    }

    fn reset(&mut self) {
        self.reset_spots.clear();
        self.accumulated_return = 0.0;
        self.next_reset_idx = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cliquet_creation() {
        let reset_dates = vec![0.0, 0.25, 0.5, 0.75, 1.0];
        let cliquet = CliquetCallPayoff::new(
            reset_dates,
            0.10,  // 10% local cap
            0.30,  // 30% global cap
            100_000.0,
            Currency::USD,
            100.0,
        );

        assert_eq!(cliquet.reset_dates.len(), 5);
        assert_eq!(cliquet.local_cap, 0.10);
        assert_eq!(cliquet.global_cap, 0.30);
    }

    #[test]
    fn test_cliquet_compute_return() {
        let reset_dates = vec![0.0, 0.25, 0.5];
        let mut cliquet = CliquetCallPayoff::new(
            reset_dates,
            0.10,  // 10% local cap
            0.30,  // 30% global cap
            1.0,
            Currency::USD,
            100.0,
        );

        // Simulate resets: 100 -> 110 -> 115
        cliquet.reset_spots = vec![110.0, 115.0];

        let return_val = cliquet.compute_return();
        // Period 1: min(max(110/100 - 1, 0), 0.10) = min(0.10, 0.10) = 0.10
        // Period 2: min(max(115/110 - 1, 0), 0.10) = min(0.045, 0.10) = 0.045
        // Total: 0.10 + 0.045 = 0.145
        assert!((return_val - 0.145).abs() < 1e-10);
    }

    #[test]
    fn test_cliquet_local_cap() {
        let reset_dates = vec![0.0, 0.25];
        let mut cliquet = CliquetCallPayoff::new(
            reset_dates,
            0.10,  // 10% local cap
            0.30,  // 30% global cap
            1.0,
            Currency::USD,
            100.0,
        );

        // Simulate large jump: 100 -> 150 (50% return, but capped at 10%)
        cliquet.reset_spots = vec![150.0];

        let return_val = cliquet.compute_return();
        // Period 1: min(max(150/100 - 1, 0), 0.10) = min(0.50, 0.10) = 0.10
        assert!((return_val - 0.10).abs() < 1e-10);
    }

    #[test]
    fn test_cliquet_global_cap() {
        let reset_dates = vec![0.0, 0.25, 0.5, 0.75];
        let mut cliquet = CliquetCallPayoff::new(
            reset_dates,
            0.10,  // 10% local cap
            0.30,  // 30% global cap
            1.0,
            Currency::USD,
            100.0,
        );

        // Simulate 4 periods each hitting local cap: 4 * 10% = 40%, but capped at 30%
        cliquet.reset_spots = vec![110.0, 121.0, 133.1, 146.41];

        let return_val = cliquet.compute_return();
        // Total uncapped: 0.10 + 0.10 + 0.10 + 0.10 = 0.40
        // But global cap: min(0.40, 0.30) = 0.30
        assert!((return_val - 0.30).abs() < 1e-10);
    }

    #[test]
    fn test_cliquet_reset() {
        let reset_dates = vec![0.0, 0.25];
        let mut cliquet = CliquetCallPayoff::new(
            reset_dates,
            0.10,
            0.30,
            1.0,
            Currency::USD,
            100.0,
        );

        cliquet.reset_spots = vec![110.0];
        cliquet.next_reset_idx = 1;

        cliquet.reset();

        assert!(cliquet.reset_spots.is_empty());
        assert_eq!(cliquet.next_reset_idx, 0);
        assert_eq!(cliquet.accumulated_return, 0.0);
    }
}

