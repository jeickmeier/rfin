//! Interest rate derivative payoffs for Monte Carlo pricing.
//!
//! Provides payoffs for caps, floors, and swaptions under short rate models
//! like Hull-White 1F.
//!
//! # Cap/Floor Basics
//!
//! - **Cap**: Portfolio of caplets, each pays max(L - K, 0) on fixing dates
//! - **Floor**: Portfolio of floorlets, each pays max(K - L, 0) on fixing dates
//! - **L**: Forward rate (LIBOR/SOFR) for period [T_i, T_i+1]
//! - **K**: Strike rate
//!
//! # Implementation Note
//!
//! For Hull-White, the short rate r(t) is simulated, and forward rates
//! are derived using the model's bond price formulas.

use crate::instruments::common::mc::traits::PathState;
use crate::instruments::common::models::monte_carlo::traits::Payoff;
use finstack_core::currency::Currency;
use finstack_core::money::Money;

/// Direction of the interest rate payoff (cap vs floor).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RatesPayoffType {
    /// Cap payoff: max(L - K, 0)
    Cap,
    /// Floor payoff: max(K - L, 0)
    Floor,
}

/// Unified interest rate derivative payoff (cap or floor).
///
/// A cap pays max(L - K, 0) at each fixing date.
/// A floor pays max(K - L, 0) at each fixing date.
///
/// Where L is the forward rate for the period and K is the strike rate.
///
/// # State Requirements
///
/// Expects `PathState` to contain "short_rate" at fixing dates.
#[derive(Clone, Debug)]
pub struct RatesPayoff {
    /// Type of payoff (cap or floor)
    pub payoff_type: RatesPayoffType,
    /// Strike rate (e.g., 0.03 for 3%)
    pub strike_rate: f64,
    /// Notional amount
    pub notional: f64,
    /// Fixing dates (time in years)
    pub fixing_dates: Vec<f64>,
    /// Accrual fractions (daycount) for each period
    pub accrual_fractions: Vec<f64>,
    /// Currency
    pub currency: Currency,
    /// Discount factors for each payment (pre-computed or from curve)
    pub discount_factors: Vec<f64>,

    // State
    accumulated_pv: f64,
    next_fixing_idx: usize,
}

impl RatesPayoff {
    /// Create a new rates payoff (cap or floor).
    ///
    /// # Arguments
    ///
    /// * `payoff_type` - Type of payoff (Cap or Floor)
    /// * `strike_rate` - Strike rate (as decimal, e.g., 0.03 for 3%)
    /// * `notional` - Notional amount
    /// * `fixing_dates` - Time points for rate fixings
    /// * `accrual_fractions` - Daycount fractions for each period
    /// * `discount_factors` - Discount factors for each payment
    /// * `currency` - Currency for the payoff
    pub fn new(
        payoff_type: RatesPayoffType,
        strike_rate: f64,
        notional: f64,
        fixing_dates: Vec<f64>,
        accrual_fractions: Vec<f64>,
        discount_factors: Vec<f64>,
        currency: Currency,
    ) -> Self {
        assert_eq!(
            fixing_dates.len(),
            accrual_fractions.len(),
            "Fixing dates and accrual fractions must match"
        );
        assert_eq!(
            fixing_dates.len(),
            discount_factors.len(),
            "Fixing dates and discount factors must match"
        );

        Self {
            payoff_type,
            strike_rate,
            notional,
            fixing_dates,
            accrual_fractions,
            discount_factors,
            currency,
            accumulated_pv: 0.0,
            next_fixing_idx: 0,
        }
    }
}

impl Payoff for RatesPayoff {
    fn on_event(&mut self, state: &mut PathState) {
        // Check if we're at a fixing date
        if self.next_fixing_idx < self.fixing_dates.len() {
            let target_time = self.fixing_dates[self.next_fixing_idx];

            // Use small tolerance for time matching
            if (state.time - target_time).abs() < 1e-6 {
                // Get short rate from path state
                // Defaults to 0.0 if short_rate is not available, which results in
                // zero forward rate and zero payoff for that fixing period.
                let short_rate = state.get("short_rate").unwrap_or(0.0);

                // TODO: Replace with proper Hull-White forward rate calculation:
                // L(t,T,T+τ) = [P(t,T) - P(t,T+τ)] / [τ P(t,T+τ)]
                // where P(t,T) is the Hull-White bond price formula.
                // Currently using short_rate as a simplified approximation.
                let forward_rate = short_rate;

                // Compute payoff based on type
                let intrinsic_value = match self.payoff_type {
                    RatesPayoffType::Cap => (forward_rate - self.strike_rate).max(0.0),
                    RatesPayoffType::Floor => (self.strike_rate - forward_rate).max(0.0),
                };

                let period_payoff = intrinsic_value
                    * self.accrual_fractions[self.next_fixing_idx]
                    * self.notional
                    * self.discount_factors[self.next_fixing_idx];

                self.accumulated_pv += period_payoff;
                self.next_fixing_idx += 1;
            }
        }
    }

    fn value(&self, _currency: Currency) -> Money {
        Money::new(self.accumulated_pv, self.currency)
    }

    fn reset(&mut self) {
        self.accumulated_pv = 0.0;
        self.next_fixing_idx = 0;
    }
}

// ============================================================================
// Backward Compatibility Type Aliases (Deprecated)
// ============================================================================

/// Legacy cap payoff type.
///
/// **Deprecated**: Use `RatesPayoff` with `RatesPayoffType::Cap` instead.
///
/// # Migration
///
/// ```ignore
/// // Old:
/// let cap = CapPayoff::new(strike, notional, dates, accruals, dfs, ccy);
///
/// // New:
/// let cap = RatesPayoff::new(RatesPayoffType::Cap, strike, notional, dates, accruals, dfs, ccy);
/// ```
#[deprecated(
    since = "0.1.0",
    note = "Use RatesPayoff with RatesPayoffType::Cap instead"
)]
pub type CapPayoff = RatesPayoff;

/// Legacy floor payoff type.
///
/// **Deprecated**: Use `RatesPayoff` with `RatesPayoffType::Floor` instead.
///
/// # Migration
///
/// ```ignore
/// // Old:
/// let floor = FloorPayoff::new(strike, notional, dates, accruals, dfs, ccy);
///
/// // New:
/// let floor = RatesPayoff::new(RatesPayoffType::Floor, strike, notional, dates, accruals, dfs, ccy);
/// ```
#[deprecated(
    since = "0.1.0",
    note = "Use RatesPayoff with RatesPayoffType::Floor instead"
)]
pub type FloorPayoff = RatesPayoff;

/// Cap-floor parity relationship.
///
/// Validates: Cap - Floor = Swap (fixed for floating)
///
/// For a cap with strike K and floor with same strike:
/// ```text
/// Cap(K) - Floor(K) = Σ DF_i * τ_i * N * (L_i - K)
/// ```
///
/// This is the value of receiving floating and paying fixed at K.
pub fn cap_floor_parity_swap_value(
    fixing_dates: &[f64],
    forward_rates: &[f64],
    accrual_fractions: &[f64],
    discount_factors: &[f64],
    strike_rate: f64,
    notional: f64,
) -> f64 {
    use finstack_core::math::summation::NeumaierAccumulator;

    assert_eq!(fixing_dates.len(), forward_rates.len());
    assert_eq!(fixing_dates.len(), accrual_fractions.len());
    assert_eq!(fixing_dates.len(), discount_factors.len());

    let mut pv = NeumaierAccumulator::new();
    for i in 0..fixing_dates.len() {
        let cashflow = (forward_rates[i] - strike_rate) * accrual_fractions[i] * notional;
        pv.add(cashflow * discount_factors[i]);
    }

    pv.total()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cap_payoff_structure() {
        let fixing_dates = vec![0.25, 0.5, 0.75, 1.0];
        let accruals = vec![0.25, 0.25, 0.25, 0.25];
        let dfs = vec![0.99, 0.98, 0.97, 0.96];

        let cap = RatesPayoff::new(
            RatesPayoffType::Cap,
            0.03,
            1_000_000.0,
            fixing_dates,
            accruals,
            dfs,
            Currency::USD,
        );

        assert_eq!(cap.payoff_type, RatesPayoffType::Cap);
        assert_eq!(cap.strike_rate, 0.03);
        assert_eq!(cap.notional, 1_000_000.0);
        assert_eq!(cap.fixing_dates.len(), 4);
    }

    #[test]
    fn test_floor_payoff_structure() {
        let fixing_dates = vec![0.25, 0.5];
        let accruals = vec![0.25, 0.25];
        let dfs = vec![0.99, 0.98];

        let floor = RatesPayoff::new(
            RatesPayoffType::Floor,
            0.02,
            500_000.0,
            fixing_dates,
            accruals,
            dfs,
            Currency::EUR,
        );

        assert_eq!(floor.payoff_type, RatesPayoffType::Floor);
        assert_eq!(floor.strike_rate, 0.02);
        assert_eq!(floor.notional, 500_000.0);
    }

    #[test]
    fn test_cap_floor_parity() {
        let fixing_dates = vec![0.5, 1.0];
        let forward_rates = vec![0.04, 0.045];
        let accruals = vec![0.5, 0.5];
        let dfs = vec![0.98, 0.96];
        let strike = 0.03;
        let notional = 1_000_000.0;

        let swap_value = cap_floor_parity_swap_value(
            &fixing_dates,
            &forward_rates,
            &accruals,
            &dfs,
            strike,
            notional,
        );

        // Manual calculation:
        // Period 1: (0.04 - 0.03) * 0.5 * 1M * 0.98 = 4,900
        // Period 2: (0.045 - 0.03) * 0.5 * 1M * 0.96 = 7,200
        // Total: 12,100

        let expected =
            (0.04 - 0.03) * 0.5 * 1_000_000.0 * 0.98 + (0.045 - 0.03) * 0.5 * 1_000_000.0 * 0.96;

        assert!((swap_value - expected).abs() < 1.0);
    }

    #[test]
    fn test_rates_payoff_type_enum() {
        // Verify enum values are distinct
        assert_ne!(RatesPayoffType::Cap, RatesPayoffType::Floor);

        // Verify Copy trait
        let cap_type = RatesPayoffType::Cap;
        let _cap_type_copy = cap_type;
        assert_eq!(cap_type, RatesPayoffType::Cap);
    }

    #[test]
    fn test_payoff_computation_cap() {
        let fixing_dates = vec![0.5];
        let accruals = vec![0.5];
        let dfs = vec![0.98];

        let mut cap = RatesPayoff::new(
            RatesPayoffType::Cap,
            0.03,
            1_000_000.0,
            fixing_dates.clone(),
            accruals.clone(),
            dfs.clone(),
            Currency::USD,
        );

        // Simulate path state with forward rate above strike
        let mut state = PathState::new(0, 0.5);
        state.set("short_rate", 0.05); // Forward rate = 5% > 3% strike

        cap.on_event(&mut state);

        // Expected: (0.05 - 0.03) * 0.5 * 1M * 0.98 = 9,800
        let expected_pv = (0.05 - 0.03) * 0.5 * 1_000_000.0 * 0.98;
        assert!((cap.accumulated_pv - expected_pv).abs() < 1.0);
    }

    #[test]
    fn test_payoff_computation_floor() {
        let fixing_dates = vec![0.5];
        let accruals = vec![0.5];
        let dfs = vec![0.98];

        let mut floor = RatesPayoff::new(
            RatesPayoffType::Floor,
            0.03,
            1_000_000.0,
            fixing_dates.clone(),
            accruals.clone(),
            dfs.clone(),
            Currency::USD,
        );

        // Simulate path state with forward rate below strike
        let mut state = PathState::new(0, 0.5);
        state.set("short_rate", 0.01); // Forward rate = 1% < 3% strike

        floor.on_event(&mut state);

        // Expected: (0.03 - 0.01) * 0.5 * 1M * 0.98 = 9,800
        let expected_pv = (0.03 - 0.01) * 0.5 * 1_000_000.0 * 0.98;
        assert!((floor.accumulated_pv - expected_pv).abs() < 1.0);
    }

    #[test]
    fn test_backward_compatibility_aliases() {
        // Test that deprecated type aliases still work
        #[allow(deprecated)]
        {
            let fixing_dates = vec![0.5];
            let accruals = vec![0.5];
            let dfs = vec![0.98];

            let _cap: CapPayoff = RatesPayoff::new(
                RatesPayoffType::Cap,
                0.03,
                1_000_000.0,
                fixing_dates.clone(),
                accruals.clone(),
                dfs.clone(),
                Currency::USD,
            );

            let _floor: FloorPayoff = RatesPayoff::new(
                RatesPayoffType::Floor,
                0.03,
                1_000_000.0,
                fixing_dates,
                accruals,
                dfs,
                Currency::USD,
            );
        }
    }
}
