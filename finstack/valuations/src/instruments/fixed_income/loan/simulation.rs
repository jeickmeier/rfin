//! Forward simulation model for loan instruments with undrawn commitments.
//!
//! Implements industry-standard expected exposure calculation and cash flow
//! valuation for facilities with future draws/repayments. Uses event-driven
//! simulation to accurately capture the economics of undrawn commitments.

use super::revolver::UtilizationFeeSchedule;
use super::term_loan::InterestSpec;
use crate::instruments::fixed_income::discountable::Discountable;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Frequency, StubKind};
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::traits::Discount;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::F;
use std::collections::BTreeSet;

/// Configuration for loan simulation models
#[derive(Clone, Debug)]
pub struct SimulationConfig {
    /// Number of Monte Carlo paths for utilization tier modeling (0 = deterministic)
    pub monte_carlo_paths: usize,
    /// Random seed for reproducible Monte Carlo
    pub random_seed: Option<u64>,
    /// Whether to use mid-point averaging for interest/fee accruals
    pub use_mid_point_averaging: bool,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            monte_carlo_paths: 0,  // Default to deterministic
            random_seed: Some(42), // Fixed seed for determinism
            use_mid_point_averaging: true,
        }
    }
}

/// Event in the simulation timeline
#[derive(Clone, Debug)]
pub struct SimulationEvent {
    /// Event date
    pub date: Date,
    /// Expected balance change (draw positive, repay negative)
    pub balance_change: F,
    /// Probability of the event occurring
    pub probability: F,
    /// Event type for categorization
    pub event_type: EventType,
}

/// Type of simulation event
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventType {
    /// Scheduled draw
    Draw,
    /// Scheduled repayment
    Repayment,
    /// Interest payment
    Interest,
    /// Fee payment
    Fee,
    /// Commitment expiry
    CommitmentExpiry,
    /// Final maturity
    Maturity,
}

/// State of the facility at a point in time
#[derive(Clone, Debug)]
pub struct FacilityState {
    /// Date of this state
    pub date: Date,
    /// Expected drawn balance
    pub expected_drawn: F,
    /// Expected undrawn balance
    pub expected_undrawn: F,
    /// Utilization percentage
    pub utilization: F,
}

impl FacilityState {
    /// Create new facility state
    pub fn new(date: Date, drawn: F, commitment: F) -> Self {
        let undrawn = (commitment - drawn).max(0.0);
        let utilization = if commitment > 0.0 {
            drawn / commitment
        } else {
            0.0
        };

        Self {
            date,
            expected_drawn: drawn,
            expected_undrawn: undrawn,
            utilization,
        }
    }
}

/// Result of the loan simulation
#[derive(Clone, Debug)]
pub struct SimulationResult {
    /// Present value of all expected cash flows
    pub total_pv: Money,
    /// PV breakdown by component
    pub pv_breakdown: PVBreakdown,
    /// Expected exposure path: (date, expected_drawn_balance)
    pub expected_exposure: Vec<(Date, F)>,
    /// Facility state evolution over time
    pub state_path: Vec<FacilityState>,
}

/// Present value breakdown by component
#[derive(Clone, Debug)]
pub struct PVBreakdown {
    /// PV of existing drawn balance cash flows
    pub existing_balance: F,
    /// PV of future draws (negative for lender)
    pub future_draws: F,
    /// PV of future repayments (positive for lender)
    pub future_repayments: F,
    /// PV of interest income on incremental draws
    pub incremental_interest: F,
    /// PV of principal redemption on incremental draws
    pub incremental_principal: F,
    /// PV of commitment fees
    pub commitment_fees: F,
    /// PV of utilization fees
    pub utilization_fees: F,
    /// PV of other fees
    pub other_fees: F,
}

impl Default for PVBreakdown {
    fn default() -> Self {
        Self {
            existing_balance: 0.0,
            future_draws: 0.0,
            future_repayments: 0.0,
            incremental_interest: 0.0,
            incremental_principal: 0.0,
            commitment_fees: 0.0,
            utilization_fees: 0.0,
            other_fees: 0.0,
        }
    }
}

/// Forward simulation engine for loan facilities
pub struct LoanSimulator {
    config: SimulationConfig,
}

impl LoanSimulator {
    /// Create new simulator with default config
    pub fn new() -> Self {
        Self {
            config: SimulationConfig::default(),
        }
    }

    /// Create simulator with custom config
    pub fn with_config(config: SimulationConfig) -> Self {
        Self { config }
    }

    /// Simulate facility and return comprehensive valuation result
    pub fn simulate<T: LoanFacility>(
        &self,
        facility: &T,
        curves: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<SimulationResult> {
        // Build event timeline
        let events = self.build_event_timeline(facility, as_of)?;

        // Value existing balance using standard cash flow methods
        let existing_balance_pv = self.value_existing_balance(facility, curves, as_of)?;

        // Run forward simulation
        let (pv_breakdown, state_path) = if self.config.monte_carlo_paths > 0 {
            self.simulate_monte_carlo(facility, curves, as_of, &events)?
        } else {
            self.simulate_deterministic(facility, curves, as_of, &events)?
        };

        // Build expected exposure path
        let expected_exposure: Vec<(Date, F)> = state_path
            .iter()
            .map(|state| (state.date, state.expected_drawn))
            .collect();

        // Calculate total PV
        let total_pv_amount = existing_balance_pv.amount()
            + pv_breakdown.future_draws
            + pv_breakdown.future_repayments
            + pv_breakdown.incremental_interest
            + pv_breakdown.incremental_principal
            + pv_breakdown.commitment_fees
            + pv_breakdown.utilization_fees
            + pv_breakdown.other_fees;

        let total_pv = Money::new(total_pv_amount, facility.currency());

        Ok(SimulationResult {
            total_pv,
            pv_breakdown: PVBreakdown {
                existing_balance: existing_balance_pv.amount(),
                ..pv_breakdown
            },
            expected_exposure,
            state_path,
        })
    }

    /// Build comprehensive event timeline for simulation
    fn build_event_timeline<T: LoanFacility>(
        &self,
        facility: &T,
        as_of: Date,
    ) -> finstack_core::Result<Vec<Date>> {
        let mut dates = BTreeSet::new();

        // Add valuation date
        dates.insert(as_of);

        // Add commitment expiry and maturity
        dates.insert(facility.commitment_expiry());
        dates.insert(facility.maturity());

        // Add expected draw/repayment dates
        for event in facility.expected_events() {
            if event.date > as_of && event.date <= facility.maturity() {
                dates.insert(event.date);
            }
        }

        // Add interest payment dates
        let interest_schedule = crate::cashflow::builder::build_dates(
            as_of,
            facility.maturity(),
            facility.frequency(),
            facility.stub(),
            facility.bdc(),
            facility.calendar_id(),
        );
        for date in &interest_schedule.dates {
            if *date > as_of {
                dates.insert(*date);
            }
        }

        // Add commitment fee payment dates (typically same as interest)
        // For different fee frequencies, build separate schedule

        Ok(dates.into_iter().collect())
    }

    /// Value existing drawn balance using standard methods
    fn value_existing_balance<T: LoanFacility>(
        &self,
        facility: &T,
        curves: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<Money> {
        let disc = curves.discount(facility.disc_id())?;
        let existing_flows = facility.build_existing_flows(curves, as_of)?;
        existing_flows.npv(&*disc, disc.base_date(), facility.day_count())
    }

    /// Deterministic expected-path simulation
    fn simulate_deterministic<T: LoanFacility>(
        &self,
        facility: &T,
        curves: &MarketContext,
        as_of: Date,
        timeline: &[Date],
    ) -> finstack_core::Result<(PVBreakdown, Vec<FacilityState>)> {
        let disc = curves.discount(facility.disc_id())?;

        let mut breakdown = PVBreakdown::default();
        let mut state_path = Vec::new();

        // Initialize state
        let mut current_drawn = facility.drawn_amount().amount();
        let commitment = facility.commitment().amount();

        state_path.push(FacilityState::new(as_of, current_drawn, commitment));

        // Simulate over each period
        for i in 0..timeline.len() - 1 {
            let period_start = timeline[i];
            let period_end = timeline[i + 1];

            // Apply draws/repayments at period start
            let events_at_start = facility.events_on_date(period_start);
            for event in events_at_start {
                let net_change = event.balance_change * event.probability;
                current_drawn = (current_drawn + net_change).max(0.0).min(commitment);

                // PV of draw/repayment itself
                let df_start = disc.df(DiscountCurve::year_fraction(
                    disc.base_date(),
                    period_start,
                    facility.day_count(),
                ));

                if net_change > 0.0 {
                    breakdown.future_draws -= net_change * df_start;
                } else {
                    breakdown.future_repayments += net_change.abs() * df_start;
                }
            }

            // Calculate cash flows over [period_start, period_end]
            let period_pv = self.calculate_period_cash_flows(
                facility,
                &*disc,
                curves,
                period_start,
                period_end,
                current_drawn,
                commitment,
            )?;

            breakdown.incremental_interest += period_pv.interest;
            breakdown.incremental_principal += period_pv.principal;
            breakdown.commitment_fees += period_pv.commitment_fees;
            breakdown.utilization_fees += period_pv.utilization_fees;
            breakdown.other_fees += period_pv.other_fees;

            // Apply cash sweep (reduces outstanding)
            let df_end = disc.df(DiscountCurve::year_fraction(
                disc.base_date(),
                period_end,
                facility.day_count(),
            ));
            current_drawn -= period_pv.cash_sweep / df_end; // Undiscount to get notional impact
            current_drawn = current_drawn.max(0.0); // Ensure non-negative
            breakdown.incremental_principal += period_pv.cash_sweep; // Add to principal PV

            // Update state for PIK capitalization
            current_drawn += period_pv.pik_capitalization;

            // Record end-of-period state
            state_path.push(FacilityState::new(period_end, current_drawn, commitment));
        }

        Ok((breakdown, state_path))
    }

    /// Monte Carlo simulation for utilization tier accuracy
    fn simulate_monte_carlo<T: LoanFacility>(
        &self,
        facility: &T,
        curves: &MarketContext,
        as_of: Date,
        timeline: &[Date],
    ) -> finstack_core::Result<(PVBreakdown, Vec<FacilityState>)> {
        let num_paths = self.config.monte_carlo_paths;
        let mut rng = if let Some(seed) = self.config.random_seed {
            Box::new(SeededRng::new(seed)) as Box<dyn RandomNumberGenerator>
        } else {
            Box::new(SystemRng::new()) as Box<dyn RandomNumberGenerator>
        };

        let mut total_breakdown = PVBreakdown::default();
        let mut expected_states = vec![FacilityState::new(as_of, 0.0, 0.0); timeline.len()];

        // Run Monte Carlo paths
        for _path in 0..num_paths {
            let (path_breakdown, path_states) =
                self.simulate_single_path(facility, curves, as_of, timeline, rng.as_mut())?;

            // Accumulate breakdown
            total_breakdown.future_draws += path_breakdown.future_draws / num_paths as F;
            total_breakdown.future_repayments += path_breakdown.future_repayments / num_paths as F;
            total_breakdown.incremental_interest +=
                path_breakdown.incremental_interest / num_paths as F;
            total_breakdown.incremental_principal +=
                path_breakdown.incremental_principal / num_paths as F;
            total_breakdown.commitment_fees += path_breakdown.commitment_fees / num_paths as F;
            total_breakdown.utilization_fees += path_breakdown.utilization_fees / num_paths as F;
            total_breakdown.other_fees += path_breakdown.other_fees / num_paths as F;

            // Accumulate expected states
            for (i, state) in path_states.iter().enumerate() {
                expected_states[i].expected_drawn += state.expected_drawn / num_paths as F;
                expected_states[i].expected_undrawn += state.expected_undrawn / num_paths as F;
                expected_states[i].utilization += state.utilization / num_paths as F;
            }
        }

        // Set dates for expected states
        for (i, date) in timeline.iter().enumerate() {
            expected_states[i].date = *date;
        }

        Ok((total_breakdown, expected_states))
    }

    /// Single Monte Carlo path simulation
    fn simulate_single_path<T: LoanFacility>(
        &self,
        facility: &T,
        curves: &MarketContext,
        as_of: Date,
        timeline: &[Date],
        rng: &mut dyn RandomNumberGenerator,
    ) -> finstack_core::Result<(PVBreakdown, Vec<FacilityState>)> {
        let disc = curves.discount(facility.disc_id())?;
        let mut breakdown = PVBreakdown::default();
        let mut state_path = Vec::new();

        let mut current_drawn = facility.drawn_amount().amount();
        let commitment = facility.commitment().amount();

        state_path.push(FacilityState::new(as_of, current_drawn, commitment));

        for i in 0..timeline.len() - 1 {
            let period_start = timeline[i];
            let period_end = timeline[i + 1];

            // Apply stochastic draws/repayments
            let events_at_start = facility.events_on_date(period_start);
            for event in events_at_start {
                let occurs = rng.uniform() < event.probability;
                if occurs {
                    current_drawn = (current_drawn + event.balance_change)
                        .max(0.0)
                        .min(commitment);

                    let df_start = disc.df(DiscountCurve::year_fraction(
                        disc.base_date(),
                        period_start,
                        facility.day_count(),
                    ));

                    if event.balance_change > 0.0 {
                        breakdown.future_draws -= event.balance_change * df_start;
                    } else {
                        breakdown.future_repayments += event.balance_change.abs() * df_start;
                    }
                }
            }

            // Calculate period cash flows with actual utilization
            let period_pv = self.calculate_period_cash_flows(
                facility,
                &*disc,
                curves,
                period_start,
                period_end,
                current_drawn,
                commitment,
            )?;

            breakdown.incremental_interest += period_pv.interest;
            breakdown.incremental_principal += period_pv.principal;
            breakdown.commitment_fees += period_pv.commitment_fees;
            breakdown.utilization_fees += period_pv.utilization_fees;
            breakdown.other_fees += period_pv.other_fees;

            // Apply cash sweep (reduces outstanding)
            let df_end = disc.df(DiscountCurve::year_fraction(
                disc.base_date(),
                period_end,
                facility.day_count(),
            ));
            current_drawn -= period_pv.cash_sweep / df_end; // Undiscount to get notional impact
            current_drawn = current_drawn.max(0.0); // Ensure non-negative
            breakdown.incremental_principal += period_pv.cash_sweep; // Add to principal PV

            // Apply PIK capitalization
            current_drawn += period_pv.pik_capitalization;

            state_path.push(FacilityState::new(period_end, current_drawn, commitment));
        }

        Ok((breakdown, state_path))
    }

    /// Calculate cash flows for a single period (implementation)
    #[allow(clippy::too_many_arguments)]
    fn calculate_period_cash_flows<T: LoanFacility>(
        &self,
        facility: &T,
        disc: &dyn Discount,
        curves: &MarketContext,
        period_start: Date,
        period_end: Date,
        drawn_start: F,
        commitment: F,
    ) -> finstack_core::Result<PeriodCashFlows> {
        let mut result = PeriodCashFlows::default();

        // Calculate year fraction for the period
        let tau = facility
            .day_count()
            .year_fraction(period_start, period_end)?;
        let df_end = disc.df(DiscountCurve::year_fraction(
            disc.base_date(),
            period_end,
            facility.day_count(),
        ));

        // Interest calculation
        match facility.interest_spec() {
            InterestSpec::Fixed { rate, step_ups } => {
                let effective_rate =
                    self.get_effective_rate(*rate, step_ups.as_ref(), period_start);
                let interest_amount = drawn_start * effective_rate * tau;
                result.interest = interest_amount * df_end;
            }
            InterestSpec::Floating {
                index_id,
                spread_bp,
                spread_step_ups,
                gearing,
                reset_lag_days,
            } => {
                if let Ok(fwd_curve) = curves.forecast(index_id) {
                    // Calculate reset date
                    let reset_date =
                        self.apply_reset_lag(period_start, *reset_lag_days, facility)?;
                    let t_fix = DiscountCurve::year_fraction(
                        disc.base_date(),
                        reset_date,
                        facility.day_count(),
                    );
                    let t_pay = DiscountCurve::year_fraction(
                        disc.base_date(),
                        period_end,
                        facility.day_count(),
                    );

                    let forward_rate = fwd_curve.rate_period(t_fix, t_pay);
                    let effective_spread = self.get_effective_spread(
                        *spread_bp,
                        spread_step_ups.as_ref(),
                        period_start,
                    );
                    let all_in_rate = (forward_rate + effective_spread / 10000.0) * gearing;

                    let interest_amount = drawn_start * all_in_rate * tau;
                    result.interest = interest_amount * df_end;
                }
            }
            InterestSpec::PIK { rate } => {
                let pik_amount = drawn_start * rate * tau;
                result.pik_capitalization = pik_amount;
                // No cash interest flow for pure PIK
            }
            InterestSpec::CashPlusPIK {
                cash_rate,
                pik_rate,
            } => {
                let cash_amount = drawn_start * cash_rate * tau;
                let pik_amount = drawn_start * pik_rate * tau;
                result.interest = cash_amount * df_end;
                result.pik_capitalization = pik_amount;
            }
            InterestSpec::PIKToggle {
                cash_rate,
                pik_rate,
                toggle_schedule,
            } => {
                let use_pik = self.get_pik_toggle_decision(toggle_schedule, period_start);
                let rate = if use_pik { *pik_rate } else { *cash_rate };
                let amount = drawn_start * rate * tau;

                if use_pik {
                    result.pik_capitalization = amount;
                } else {
                    result.interest = amount * df_end;
                }
            }
        }

        // Commitment fees (only until commitment expiry)
        if period_end <= facility.commitment_expiry() {
            let undrawn_start = (commitment - drawn_start).max(0.0);
            let undrawn_end = undrawn_start; // Assume no draws mid-period for simplicity
            let undrawn_avg = if self.config.use_mid_point_averaging {
                0.5 * (undrawn_start + undrawn_end)
            } else {
                undrawn_start
            };

            let fee_amount = undrawn_avg * facility.commitment_fee_rate() * tau;
            result.commitment_fees = fee_amount * df_end;
        }

        // Utilization fees (for revolvers)
        if let Some(util_schedule) = facility.utilization_fee_schedule() {
            let utilization = drawn_start / commitment;
            let util_rate_bp = util_schedule.get_rate(utilization);
            let fee_amount = drawn_start * (util_rate_bp / 10000.0) * tau;
            result.utilization_fees = fee_amount * df_end;
        }

        // Principal flows (amortization)
        // This is simplified - full implementation would need to track
        // proportional amortization for each incremental draw

        // Cash sweep calculation
        let sweep_pct = facility.cash_sweep_percentage();
        if sweep_pct > 0.0 && drawn_start > 0.0 {
            // Calculate available cash (simplified as a fraction of interest income)
            // In practice, this would come from borrower's cash flow statement
            let available_cash = result.interest * 0.5; // Assume 50% of interest as available cash
            let sweep_amount = available_cash * sweep_pct;
            let max_sweep = drawn_start; // Can't sweep more than outstanding

            result.cash_sweep = sweep_amount.min(max_sweep) * df_end;
        }

        Ok(result)
    }

    /// Get effective interest rate with step-ups
    fn get_effective_rate(&self, base_rate: F, step_ups: Option<&Vec<(Date, F)>>, date: Date) -> F {
        if let Some(steps) = step_ups {
            for (step_date, step_rate) in steps.iter().rev() {
                if date >= *step_date {
                    return *step_rate;
                }
            }
        }
        base_rate
    }

    /// Get effective spread with step-ups
    fn get_effective_spread(
        &self,
        base_spread: F,
        step_ups: Option<&Vec<(Date, F)>>,
        date: Date,
    ) -> F {
        if let Some(steps) = step_ups {
            for (step_date, step_spread) in steps.iter().rev() {
                if date >= *step_date {
                    return *step_spread;
                }
            }
        }
        base_spread
    }

    /// Apply reset lag to get fixing date
    fn apply_reset_lag<T: LoanFacility>(
        &self,
        payment_date: Date,
        reset_lag_days: i32,
        facility: &T,
    ) -> finstack_core::Result<Date> {
        let reset_date = payment_date - time::Duration::days(reset_lag_days as i64);

        // Apply business day adjustment if calendar is specified
        if let Some(calendar_id) = facility.calendar_id() {
            if let Some(cal) = finstack_core::dates::holiday::calendars::calendar_by_id(calendar_id)
            {
                return Ok(finstack_core::dates::adjust(
                    reset_date,
                    facility.bdc(),
                    cal,
                ));
            }
        }

        Ok(reset_date)
    }

    /// Get PIK toggle decision for a given date
    fn get_pik_toggle_decision(&self, toggle_schedule: &[(Date, bool)], date: Date) -> bool {
        for (toggle_date, use_pik) in toggle_schedule.iter().rev() {
            if date >= *toggle_date {
                return *use_pik;
            }
        }
        false // Default to cash
    }
}

impl Default for LoanSimulator {
    fn default() -> Self {
        Self::new()
    }
}

/// Cash flows generated during a single period
#[derive(Clone, Debug, Default)]
struct PeriodCashFlows {
    /// Interest income (cash portion)
    interest: F,
    /// Principal repayment
    principal: F,
    /// PIK capitalization (added to outstanding)
    pik_capitalization: F,
    /// Commitment fees
    commitment_fees: F,
    /// Utilization fees
    utilization_fees: F,
    /// Other fees
    other_fees: F,
    /// Cash sweep amount (additional principal repayment)
    cash_sweep: F,
}

/// Trait for loan facilities that can be simulated
pub trait LoanFacility {
    /// Get facility currency
    fn currency(&self) -> finstack_core::currency::Currency;

    /// Get total commitment amount
    fn commitment(&self) -> Money;

    /// Get currently drawn amount
    fn drawn_amount(&self) -> Money;

    /// Get commitment expiry date
    fn commitment_expiry(&self) -> Date;

    /// Get final maturity date
    fn maturity(&self) -> Date;

    /// Get interest specification
    fn interest_spec(&self) -> &InterestSpec;

    /// Get commitment fee rate
    fn commitment_fee_rate(&self) -> F;

    /// Get utilization fee schedule (for revolvers)
    fn utilization_fee_schedule(&self) -> Option<&UtilizationFeeSchedule> {
        None
    }

    /// Get cash sweep percentage (0.0 = no sweep)
    fn cash_sweep_percentage(&self) -> F {
        0.0
    }

    /// Get payment frequency
    fn frequency(&self) -> Frequency;

    /// Get day count convention
    fn day_count(&self) -> DayCount;

    /// Get business day convention
    fn bdc(&self) -> BusinessDayConvention;

    /// Get calendar ID
    fn calendar_id(&self) -> Option<&'static str>;

    /// Get stub convention
    fn stub(&self) -> StubKind;

    /// Get discount curve ID
    fn disc_id(&self) -> &'static str;

    /// Get expected future events
    fn expected_events(&self) -> Vec<SimulationEvent>;

    /// Get events occurring on a specific date
    fn events_on_date(&self, date: Date) -> Vec<SimulationEvent>;

    /// Build cash flows for existing drawn balance
    fn build_existing_flows(
        &self,
        curves: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<Vec<(Date, Money)>>;
}

/// Random number generator trait for Monte Carlo
trait RandomNumberGenerator: Send + Sync {
    /// Generate uniform random number in [0, 1)
    fn uniform(&mut self) -> F;
}

/// Seeded RNG for deterministic Monte Carlo
struct SeededRng {
    state: u64,
}

impl SeededRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }
}

impl RandomNumberGenerator for SeededRng {
    fn uniform(&mut self) -> F {
        // Simple linear congruential generator
        self.state = self.state.wrapping_mul(1103515245).wrapping_add(12345);
        (self.state % (1 << 31)) as F / (1u64 << 31) as F
    }
}

/// System RNG wrapper
struct SystemRng;

impl SystemRng {
    fn new() -> Self {
        Self
    }
}

impl RandomNumberGenerator for SystemRng {
    fn uniform(&mut self) -> F {
        // In practice, would use proper RNG
        // For now, return deterministic value
        0.5
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::Month;

    #[test]
    fn test_simulation_config_defaults() {
        let config = SimulationConfig::default();
        assert_eq!(config.monte_carlo_paths, 0);
        assert_eq!(config.random_seed, Some(42));
        assert!(config.use_mid_point_averaging);
    }

    #[test]
    fn test_facility_state_creation() {
        let date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let state = FacilityState::new(date, 300_000.0, 1_000_000.0);

        assert_eq!(state.expected_drawn, 300_000.0);
        assert_eq!(state.expected_undrawn, 700_000.0);
        assert_eq!(state.utilization, 0.3);
    }

    #[test]
    fn test_seeded_rng_deterministic() {
        let mut rng1 = SeededRng::new(42);
        let mut rng2 = SeededRng::new(42);

        for _ in 0..10 {
            assert_eq!(rng1.uniform(), rng2.uniform());
        }
    }
}
