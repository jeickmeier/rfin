//! Payoff computation for revolving credit facility Monte Carlo pricing.
//!
//! Generates cashflows over the facility life, handles default events,
//! and tracks principal balances. Uses market-standard conventions for
//! cashflow signs and principal tracking.
//!
//! # Architecture
//!
//! This payoff follows a clean separation of concerns:
//! - **Cashflow generation**: Emits undiscounted typed cashflows via `PathState`
//! - **Default detection**: Delegated to `FirstPassageCalculator`
//! - **Discounting**: Handled by the MC engine (not in payoff)
//!
//! # Sign Conventions (Lender Perspective)
//!
//! All cashflows follow lender perspective:
//! - **Principal deployment (draw)**: Negative (outflow to borrower)
//! - **Principal repayment**: Positive (inflow from borrower)
//! - **Interest/fees received**: Positive (inflow)
//! - **Upfront fee paid**: Negative (handled at pricer level)
//! - **Recovery**: Positive (partial recovery of defaulted principal)
//!
//! # Cashflow Types
//!
//! Uses `CashflowType` enum for typed cashflow tracking:
//! - `Principal`: Draws, repayments, and terminal repayment
//! - `Interest`: Interest on drawn amounts
//! - `CommitmentFee`: Fee on undrawn amounts
//! - `UsageFee`: Fee on drawn amounts  
//! - `FacilityFee`: Fee on total commitment
//! - `Recovery`: Recovery proceeds on default
//!
//! # References
//!
//! - Basel Committee on Banking Supervision (2017). *Basel III: Finalising
//!   post-crisis reforms*. Bank for International Settlements.
//! - Altman, E. I., & Saunders, A. (1998). "Credit risk measurement:
//!   Developments over the last 20 years." *Journal of Banking & Finance*, 21(11-12).

use crate::instruments::common::mc::paths::CashflowType;
use crate::instruments::common::mc::traits::{state_keys, PathState, RandomStream};
use crate::instruments::common::models::monte_carlo::payoff::default_calculator::{
    DefaultEvent, FirstPassageCalculator,
};
use crate::instruments::common::models::monte_carlo::traits::Payoff;
use finstack_core::currency::Currency;
use finstack_core::dates::DayCount;
use finstack_core::money::Money;
use finstack_core::config::{RoundingContext, ZeroKind};

/// Rate projection mode for floating-rate facilities.
#[derive(Clone, Debug)]
pub enum RateProjection {
    /// Integrate short rate + margin at each step (OIS-style compounding).
    ShortRateIntegral,
    /// Use term-locked rates per step (locked at reset for the period).
    /// Each entry is the all-in rate (base + margin, with floor applied) for that step.
    TermLocked {
        /// Locked all-in rates by step index.
        rates_by_step: Vec<f64>,
    },
}

/// Rate specification for revolving credit facility.
#[derive(Clone, Debug)]
pub enum RateSpec {
    /// Fixed rate (annualized)
    Fixed {
        /// Annual rate (e.g., 0.05 for 5%)
        rate: f64,
    },
    /// Floating rate with margin
    Floating {
        /// Margin in basis points over short rate
        margin_bp: f64,
        /// Rate projection mode
        projection: RateProjection,
    },
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
}

impl FeeStructure {
    /// Create a new fee structure.
    pub fn new(commitment_fee_bp: f64, usage_fee_bp: f64, facility_fee_bp: f64) -> Self {
        Self {
            commitment_fee_bp,
            usage_fee_bp,
            facility_fee_bp,
        }
    }
}

/// Payoff for revolving credit facility.
///
/// Tracks utilization, interest rates, credit spreads, and generates
/// cashflows (interest, fees, principal) over time. Handles default events
/// with recovery using first-passage time methodology.
///
/// # Design Principles
///
/// - **Dual tracking**: Accumulates discounted PV for engine + undiscounted cashflows for Python
/// - **Explicit principal tracking**: Maintains outstanding balance, not cumulative flows
/// - **Modular default**: Uses `FirstPassageCalculator` for credit risk
///
/// # State Management
///
/// Per-path state:
/// - Current utilization rate
/// - Outstanding principal (absolute amount)
/// - Accumulated discounted PV (for engine)
/// - Previous timestamp (for integration)
/// - Default calculator (encapsulates default state)
#[derive(Clone, Debug)]
pub struct RevolvingCreditPayoff {
    // Static configuration (set once, never mutated during path)
    /// Total commitment amount
    pub commitment_amount: f64,
    /// Day count convention for accrual
    pub day_count: DayCount,
    /// Rate specification (fixed or floating)
    pub rate_spec: RateSpec,
    /// Fee structure
    pub fees: FeeStructure,
    /// Maturity time (in years from valuation)
    pub maturity_time: f64,
    /// Precomputed discount factors at each step
    pub discount_factors: Vec<f64>,

    // Default detection (stateful but encapsulated)
    /// First-passage time default calculator
    default_calculator: FirstPassageCalculator,

    // Per-path state (reset on each path)
    /// Current utilization rate (0.0 to 1.0)
    current_utilization: f64,
    /// Outstanding principal (absolute amount drawn)
    outstanding_principal: f64,
    /// Accumulated discounted PV (for engine's value() call)
    accumulated_pv: f64,
    /// Previous time step (for integration)
    prev_time: f64,
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
    /// * `discount_factors` - Precomputed discount factors at each step
    /// * `rate_projection` - Rate projection mode for floating rates
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
        discount_factors: Vec<f64>,
        rate_projection: RateProjection,
    ) -> Self {
        let rate_spec = if is_fixed_rate {
            RateSpec::Fixed { rate: fixed_rate }
        } else {
            RateSpec::Floating { margin_bp, projection: rate_projection }
        };

        Self {
            commitment_amount,
            day_count,
            rate_spec,
            fees,
            maturity_time,
            discount_factors,
            default_calculator: FirstPassageCalculator::new(recovery_rate),
            current_utilization: 0.0,
            outstanding_principal: 0.0,
            accumulated_pv: 0.0,
            prev_time: 0.0,
        }
    }

    /// Compute the current interest rate from short rate and rate spec.
    fn compute_rate(&self, short_rate: f64, step: usize) -> f64 {
        match &self.rate_spec {
            RateSpec::Fixed { rate } => *rate,
            RateSpec::Floating { margin_bp, projection } => {
                match projection {
                    RateProjection::ShortRateIntegral => {
                        // Floating: short_rate + margin (OIS-style compounding)
                        short_rate + (margin_bp * 1e-4)
                    }
                    RateProjection::TermLocked { rates_by_step } => {
                        // Use pre-locked all-in rate for this step
                        rates_by_step.get(step).copied().unwrap_or(0.0)
                    }
                }
            }
        }
    }

    /// Check if the given time is at or near maturity.
    fn is_maturity(&self, time: f64) -> bool {
        (time - self.maturity_time).abs() < 1e-2
    }
}

impl Payoff for RevolvingCreditPayoff {
    fn on_path_start<R: RandomStream>(&mut self, rng: &mut R) {
        // Draw U ~ Uniform(0,1), set default threshold E = -ln(U)
        let u = rng.next_u01().clamp(1e-12, 1.0 - 1e-12);
        let threshold = -u.ln();
        self.default_calculator.set_threshold(threshold);
    }

    fn on_event(&mut self, state: &mut PathState) {
        let rc = RoundingContext::default();
        let current_time = state.time;
        let dt = current_time - self.prev_time;

        // 1. Extract state variables
        // Utilization is in SPOT slot (state[0])
        let new_utilization = state.get(state_keys::SPOT).unwrap_or(0.0).clamp(0.0, 1.0);

        // Short rate from SHORT_RATE key or VARIANCE slot as fallback
        let short_rate = state
            .get(state_keys::SHORT_RATE)
            .or_else(|| state.get(state_keys::VARIANCE))
            .unwrap_or(0.0);

        // Credit spread from custom key "credit_spread"
        let credit_spread = state.get("credit_spread").unwrap_or(0.0).max(0.0);

        // 2. Check for default
        let default_event = self
            .default_calculator
            .update(credit_spread, dt, current_time);

        if let DefaultEvent::DefaultOccurred {
            time,
            recovery_fraction,
        } = default_event
        {
            // Emit recovery cashflow and stop all further processing
            let recovery = self.outstanding_principal * recovery_fraction;
            if !rc.is_effectively_zero(recovery, ZeroKind::Generic) {
                state.add_typed_cashflow(time, recovery, CashflowType::Recovery);

                // Add discounted recovery to accumulated PV
                let step = state
                    .step
                    .min(self.discount_factors.len().saturating_sub(1));
                let df = self.discount_factors[step];
                self.accumulated_pv += recovery * df;
            }
            // Note: Principal loss is implicit (no negative principal flow)
            // The lender deployed outstanding_principal but only receives recovery
            return;
        }

        // 3. Compute principal change from utilization delta
        let new_balance = self.commitment_amount * new_utilization;
        let principal_change = new_balance - self.outstanding_principal;

        if principal_change.abs() > 1e-6 {
            // Sign convention (lender perspective):
            // - Draw (principal_change > 0): negative cashflow (deployment to borrower)
            // - Repay (principal_change < 0): positive cashflow (receipt from borrower)
            state.add_typed_cashflow(current_time, -principal_change, CashflowType::Principal);
            self.outstanding_principal = new_balance;
        }

        // 4. Generate operational cashflows (interest + fees)
        if dt > 0.0 {
            let drawn = self.outstanding_principal;
            let undrawn = self.commitment_amount - drawn;
            let rate = self.compute_rate(short_rate, state.step);

            // Get discount factor for this step
            let step = state
                .step
                .min(self.discount_factors.len().saturating_sub(1));
            let df = self.discount_factors[step];

            // Interest on drawn amount
            let interest = drawn * rate * dt;
            if !rc.is_effectively_zero(interest, ZeroKind::Generic) {
                state.add_typed_cashflow(current_time, interest, CashflowType::Interest);
                self.accumulated_pv += interest * df;
            }

            // Commitment fee on undrawn
            let commitment_fee = undrawn * (self.fees.commitment_fee_bp * 1e-4) * dt;
            if !rc.is_effectively_zero(commitment_fee, ZeroKind::Generic) {
                state.add_typed_cashflow(current_time, commitment_fee, CashflowType::CommitmentFee);
                self.accumulated_pv += commitment_fee * df;
            }

            // Usage fee on drawn
            let usage_fee = drawn * (self.fees.usage_fee_bp * 1e-4) * dt;
            if !rc.is_effectively_zero(usage_fee, ZeroKind::Generic) {
                state.add_typed_cashflow(current_time, usage_fee, CashflowType::UsageFee);
                self.accumulated_pv += usage_fee * df;
            }

            // Facility fee on total commitment
            let facility_fee = self.commitment_amount * (self.fees.facility_fee_bp * 1e-4) * dt;
            if !rc.is_effectively_zero(facility_fee, ZeroKind::Generic) {
                state.add_typed_cashflow(current_time, facility_fee, CashflowType::FacilityFee);
                self.accumulated_pv += facility_fee * df;
            }
        }

        // 5. At maturity: repay outstanding principal (if any)
        if self.is_maturity(current_time) && self.outstanding_principal > 1e-6 {
            state.add_typed_cashflow(
                current_time,
                self.outstanding_principal,
                CashflowType::Principal,
            );

            // Add discounted terminal repayment to PV
            let step = state
                .step
                .min(self.discount_factors.len().saturating_sub(1));
            let df = self.discount_factors[step];
            self.accumulated_pv += self.outstanding_principal * df;

            self.outstanding_principal = 0.0;
        }

        // Update state for next event
        self.prev_time = current_time;
        self.current_utilization = new_utilization;
    }

    fn value(&self, currency: Currency) -> Money {
        // Return accumulated discounted PV for the engine
        Money::new(self.accumulated_pv, currency)
    }

    fn reset(&mut self) {
        // Reset per-path state
        self.current_utilization = 0.0;
        self.outstanding_principal = 0.0;
        self.accumulated_pv = 0.0;
        self.prev_time = 0.0;
        self.default_calculator.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::dates::DayCount;

    #[test]
    fn test_payoff_creation() {
        let fees = FeeStructure::new(25.0, 10.0, 5.0);
        let discounts = vec![1.0, 0.99, 0.98, 0.97, 0.96];
        let payoff = RevolvingCreditPayoff::new(
            10_000_000.0,
            DayCount::Act360,
            true,
            0.05,
            0.0,
            fees,
            0.4,
            1.0,
            discounts,
            RateProjection::ShortRateIntegral, // For fixed rate, projection is ignored
        );

        assert_eq!(payoff.commitment_amount, 10_000_000.0);
        assert_eq!(payoff.maturity_time, 1.0);
        assert!(matches!(payoff.rate_spec, RateSpec::Fixed { rate } if rate == 0.05));
    }

    #[test]
    fn test_fixed_rate_computation() {
        let fees = FeeStructure::new(0.0, 0.0, 0.0);
        let discounts = vec![1.0];
        let payoff = RevolvingCreditPayoff::new(
            10_000_000.0,
            DayCount::Act360,
            true,
            0.05,
            0.0,
            fees,
            0.4,
            1.0,
            discounts,
            RateProjection::ShortRateIntegral,
        );

        assert_eq!(payoff.compute_rate(0.03, 0), 0.05); // Short rate ignored for fixed
    }

    #[test]
    fn test_floating_rate_computation() {
        let fees = FeeStructure::new(0.0, 0.0, 0.0);
        let discounts = vec![1.0];
        let payoff = RevolvingCreditPayoff::new(
            10_000_000.0,
            DayCount::Act360,
            false,
            0.0,
            50.0, // 50bp margin
            fees,
            0.4,
            1.0,
            discounts,
            RateProjection::ShortRateIntegral,
        );

        // Floating: short_rate + margin (ShortRateIntegral mode)
        // 0.03 + 50bp = 0.03 + 0.005 = 0.035
        assert!((payoff.compute_rate(0.03, 0) - 0.035).abs() < 1e-10);
    }

    #[test]
    fn test_maturity_check() {
        let fees = FeeStructure::new(0.0, 0.0, 0.0);
        let discounts = vec![1.0];
        let payoff = RevolvingCreditPayoff::new(
            10_000_000.0,
            DayCount::Act360,
            true,
            0.05,
            0.0,
            fees,
            0.4,
            1.0,
            discounts,
            RateProjection::ShortRateIntegral,
        );

        assert!(payoff.is_maturity(1.0));
        assert!(payoff.is_maturity(1.005)); // Within tolerance
        assert!(!payoff.is_maturity(0.5));
        assert!(!payoff.is_maturity(1.02)); // Outside tolerance
    }

    #[test]
    fn test_reset() {
        let fees = FeeStructure::new(0.0, 0.0, 0.0);
        let discounts = vec![1.0];
        let mut payoff = RevolvingCreditPayoff::new(
            10_000_000.0,
            DayCount::Act360,
            true,
            0.05,
            0.0,
            fees,
            0.4,
            1.0,
            discounts,
            RateProjection::ShortRateIntegral,
        );

        // Set some state
        payoff.current_utilization = 0.5;
        payoff.outstanding_principal = 5_000_000.0;
        payoff.accumulated_pv = 100_000.0;
        payoff.prev_time = 0.5;

        // Reset
        payoff.reset();

        assert_eq!(payoff.current_utilization, 0.0);
        assert_eq!(payoff.outstanding_principal, 0.0);
        assert_eq!(payoff.accumulated_pv, 0.0);
        assert_eq!(payoff.prev_time, 0.0);
    }

    #[test]
    fn test_value_returns_accumulated_pv() {
        let fees = FeeStructure::new(0.0, 0.0, 0.0);
        let discounts = vec![1.0];
        let mut payoff = RevolvingCreditPayoff::new(
            10_000_000.0,
            DayCount::Act360,
            true,
            0.05,
            0.0,
            fees,
            0.4,
            1.0,
            discounts,
            RateProjection::ShortRateIntegral,
        );

        // Initially zero
        assert_eq!(payoff.value(Currency::USD).amount(), 0.0);

        // After accumulating PV
        payoff.accumulated_pv = 123_456.78;
        assert_eq!(payoff.value(Currency::USD).amount(), 123_456.78);
    }
}
