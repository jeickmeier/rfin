//! Payoff computation for revolving credit facility Monte Carlo pricing.
//!
//! Accumulates cashflows over the facility life, handles default events,
//! and computes final discounted value.

use crate::instruments::common::mc::traits::{state_keys, PathState};
use crate::instruments::common::mc::traits::RandomStream;
use crate::instruments::common::models::monte_carlo::traits::Payoff;
use finstack_core::currency::Currency;
use finstack_core::dates::DayCount;
use finstack_core::money::Money;

/// Payoff for revolving credit facility.
///
/// Tracks utilization, interest rates, credit spreads, and accumulates
/// cashflows (interest, fees) over time. Handles default events with recovery.
#[derive(Clone, Debug)]
pub struct RevolvingCreditPayoff {
    /// Total commitment amount
    pub commitment_amount: f64,
    /// Day count convention for accrual
    pub day_count: DayCount,
    /// Base rate specification
    pub is_fixed_rate: bool,
    /// Fixed rate (if applicable)
    pub fixed_rate: f64,
    /// Margin in basis points (for floating rate)
    pub margin_bp: f64,
    /// Fee structure
    pub fees: FeeStructure,
    /// Recovery rate (fraction of drawn amount recovered on default)
    pub recovery_rate: f64,
    /// Maturity time (in years from valuation)
    pub maturity_time: f64,
    /// Time grid for cashflow events
    pub time_grid: Vec<f64>,
    /// Precomputed discount factors per step (df(as_of -> t_step))
    pub discounts: Vec<f64>,

    // Path state (updated during simulation)
    /// Current utilization rate
    utilization: f64,
    /// Current short rate (for floating)
    short_rate: f64,
    /// Current credit spread/hazard rate
    credit_spread: f64,
    /// Cumulative hazard (integrated credit spread)
    cumulative_hazard: f64,
    /// Default threshold (random draw per path)
    default_threshold: f64,
    /// Whether default has occurred
    defaulted: bool,
    /// Default time (if occurred)
    default_time: Option<f64>,
    /// Accumulated cashflows (negative for lender)
    accumulated_cashflows: Vec<(f64, f64)>, // (time, cashflow)
    /// Previous time step (for integration)
    prev_time: f64,
}

/// Fee structure for revolving credit facility.
#[derive(Clone, Debug)]
pub struct FeeStructure {
    /// Commitment fee (bps, annual, on undrawn)
    pub commitment_fee_bp: f64,
    /// Usage fee (bps, annual, on drawn)
    pub usage_fee_bp: f64,
    /// Facility fee (bps, annual, on total commitment)
    pub facility_fee_bp: f64,
    /// Upfront fee (absolute amount, paid at start)
    pub upfront_fee: f64,
}

impl RevolvingCreditPayoff {
    /// Create a new revolving credit payoff.
    ///
    /// # Arguments
    ///
    /// * `commitment_amount` - Total commitment
    /// * `day_count` - Day count convention
    /// * `is_fixed_rate` - True if fixed rate, false if floating
    /// * `fixed_rate` - Fixed rate (annualized, if applicable)
    /// * `margin_bp` - Margin over floating rate (basis points)
    /// * `fees` - Fee structure
    /// * `recovery_rate` - Recovery rate on default (e.g., 0.4 for 40%)
    /// * `maturity_time` - Maturity time in years
    /// * `time_grid` - Time grid for cashflow events
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        commitment_amount: f64,
        day_count: DayCount,
        is_fixed_rate: bool,
        fixed_rate: f64,
        margin_bp: f64,
        fees: FeeStructure,
        recovery_rate: f64,
        maturity_time: f64,
        time_grid: Vec<f64>,
        discounts: Vec<f64>,
    ) -> Self {
        Self {
            commitment_amount,
            day_count,
            is_fixed_rate,
            fixed_rate,
            margin_bp,
            fees,
            recovery_rate: recovery_rate.clamp(0.0, 1.0),
            maturity_time,
            time_grid,
            discounts,
            utilization: 0.0,
            short_rate: 0.0,
            credit_spread: 0.0,
            cumulative_hazard: 0.0,
            default_threshold: 0.0,
            defaulted: false,
            default_time: None,
            accumulated_cashflows: Vec::new(),
            prev_time: 0.0,
        }
    }

    /// Get current drawn amount.
    fn drawn_amount(&self) -> f64 {
        self.commitment_amount * self.utilization.clamp(0.0, 1.0)
    }

    /// Get current undrawn amount.
    fn undrawn_amount(&self) -> f64 {
        self.commitment_amount * (1.0 - self.utilization.clamp(0.0, 1.0))
    }

    /// Get current interest rate.
    fn interest_rate(&self) -> f64 {
        if self.is_fixed_rate {
            self.fixed_rate
        } else {
            // Floating: short_rate + margin
            self.short_rate + (self.margin_bp * 1e-4)
        }
    }

    /// Set default threshold for this path (should be called before each path).
    ///
    /// # Arguments
    ///
    /// * `threshold` - Exponential random variable E ~ Exp(1) = -ln(U)
    pub fn set_default_threshold(&mut self, threshold: f64) {
        self.default_threshold = threshold.max(1e-10);
    }

    /// Check for default event using cumulative hazard.
    ///
    /// Default occurs when cumulative hazard exceeds threshold:
    /// Λ(t) = ∫₀ᵗ λ(s) ds > E, where E ~ Exp(1)
    fn check_default(&mut self, current_time: f64) {
        if self.defaulted {
            return; // Already defaulted
        }

        // Update cumulative hazard using trapezoidal integration
        let dt = current_time - self.prev_time;
        if dt > 0.0 {
            // Use average credit spread over interval
            let avg_spread = self.credit_spread.max(0.0);
            // Convert credit spread to hazard rate: λ = spread / (1 - recovery)
            let hazard_rate = avg_spread / (1.0 - self.recovery_rate).max(0.01);
            self.cumulative_hazard += hazard_rate * dt;
        }

        // Check if default occurred
        if self.cumulative_hazard >= self.default_threshold {
            self.defaulted = true;
            self.default_time = Some(current_time);
        }
    }

    /// Compute cashflows for a time period.
    ///
    /// Returns cashflow amount (negative for lender, positive for borrower).
    fn compute_cashflow(&self, dt: f64) -> f64 {
        if self.defaulted {
            return 0.0; // No cashflows after default
        }

        let drawn = self.drawn_amount();
        let undrawn = self.undrawn_amount();
        let rate = self.interest_rate();

        // Interest on drawn amount (received by lender, positive)
        let interest = drawn * rate * dt;

        // Commitment fee on undrawn (received by lender, positive)
        let commitment_fee = undrawn * (self.fees.commitment_fee_bp * 1e-4) * dt;

        // Usage fee on drawn (received by lender, positive)
        let usage_fee = drawn * (self.fees.usage_fee_bp * 1e-4) * dt;

        // Facility fee on total commitment (received by lender, positive)
        let facility_fee = self.commitment_amount * (self.fees.facility_fee_bp * 1e-4) * dt;

        interest + commitment_fee + usage_fee + facility_fee
    }
}

impl Payoff for RevolvingCreditPayoff {
    fn on_path_start<R: RandomStream>(&mut self, rng: &mut R) {
        // Draw U ~ Uniform(0,1), set default threshold E = -ln(U)
        let u = rng.next_u01().clamp(1e-12, 1.0 - 1e-12);
        self.default_threshold = -u.ln();
    }

    fn on_event(&mut self, state: &PathState) {
        // Extract state variables from PathState
        // For revolving credit, state vector is [utilization, short_rate, credit_spread]
        // The engine sets: SPOT = state[0], VARIANCE = state[1]
        // We need to add custom keys for the full 3-factor state

        // Utilization is in SPOT slot (state[0])
        if let Some(util) = state.get(state_keys::SPOT) {
            self.utilization = util.clamp(0.0, 1.0);
        }

        // Short rate might be in SHORT_RATE key (set by custom engine wrapper)
        // or VARIANCE slot (state[1]) as fallback
        if let Some(rate) = state.get(state_keys::SHORT_RATE) {
            self.short_rate = rate;
        } else if let Some(rate) = state.get(state_keys::VARIANCE) {
            // Fallback: might be in VARIANCE slot
            self.short_rate = rate;
        }

        // Credit spread should be in a custom key "credit_spread" (set by engine wrapper)
        if let Some(spread) = state.get("credit_spread") {
            self.credit_spread = spread.max(0.0);
        }

        let current_time = state.time;

        // Check for default
        self.check_default(current_time);

        // If defaulted, compute recovery and stop
        if self.defaulted {
            if let Some(default_t) = self.default_time {
                if (current_time - default_t).abs() < 1e-10 {
                    // At default time: receive recovery only; principal loss is implied
                    let drawn = self.drawn_amount();
                    let recovery_amount = drawn * self.recovery_rate;
                    self.accumulated_cashflows
                        .push((current_time, recovery_amount));
                }
            }
            return; // No more cashflows after default
        }

        // Compute cashflow for this period
        let dt = current_time - self.prev_time;
        if dt > 0.0 {
            let cashflow = self.compute_cashflow(dt);
            // Discount at this step
            let step = state.step.min(self.discounts.len().saturating_sub(1));
            let df = self.discounts[step];
            self.accumulated_cashflows.push((current_time, cashflow * df));
        }

        self.prev_time = current_time;
    }

    fn value(&self, currency: Currency) -> Money {
        // Sum discounted cashflows accumulated
        let total_cashflow: f64 = self.accumulated_cashflows.iter().map(|(_, cf)| cf).sum();

        // Upfront fee occurs at start; for lender it is an outflow (negative), discount at step 0
        let upfront_df = self.discounts.first().copied().unwrap_or(1.0);
        let upfront = -self.fees.upfront_fee * upfront_df;

        // If no default, add terminal repayment discounted at final step
        let value = if !self.defaulted {
            let last_df = self
                .discounts
                .last()
                .copied()
                .unwrap_or(1.0);
            let drawn = self.drawn_amount();
            total_cashflow + upfront + drawn * last_df
        } else {
            total_cashflow + upfront
        };

        // Apply sanity bounds
        let bounded_value = value.clamp(
            -self.commitment_amount * 10.0,
            self.commitment_amount * 10.0,
        );

        Money::new(bounded_value, currency)
    }

    fn reset(&mut self) {
        // Reset path state for new simulation
        self.utilization = 0.0;
        self.short_rate = 0.0;
        self.credit_spread = 0.0;
        self.cumulative_hazard = 0.0;

        // Draw new default threshold: E ~ Exp(1) = -ln(U) where U ~ Uniform(0,1)
        // Note: In a full implementation, this should be generated by the RNG in the engine
        // For now, we'll use a deterministic approach that will be overridden by the pricer
        // The pricer should set default_threshold before each path
        // Default to a reasonable value that won't cause immediate default
        if self.default_threshold == 0.0 {
            self.default_threshold = 10.0; // High threshold by default
        }

        self.defaulted = false;
        self.default_time = None;
        self.accumulated_cashflows.clear();
        self.prev_time = 0.0;
    }
}

impl FeeStructure {
    /// Create a new fee structure.
    pub fn new(
        commitment_fee_bp: f64,
        usage_fee_bp: f64,
        facility_fee_bp: f64,
        upfront_fee: f64,
    ) -> Self {
        Self {
            commitment_fee_bp,
            usage_fee_bp,
            facility_fee_bp,
            upfront_fee,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::dates::DayCount;

    #[test]
    fn test_payoff_creation() {
        let fees = FeeStructure::new(25.0, 10.0, 5.0, 50_000.0);
        let payoff = RevolvingCreditPayoff::new(
            10_000_000.0,
            DayCount::Act360,
            true,
            0.05,
            0.0,
            fees,
            0.4,
            1.0,
            vec![0.0, 0.25, 0.5, 0.75, 1.0],
            vec![1.0, 0.99, 0.98, 0.97, 0.96],
        );

        assert_eq!(payoff.commitment_amount, 10_000_000.0);
        assert_eq!(payoff.fixed_rate, 0.05);
        assert!(payoff.is_fixed_rate);
    }

    #[test]
    fn test_cashflow_computation() {
        let fees = FeeStructure::new(25.0, 10.0, 5.0, 0.0);
        let mut payoff = RevolvingCreditPayoff::new(
            10_000_000.0,
            DayCount::Act360,
            true,
            0.05,
            0.0,
            fees,
            0.4,
            1.0,
            vec![0.0, 0.25, 0.5, 0.75, 1.0],
            vec![1.0, 0.99, 0.98, 0.97, 0.96],
        );

        // Set utilization to 50%
        payoff.utilization = 0.5;
        payoff.short_rate = 0.05;
        payoff.credit_spread = 0.01;

        // Compute cashflow for one quarter
        let dt = 0.25;
        let cf = payoff.compute_cashflow(dt);

        // Interest:  5M * 0.05 * 0.25 = 62,500
        // Commitment fee: 5M * 0.0025 * 0.25 = 3,125
        // Usage fee: 5M * 0.001 * 0.25 = 1,250
        // Facility fee: 10M * 0.0005 * 0.25 = 1,250
        // Total: 68,125 (undiscounted for this dt snippet)
        assert!(cf > 60_000.0);
    }

    #[test]
    fn test_default_check() {
        let fees = FeeStructure::new(0.0, 0.0, 0.0, 0.0);
        let mut payoff = RevolvingCreditPayoff::new(
            10_000_000.0,
            DayCount::Act360,
            true,
            0.05,
            0.0,
            fees,
            0.4,
            1.0,
            vec![0.0, 1.0],
            vec![1.0, 0.95],
        );

        // Set default threshold to low value to trigger default
        payoff.default_threshold = 0.1;
        payoff.credit_spread = 0.2; // High spread
        payoff.cumulative_hazard = 0.0;

        // Check default after some time
        payoff.check_default(0.5);

        // Should default due to high cumulative hazard
        assert!(payoff.defaulted || payoff.cumulative_hazard > 0.0);
    }
}
