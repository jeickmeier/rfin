//! Swaption payoffs for Monte Carlo pricing.
//!
//! Implements Bermudan swaption pricing using Longstaff-Schwartz Monte Carlo.
//! A swaption is an option to enter into an interest rate swap at future dates.

use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_monte_carlo::traits::PathState;
use finstack_monte_carlo::traits::Payoff;

/// Swaption type (payer or receiver).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwaptionType {
    /// Payer swaption: right to pay fixed rate
    Payer,
    /// Receiver swaption: right to receive fixed rate
    Receiver,
}

/// Swap schedule for Monte Carlo pricing.
///
/// Stores payment dates and accrual fractions for computing swap rates
/// and annuities from Hull-White short rate simulations.
#[derive(Debug, Clone)]
pub struct SwapSchedule {
    /// Payment dates (time in years from valuation date)
    pub payment_dates: Vec<f64>,
    /// Accrual fractions (daycount) for each period
    pub accrual_fractions: Vec<f64>,
    /// Start date of swap (time in years)
    pub start_date: f64,
    /// End date of swap (time in years)
    pub end_date: f64,
}

impl SwapSchedule {
    /// Create a new swap schedule.
    ///
    /// # Arguments
    ///
    /// * `start_date` - Swap start date (time in years)
    /// * `end_date` - Swap end date (time in years)
    /// * `payment_dates` - Payment dates (must be sorted, within [start_date, end_date])
    /// * `accrual_fractions` - Accrual fractions for each period
    pub fn new(
        start_date: f64,
        end_date: f64,
        payment_dates: Vec<f64>,
        accrual_fractions: Vec<f64>,
    ) -> Self {
        assert_eq!(
            payment_dates.len(),
            accrual_fractions.len(),
            "Payment dates and accrual fractions must have same length"
        );
        assert!(start_date < end_date, "Start date must be before end date");
        // Verify payment dates are sorted and within range
        for (i, &date) in payment_dates.iter().enumerate() {
            if i > 0 {
                assert!(payment_dates[i - 1] < date, "Payment dates must be sorted");
            }
            assert!(
                date >= start_date && date <= end_date,
                "Payment dates must be within [start_date, end_date]"
            );
        }

        Self {
            payment_dates,
            accrual_fractions,
            start_date,
            end_date,
        }
    }

    /// Compute annuity (PV01) at time t from discount factors.
    ///
    /// A(t) = Σ τ_i * DF(t, T_i) where τ_i are accrual fractions.
    #[cfg(test)]
    fn annuity(&self, discount_factors: &[f64]) -> f64 {
        assert_eq!(
            discount_factors.len(),
            self.payment_dates.len(),
            "Discount factors must match payment dates"
        );

        self.accrual_fractions
            .iter()
            .zip(discount_factors.iter())
            .map(|(tau, df)| tau * df)
            .sum()
    }
}

/// Bermudan swaption payoff.
///
/// A Bermudan swaption allows exercise at multiple dates before maturity.
/// At each exercise date, the holder can choose to enter into a swap with
/// fixed rate equal to the strike.
///
/// # Payoff
///
/// At exercise date t, if exercised:
/// - Payer: Pay fixed rate K, receive floating → value = (S(t) - K) * A(t) * N
/// - Receiver: Receive fixed rate K, pay floating → value = (K - S(t)) * A(t) * N
///
/// where S(t) is the forward swap rate and A(t) is the annuity.
#[derive(Debug, Clone)]
pub struct BermudanSwaptionPayoff {
    /// Exercise dates (time in years from valuation date)
    pub exercise_dates: Vec<f64>,
    /// Swap schedule
    pub swap_schedule: SwapSchedule,
    /// Strike rate (fixed rate of the swap)
    pub strike: f64,
    /// Swaption type (payer or receiver)
    pub option_type: SwaptionType,
    /// Notional amount
    pub notional: f64,
    // State variables (tracked during path simulation)
    /// Current swap value (computed at exercise dates)
    current_swap_value: f64,
    /// Index of last exercise date checked
    next_exercise_idx: usize,
    /// Whether option was exercised
    exercised: bool,
    /// Exercise date (if exercised)
    exercise_date: Option<f64>,
}

impl BermudanSwaptionPayoff {
    /// Create a new Bermudan swaption payoff.
    ///
    /// # Arguments
    ///
    /// * `exercise_dates` - Dates when exercise is allowed (must be sorted)
    /// * `swap_schedule` - Underlying swap schedule
    /// * `strike` - Fixed rate of the swap (e.g., 0.0325 for 3.25%)
    /// * `option_type` - Payer or receiver
    /// * `notional` - Notional amount
    pub fn new(
        exercise_dates: Vec<f64>,
        swap_schedule: SwapSchedule,
        strike: f64,
        option_type: SwaptionType,
        notional: f64,
    ) -> Self {
        // Verify exercise dates are sorted
        for i in 1..exercise_dates.len() {
            assert!(
                exercise_dates[i - 1] < exercise_dates[i],
                "Exercise dates must be sorted"
            );
        }

        Self {
            exercise_dates,
            swap_schedule,
            strike,
            option_type,
            notional,
            current_swap_value: 0.0,
            next_exercise_idx: 0,
            exercised: false,
            exercise_date: None,
        }
    }

    /// Check if we should exercise at current time.
    ///
    /// For payer: exercise if S(t) > K (swap value > 0)
    /// For receiver: exercise if K > S(t) (swap value > 0 when considering receiver)
    fn should_exercise(&self) -> bool {
        match self.option_type {
            SwaptionType::Payer => self.current_swap_value > 0.0,
            SwaptionType::Receiver => self.current_swap_value < 0.0, // Receiver wants negative (pay float, receive fixed)
        }
    }
}

impl Payoff for BermudanSwaptionPayoff {
    fn on_event(&mut self, state: &mut PathState) {
        // Check if we're at an exercise date
        if !self.exercised && self.next_exercise_idx < self.exercise_dates.len() {
            let target_date = self.exercise_dates[self.next_exercise_idx];

            // Check if current time matches exercise date (within tolerance)
            if (state.time - target_date).abs() < 1e-6 {
                // Swap value should be computed by pricer before calling on_event
                // If swap value indicates exercise, mark as exercised
                if self.should_exercise() {
                    self.exercised = true;
                    self.exercise_date = Some(target_date);
                }
                self.next_exercise_idx += 1;
            }
        }
    }

    fn value(&self, currency: Currency) -> Money {
        if self.exercised {
            // Value at exercise: (S(t) - K) * A(t) * N for payer
            // Note: This is simplified - full implementation requires annuity calculation
            let payoff = match self.option_type {
                SwaptionType::Payer => self.current_swap_value.max(0.0),
                SwaptionType::Receiver => (-self.current_swap_value).max(0.0),
            };
            Money::new(payoff * self.notional, currency)
        } else {
            // Not exercised - value is zero (continuation value handled by LSMC)
            Money::new(0.0, currency)
        }
    }

    fn reset(&mut self) {
        self.current_swap_value = 0.0;
        self.next_exercise_idx = 0;
        self.exercised = false;
        self.exercise_date = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_swap_schedule_creation() {
        let payment_dates = vec![1.0, 1.25, 1.5, 1.75, 2.0];
        let accruals = vec![0.25, 0.25, 0.25, 0.25, 0.25];
        let schedule = SwapSchedule::new(1.0, 2.0, payment_dates, accruals);

        assert_eq!(schedule.start_date, 1.0);
        assert_eq!(schedule.end_date, 2.0);
        assert_eq!(schedule.payment_dates.len(), 5);
    }

    #[test]
    fn test_swap_schedule_annuity() {
        let payment_dates = vec![1.0, 1.25, 1.5];
        let accruals = vec![0.25, 0.25, 0.25];
        let schedule = SwapSchedule::new(1.0, 1.5, payment_dates, accruals);

        let discount_factors = vec![0.95, 0.94, 0.93];
        let annuity = schedule.annuity(&discount_factors);

        // Annuity = 0.25 * 0.95 + 0.25 * 0.94 + 0.25 * 0.93 = 0.705
        assert!((annuity - 0.705).abs() < 1e-10);
    }

    #[test]
    fn test_bermudan_swaption_payoff_creation() {
        let exercise_dates = vec![1.0, 1.5, 2.0];
        let payment_dates = vec![1.0, 1.25, 1.5, 1.75, 2.0];
        let accruals = vec![0.25, 0.25, 0.25, 0.25, 0.25];
        let schedule = SwapSchedule::new(1.0, 2.0, payment_dates, accruals);

        let payoff = BermudanSwaptionPayoff::new(
            exercise_dates,
            schedule,
            0.0325,
            SwaptionType::Payer,
            10_000_000.0,
        );

        assert_eq!(payoff.strike, 0.0325);
        assert_eq!(payoff.exercise_dates.len(), 3);
        assert!(!payoff.exercised);
    }

    #[test]
    fn test_bermudan_swaption_reset() {
        let exercise_dates = vec![1.0];
        let payment_dates = vec![1.0, 1.25];
        let accruals = vec![0.25, 0.25];
        let schedule = SwapSchedule::new(1.0, 1.25, payment_dates, accruals);

        let mut payoff =
            BermudanSwaptionPayoff::new(exercise_dates, schedule, 0.0325, SwaptionType::Payer, 1.0);

        // Simulate some state
        payoff.current_swap_value = 0.01;
        payoff.exercised = true;
        payoff.exercise_date = Some(1.0);

        // Reset
        payoff.reset();

        assert_eq!(payoff.current_swap_value, 0.0);
        assert!(!payoff.exercised);
        assert_eq!(payoff.exercise_date, None);
        assert_eq!(payoff.next_exercise_idx, 0);
    }
}
