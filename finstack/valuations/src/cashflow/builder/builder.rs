//! Cash-flow builder API and build orchestration.
//!
//! This module contains the main `CashFlowBuilder` API that accumulates principal,
//! amortization, coupon windows, and fees, then orchestrates the compilation into
//! a deterministic `CashFlowSchedule`.
//!
//! ## Responsibilities
//!
//! - `CashFlowBuilder` struct and its public builder methods
//! - Build orchestration (validation, compilation, date collection, state management)
//! - Pipeline stages: validate inputs, compile schedules, initialize state, process dates
//! - Amortization setup and parameter derivation
//! - Integration with emission, compiler, and date generation modules
//!
//! Quick start
//! -----------
//! ```rust
//! use finstack_core::currency::Currency;
//! use finstack_core::dates::{Date, Tenor, DayCount, BusinessDayConvention};
//! use finstack_core::dates::StubKind;
//! use finstack_core::money::Money;
//! use finstack_valuations::cashflow::builder::{CashFlowSchedule, FixedCouponSpec, CouponType};
//! use rust_decimal_macros::dec;
//! use time::Month;
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//!
//! let issue = Date::from_calendar_date(2025, Month::January, 15).expect("valid date");
//! let maturity = Date::from_calendar_date(2026, Month::January, 15).expect("valid date");
//! let mut b = CashFlowSchedule::builder();
//! b.principal(Money::new(1_000.0, Currency::USD), issue, maturity)
//!  .fixed_cf(FixedCouponSpec{
//!      coupon_type: CouponType::Cash,
//!      rate: dec!(0.05),
//!      freq: Tenor::semi_annual(),
//!      dc: DayCount::Act365F,
//!      bdc: BusinessDayConvention::Following,
//!      calendar_id: "weekends_only".to_string(),
//!      end_of_month: false,
//!      payment_lag_days: 0,
//!      stub: StubKind::None,
//!  });
//! let schedule = b.build_with_curves(None)
//!     .map_err(|e| format!("Failed to build cashflow schedule: {}", e))?;
//! assert!(!schedule.flows.is_empty());
//! # Ok(())
//! # }
//! ```

use super::schedule::{finalize_flows, CashFlowSchedule};
use crate::cashflow::builder::{AmortizationSpec, FloatingRateSpec, Notional};
use crate::cashflow::primitives::{CFKind, CashFlow};
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::term_structures::ForwardCurve;
use finstack_core::money::Money;
use finstack_core::InputError;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use std::sync::Arc;

use super::compiler::{
    build_fee_schedules, collect_dates, compute_coupon_schedules, CompiledSchedules,
    CouponProgramPiece, CouponSpec, DateWindow, FixedSchedule, FloatSchedule, PaymentProgramPiece,
    PeriodicFee,
};
use super::emission::{
    emit_amortization_on, emit_fees_on, emit_fixed_coupons_on, emit_float_coupons_on,
    AmortizationParams,
};
use super::specs::{
    CouponType, FeeSpec, FixedCouponSpec, FixedWindow, FloatCouponParams, FloatWindow,
    FloatingCouponSpec, ScheduleParams, StepUpCouponSpec,
};
use smallvec::SmallVec;

// -------------------------------------------------------------------------
// Pipeline scaffolding — pure-ish stages and fold state
// -------------------------------------------------------------------------

/// Internal state accumulated during schedule building.
#[derive(Debug, Clone)]
struct BuildState {
    flows: Vec<CashFlow>,
    outstanding_after: finstack_core::HashMap<Date, f64>,
    /// Outstanding balance tracked as f64 for performance.
    ///
    /// For typical instruments (< 200 periods), f64 accumulation error is negligible
    /// (< 1e-12 relative error). For very long-dated instruments with many small
    /// cashflows, consider validating final outstanding against expected value.
    outstanding: f64,
}

/// Principal event applied during schedule build (draws/repays).
///
/// `delta` adjusts outstanding (positive increases, negative decreases).
/// `cash` represents the cash leg (e.g., net of OID/fees). If `delta` differs
/// from `cash`, the difference is interpreted as non-cash adjustments.
/// `kind` classifies the cashflow (Notional/Amortization/etc.).
#[derive(Debug, Clone)]
pub struct PrincipalEvent {
    /// Event date
    pub date: Date,
    /// Outstanding delta (positive = increases balance, negative = repays)
    pub delta: Money,
    /// Cash leg paid/received (may differ from delta for OID/fees)
    pub cash: Money,
    /// Classification for emitted cashflow
    pub kind: CFKind,
}

#[derive(Debug, Clone)]
struct AmortizationSetup {
    amort_dates: finstack_core::HashSet<Date>,
    step_remaining_map: Option<finstack_core::HashMap<Date, Money>>, // for StepRemaining
    linear_delta: Option<f64>,                                       // for LinearTo
    percent_per: Option<f64>, // for PercentOfOriginalPerPeriod
}

#[derive(Debug, Clone, Copy)]
struct BuildContext<'a> {
    ccy: Currency,
    maturity: Date,
    notional: &'a Notional,
    fixed_schedules: &'a [FixedSchedule],
    float_schedules: &'a [FloatSchedule],
    periodic_fees: &'a [PeriodicFee],
    fixed_fees: &'a [(Date, Money)],
    principal_events: &'a [PrincipalEvent],
}

/// Grouped inputs for collecting all relevant schedule dates.
#[derive(Debug, Clone, Copy)]
struct DateCollectionInputs<'a> {
    issue: Date,
    maturity: Date,
    fixed_schedules: &'a [FixedSchedule],
    float_schedules: &'a [FloatSchedule],
    periodic_fees: &'a [PeriodicFee],
    fixed_fees: &'a [(Date, Money)],
    notional: &'a Notional,
    principal_events: &'a [PrincipalEvent],
}

fn validate_core_inputs(b: &CashFlowBuilder) -> finstack_core::Result<(Notional, Date, Date)> {
    let notional = b.notional.clone().ok_or_else(|| InputError::NotFound {
        id: "notional (call principal() first)".into(),
    })?;
    let issue = b.issue.ok_or_else(|| InputError::NotFound {
        id: "issue date (call principal() first)".into(),
    })?;
    let maturity = b.maturity.ok_or_else(|| InputError::NotFound {
        id: "maturity date (call principal() first)".into(),
    })?;

    // Validate notional and amortization spec (e.g., total amortization <= notional)
    notional.validate()?;

    Ok((notional, issue, maturity))
}

type CompiledAndFees = (CompiledSchedules, Vec<PeriodicFee>, Vec<(Date, Money)>);

fn compile_schedules_and_fees(
    b: &CashFlowBuilder,
    issue: Date,
    maturity: Date,
) -> finstack_core::Result<CompiledAndFees> {
    // Centralized wrapper so the main build pipeline remains focused on the
    // high-level orchestration (validate → compile → collect dates → emit).
    // Keeping this helper avoids repeating the tuple wiring in both
    // documentation and any future build variants.
    let compiled = compute_coupon_schedules(b, issue, maturity)?;
    let (periodic_fees, fixed_fees) = build_fee_schedules(issue, maturity, &b.fees)?;
    Ok((compiled, periodic_fees, fixed_fees))
}

fn derive_amortization_setup(
    notional: &Notional,
    fixed_schedules: &[FixedSchedule],
    float_schedules: &[FloatSchedule],
) -> finstack_core::Result<AmortizationSetup> {
    // Determine base cadence schedule for linear/percent amortization by
    // borrowing the first available schedule instead of cloning dates.
    let amort_base: Option<&[Date]> = match notional.amort {
        AmortizationSpec::LinearTo { .. } | AmortizationSpec::PercentOfOriginalPerPeriod { .. } => {
            if let Some((_, ds, _, _)) = fixed_schedules.first() {
                Some(ds.as_slice())
            } else if let Some((_, ds, _)) = float_schedules.first() {
                Some(ds.as_slice())
            } else {
                None
            }
        }
        _ => None,
    };

    if amort_base.is_none()
        && matches!(
            notional.amort,
            AmortizationSpec::LinearTo { .. } | AmortizationSpec::PercentOfOriginalPerPeriod { .. }
        )
    {
        return Err(InputError::Invalid.into());
    }

    // Precompute helpers depending on amort spec
    let step_remaining_map: Option<finstack_core::HashMap<Date, Money>> = match &notional.amort {
        AmortizationSpec::StepRemaining { schedule } => {
            let mut m = finstack_core::HashMap::default();
            m.reserve(schedule.len());
            for (d, mny) in schedule {
                m.insert(*d, *mny);
            }
            Some(m)
        }
        _ => None,
    };

    let (linear_delta, percent_per) = match &notional.amort {
        AmortizationSpec::LinearTo { final_notional } => {
            let base = amort_base.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::InputError::NotFound {
                    id: "amortization_base_schedule".to_string(),
                })
            })?;
            let steps = base.len() as f64;
            (
                Some(((notional.initial.amount() - final_notional.amount()) / steps).max(0.0)),
                None,
            )
        }
        AmortizationSpec::PercentOfOriginalPerPeriod { pct } => {
            (None, Some((notional.initial.amount() * *pct).max(0.0)))
        }
        _ => (None, None),
    };

    let amort_dates: finstack_core::HashSet<Date> = amort_base
        .map(|v| v.iter().copied().collect())
        .unwrap_or_default();

    Ok(AmortizationSetup {
        amort_dates,
        step_remaining_map,
        linear_delta,
        percent_per,
    })
}

fn initialize_build_state(
    issue: Date,
    notional: &Notional,
    estimated_dates: usize,
    principal_events: &[PrincipalEvent],
) -> BuildState {
    // Pre-allocate flows: estimate 2-3 flows per date (coupon + potential amort + fee)
    let estimated_flows = estimated_dates * 3;
    let mut flows: Vec<CashFlow> = Vec::with_capacity(estimated_flows);

    // Only emit initial notional flow if non-zero.
    // Safety: Money::new(0.0, _) produces exact zero via Decimal; direct comparison is safe.
    if notional.initial.amount() != 0.0 {
        flows.push(CashFlow {
            date: issue,
            reset_date: None,
            amount: notional.initial * -1.0,
            kind: CFKind::Notional,
            accrual_factor: 0.0,
            rate: None,
        });
    }

    // Start with initial notional amount
    let mut outstanding = notional.initial.amount();

    // Process principal events at or before issue date to set up initial outstanding.
    // This is critical when initial notional is 0 and principal events define the draws.
    for ev in principal_events.iter().filter(|ev| ev.date <= issue) {
        // Safety: Money amounts from builder are exact Decimal values; direct comparison is safe.
        if ev.delta.amount() != 0.0 || ev.cash.amount() != 0.0 {
            // Sign convention depends on flow kind:
            // - Notional (draws): cash is inflow to borrower, flow is negative (funding outflow from lender)
            // - Amortization: cash is repayment, flow is positive (inflow to lender)
            let flow_amount = match ev.kind {
                CFKind::Amortization => ev.cash.amount(),
                _ => -ev.cash.amount(),
            };
            flows.push(CashFlow {
                date: ev.date,
                reset_date: None,
                amount: Money::new(flow_amount, ev.cash.currency()),
                kind: ev.kind,
                accrual_factor: 0.0,
                rate: None,
            });
            outstanding += ev.delta.amount();
        }
    }

    // Pre-allocate outstanding_after based on number of dates
    let mut outstanding_after: finstack_core::HashMap<Date, f64> =
        finstack_core::HashMap::default();
    outstanding_after.reserve(estimated_dates);
    outstanding_after.insert(issue, outstanding);

    BuildState {
        flows,
        outstanding_after,
        outstanding,
    }
}

fn collect_all_dates(inputs: &DateCollectionInputs<'_>) -> finstack_core::Result<Vec<Date>> {
    let periodic_date_slices: Vec<&[Date]> = inputs
        .periodic_fees
        .iter()
        .map(|pf| pf.dates.as_slice())
        .collect();
    let mut dates: Vec<Date> = collect_dates(
        inputs.issue,
        inputs.maturity,
        inputs.fixed_schedules,
        inputs.float_schedules,
        &periodic_date_slices,
        inputs.fixed_fees,
        inputs.notional,
    );
    // Include accrual boundaries for periodic fees to ensure outstanding is tracked at accrual start.
    for pf in inputs.periodic_fees {
        for period in pf.prev.values() {
            dates.push(period.accrual_start);
            dates.push(period.accrual_end);
        }
    }
    for ev in inputs.principal_events {
        dates.push(ev.date);
    }
    // Re-sort and deduplicate after adding principal event dates
    dates.sort_unstable();
    dates.dedup();
    if dates.len() < 2 {
        return Err(InputError::TooFewPoints.into());
    }
    Ok(dates)
}

// =============================================================================
// DateProcessor: Encapsulates single-date processing stages
// =============================================================================
//
// This struct groups all context needed for processing a single date in the
// cashflow schedule build. Each stage is a separate method for clarity and
// unit testability.

/// Processes cashflows for a single date in the schedule build.
///
/// Encapsulates the context and provides methods for each processing stage:
/// - Coupon emission (fixed and floating)
/// - Amortization
/// - PIK capitalization
/// - Fee emission
/// - Principal events
/// - Maturity handling
struct DateProcessor<'a> {
    ctx: &'a BuildContext<'a>,
    amort_setup: &'a AmortizationSetup,
    resolved_curves: &'a [Option<Arc<ForwardCurve>>],
}

impl<'a> DateProcessor<'a> {
    /// Create a new date processor with the given context.
    fn new(
        ctx: &'a BuildContext<'a>,
        amort_setup: &'a AmortizationSetup,
        resolved_curves: &'a [Option<Arc<ForwardCurve>>],
    ) -> Self {
        Self {
            ctx,
            amort_setup,
            resolved_curves,
        }
    }

    /// Emit fixed and floating coupons, returning total PIK amount to capitalize.
    fn emit_coupons(&self, d: Date, state: &mut BuildState) -> finstack_core::Result<f64> {
        let pik_f = emit_fixed_coupons_on(
            d,
            self.ctx.fixed_schedules,
            &state.outstanding_after,
            state.outstanding,
            self.ctx.ccy,
            &mut state.flows,
        )?;
        let pik_fl = emit_float_coupons_on(
            d,
            self.ctx.float_schedules,
            &state.outstanding_after,
            state.outstanding,
            self.ctx.ccy,
            self.resolved_curves,
            &mut state.flows,
        )?;
        Ok(pik_f + pik_fl)
    }

    /// Emit amortization flows based on the amortization spec.
    fn emit_amortization(&self, d: Date, state: &mut BuildState) -> finstack_core::Result<()> {
        let amort_params = AmortizationParams {
            ccy: self.ctx.ccy,
            amort_dates: &self.amort_setup.amort_dates,
            linear_delta: self.amort_setup.linear_delta,
            percent_per: self.amort_setup.percent_per,
            step_remaining_map: &self.amort_setup.step_remaining_map,
        };
        emit_amortization_on(
            d,
            self.ctx.notional,
            &mut state.outstanding,
            &amort_params,
            d == self.ctx.maturity,
            &mut state.flows,
        )
    }

    /// Emit fee flows (periodic and fixed).
    fn emit_fees(&self, d: Date, state: &mut BuildState) -> finstack_core::Result<()> {
        emit_fees_on(
            d,
            self.ctx.periodic_fees,
            self.ctx.fixed_fees,
            state.outstanding,
            self.ctx.ccy,
            &mut state.flows,
        )
    }

    /// Process custom principal events (draws/repays) for this date.
    fn process_principal_events(&self, d: Date, state: &mut BuildState) {
        for ev in self.ctx.principal_events.iter().filter(|ev| ev.date == d) {
            // Safety: Money amounts from builder are exact Decimal values; direct comparison is safe.
            if ev.delta.amount() != 0.0 || ev.cash.amount() != 0.0 {
                // Sign convention depends on flow kind:
                // - Notional (draws): cash is inflow to borrower, flow is negative (funding outflow)
                // - Amortization: cash is repayment, flow is positive (inflow to lender)
                let flow_amount = match ev.kind {
                    CFKind::Amortization => ev.cash.amount(),
                    _ => -ev.cash.amount(),
                };
                state.flows.push(CashFlow {
                    date: d,
                    reset_date: None,
                    amount: Money::new(flow_amount, ev.cash.currency()),
                    kind: ev.kind,
                    accrual_factor: 0.0,
                    rate: None,
                });
                state.outstanding += ev.delta.amount();
            }
        }
    }

    /// Handle maturity redemption: emit final principal repayment if outstanding > 0.
    fn handle_maturity(&self, d: Date, state: &mut BuildState) {
        if d == self.ctx.maturity && state.outstanding > 0.0 {
            state.flows.push(CashFlow {
                date: d,
                reset_date: None,
                amount: Money::new(state.outstanding, self.ctx.ccy),
                kind: CFKind::Notional,
                accrual_factor: 0.0,
                rate: None,
            });
            state.outstanding = 0.0;
        }
    }

    /// Process all stages for a single date.
    ///
    /// # Processing Order
    ///
    /// The ordering of stages is critical for correctness, particularly the
    /// interaction between PIK capitalization and amortization:
    ///
    /// 1. **Coupons** — Emit cash and PIK coupon flows based on the *current*
    ///    outstanding balance. PIK amounts are returned but **not yet added**
    ///    to the outstanding balance.
    /// 2. **Amortization** — Evaluate principal repayments against the current
    ///    outstanding balance (before PIK capitalization). For `StepRemaining`
    ///    schedules this means the target remaining balance is compared to the
    ///    pre-PIK outstanding, which is the standard market convention.
    /// 3. **PIK capitalization** — Only *after* amortization is processed, the
    ///    PIK coupon amount is added to the outstanding balance. This ensures
    ///    that amortization targets are evaluated on the "clean" outstanding
    ///    balance, and PIK capitalization increases the base for subsequent
    ///    periods.
    /// 4. **Fees** — Facility and usage fees emitted on updated outstanding.
    /// 5. **Principal events** — Discretionary draws, repayments, etc.
    /// 6. **Maturity handling** — Final redemption of remaining outstanding.
    fn process(&self, d: Date, mut state: BuildState) -> finstack_core::Result<BuildState> {
        // 1. Coupons (cash + PIK split; PIK amount returned but not yet capitalized)
        let pik_to_add = self.emit_coupons(d, &mut state)?;

        // 2. Amortization (evaluated against pre-PIK outstanding)
        self.emit_amortization(d, &mut state)?;

        // 3. PIK capitalization (increases outstanding for future periods)
        if pik_to_add > 0.0 {
            state.outstanding += pik_to_add;
        }

        // 4. Fees
        self.emit_fees(d, &mut state)?;

        // 5. Principal events
        self.process_principal_events(d, &mut state);

        // 6. Maturity handling
        self.handle_maturity(d, &mut state);

        // Record outstanding for this date
        state.outstanding_after.insert(d, state.outstanding);

        Ok(state)
    }
}

fn process_one_date(
    d: Date,
    state: BuildState,
    ctx: &BuildContext,
    amort_setup: &AmortizationSetup,
    resolved_curves: &[Option<Arc<ForwardCurve>>],
) -> finstack_core::Result<BuildState> {
    let processor = DateProcessor::new(ctx, amort_setup, resolved_curves);
    processor.process(d, state)
}

// -------------------------------------------------------------------------
// Segmented coupon program primitives (references from compile.rs)
// -------------------------------------------------------------------------

/// Builder for constructing cashflow schedules with validation.
///
/// Provides a fluent API for building complex cashflow schedules with
/// proper validation and business day adjustments.
#[derive(Debug, Clone)]
pub struct CashFlowBuilder {
    notional: Option<Notional>,
    issue: Option<Date>,
    maturity: Option<Date>,
    /// Fee specifications. SmallVec<4> avoids heap allocation for typical instruments
    /// with ≤4 fee specs (commitment fee, facility fee, usage fee, admin fee).
    fees: SmallVec<[FeeSpec; 4]>,
    principal_events: Vec<PrincipalEvent>,
    // Segmented programs (optional): coupon program and payment/PIK program
    pub(super) coupon_program: Vec<CouponProgramPiece>,
    pub(super) payment_program: Vec<PaymentProgramPiece>,
    // Sticky builder error for fluent APIs that cannot return Result.
    pending_error: Option<finstack_core::Error>,
}

impl Default for CashFlowBuilder {
    fn default() -> Self {
        Self {
            notional: None,
            issue: None,
            maturity: None,
            fees: SmallVec::new(),
            principal_events: Vec::new(),
            coupon_program: Vec::new(),
            payment_program: Vec::new(),
            pending_error: None,
        }
    }
}

impl CashFlowBuilder {
    fn issue_maturity_error(method_name: &str) -> finstack_core::Error {
        InputError::NotFound {
            id: format!(
                "CashFlowBuilder::{} requires principal() (issue/maturity) to be set first",
                method_name
            ),
        }
        .into()
    }

    fn issue_maturity_or_error(&self, method_name: &str) -> finstack_core::Result<(Date, Date)> {
        match (self.issue, self.maturity) {
            (Some(issue), Some(maturity)) => Ok((issue, maturity)),
            _ => Err(Self::issue_maturity_error(method_name)),
        }
    }

    fn issue_maturity_or_record_error(&mut self, method_name: &str) -> Option<(Date, Date)> {
        if self.pending_error.is_some() {
            return None;
        }
        match self.issue_maturity_or_error(method_name) {
            Ok(v) => Some(v),
            Err(e) => {
                self.pending_error = Some(e);
                None
            }
        }
    }
    /// Sets principal details and instrument horizon.
    #[must_use = "builder methods should be chained or terminated with .build_with_curves(...)"]
    pub fn principal(&mut self, initial: Money, issue_date: Date, maturity: Date) -> &mut Self {
        self.pending_error = None;
        self.notional = Some(Notional {
            initial,
            amort: AmortizationSpec::None,
        });
        self.issue = Some(issue_date);
        self.maturity = Some(maturity);
        self
    }

    /// Convenience helper to set principal by amount and currency.
    #[must_use = "builder methods should be chained or terminated with .build_with_curves(...)"]
    pub fn principal_amount(
        &mut self,
        amount: f64,
        currency: Currency,
        issue_date: Date,
        maturity: Date,
    ) -> &mut Self {
        self.principal(Money::new(amount, currency), issue_date, maturity)
    }

    /// Configures amortization on the current notional.
    #[must_use = "builder methods should be chained or terminated with .build_with_curves(...)"]
    pub fn amortization(&mut self, spec: AmortizationSpec) -> &mut Self {
        if let Some(n) = &mut self.notional {
            n.amort = spec;
        }
        self
    }

    /// Adds a fixed coupon specification.
    #[must_use = "builder methods should be chained or terminated with .build_with_curves(...)"]
    pub fn fixed_cf(&mut self, spec: FixedCouponSpec) -> &mut Self {
        let Some((issue, maturity)) = self.issue_maturity_or_record_error("fixed_cf") else {
            return self;
        };
        let FixedCouponSpec {
            coupon_type,
            rate,
            freq,
            dc,
            bdc,
            calendar_id,
            stub,
            end_of_month,
            payment_lag_days,
        } = spec;
        self.coupon_program.push(CouponProgramPiece {
            window: DateWindow {
                start: issue,
                end: maturity,
            },
            schedule: ScheduleParams {
                freq,
                dc,
                bdc,
                calendar_id,
                stub,
                end_of_month,
                payment_lag_days,
            },
            coupon: CouponSpec::Fixed { rate },
        });
        self.payment_program.push(PaymentProgramPiece {
            window: DateWindow {
                start: issue,
                end: maturity,
            },
            split: coupon_type,
        });
        self
    }

    /// Adds a floating coupon specification.
    #[must_use = "builder methods should be chained or terminated with .build_with_curves(...)"]
    pub fn floating_cf(&mut self, spec: FloatingCouponSpec) -> &mut Self {
        let Some((issue, maturity)) = self.issue_maturity_or_record_error("floating_cf") else {
            return self;
        };
        let FloatingCouponSpec {
            rate_spec,
            coupon_type,
            freq,
            stub,
        } = spec;
        let FloatingRateSpec {
            index_id,
            spread_bp,
            gearing,
            gearing_includes_spread,
            floor_bp,
            cap_bp,
            all_in_floor_bp,
            index_cap_bp,
            reset_freq: _,
            reset_lag_days,
            dc,
            bdc,
            calendar_id,
            fixing_calendar_id,
            end_of_month,
            payment_lag_days,
            overnight_compounding,
            fallback,
        } = rate_spec;
        self.coupon_program.push(CouponProgramPiece {
            window: DateWindow {
                start: issue,
                end: maturity,
            },
            schedule: ScheduleParams {
                freq,
                dc,
                bdc,
                calendar_id: calendar_id.clone(),
                stub,
                end_of_month,
                payment_lag_days,
            },
            coupon: CouponSpec::Float {
                index_id,
                margin_bp: spread_bp,
                gearing,
                reset_lag_days,
                gearing_includes_spread,
                floor_bp,
                cap_bp,
                all_in_floor_bp,
                index_cap_bp,
                fixing_calendar_id,
                overnight_compounding,
                fallback,
            },
        });
        self.payment_program.push(PaymentProgramPiece {
            window: DateWindow {
                start: issue,
                end: maturity,
            },
            split: coupon_type,
        });
        self
    }

    /// Adds a step-up coupon specification.
    ///
    /// Decomposes the step-up schedule into fixed coupon windows, one per rate period.
    #[must_use = "builder methods should be chained or terminated with .build_with_curves(...)"]
    pub fn step_up_cf(&mut self, spec: StepUpCouponSpec) -> &mut Self {
        let Some((issue, maturity)) = self.issue_maturity_or_record_error("step_up_cf") else {
            return self;
        };
        let coupon_type = spec.coupon_type;
        for (start, end, fixed_spec) in spec.to_fixed_windows(issue, maturity) {
            self.coupon_program.push(CouponProgramPiece {
                window: DateWindow { start, end },
                schedule: ScheduleParams {
                    freq: fixed_spec.freq,
                    dc: fixed_spec.dc,
                    bdc: fixed_spec.bdc,
                    calendar_id: fixed_spec.calendar_id.clone(),
                    stub: fixed_spec.stub,
                    end_of_month: fixed_spec.end_of_month,
                    payment_lag_days: fixed_spec.payment_lag_days,
                },
                coupon: CouponSpec::Fixed {
                    rate: fixed_spec.rate,
                },
            });
            self.payment_program.push(PaymentProgramPiece {
                window: DateWindow { start, end },
                split: coupon_type,
            });
        }
        self
    }

    /// Adds a fee specification.
    #[must_use = "builder methods should be chained or terminated with .build_with_curves(...)"]
    pub fn fee(&mut self, spec: FeeSpec) -> &mut Self {
        self.fees.push(spec);
        self
    }

    /// Adds custom principal events (draws/repays) that adjust outstanding balance.
    ///
    /// `delta` increases outstanding when positive and decreases when negative.
    /// `cash` is the actual cash leg (e.g., net of OID); if omitted, cash = delta.
    ///
    /// # Errors
    ///
    /// Records a pending error if any event has mismatched currencies between
    /// `delta` and `cash`. The error will be returned when `build_with_curves(...)` is called.
    #[must_use = "builder methods should be chained or terminated with .build_with_curves(...)"]
    pub fn principal_events(&mut self, events: &[PrincipalEvent]) -> &mut Self {
        if self.pending_error.is_some() {
            return self;
        }
        for ev in events {
            if ev.cash.currency() != ev.delta.currency() {
                self.pending_error = Some(finstack_core::Error::CurrencyMismatch {
                    expected: ev.delta.currency(),
                    actual: ev.cash.currency(),
                });
                return self;
            }
        }
        self.principal_events.extend(events.iter().cloned());
        self
    }

    /// Adds a single principal event.
    ///
    /// # Errors
    ///
    /// Records a pending error if `cash` is provided with a different currency
    /// than `delta`. The error will be returned when `build_with_curves(...)` is called.
    #[must_use = "builder methods should be chained or terminated with .build_with_curves(...)"]
    pub fn add_principal_event(
        &mut self,
        date: Date,
        delta: Money,
        cash: Option<Money>,
        kind: CFKind,
    ) -> &mut Self {
        if self.pending_error.is_some() {
            return self;
        }
        let cash_leg = cash.unwrap_or(delta);
        if cash_leg.currency() != delta.currency() {
            self.pending_error = Some(finstack_core::Error::CurrencyMismatch {
                expected: delta.currency(),
                actual: cash_leg.currency(),
            });
            return self;
        }
        self.principal_events.push(PrincipalEvent {
            date,
            delta,
            cash: cash_leg,
            kind,
        });
        self
    }

    /// Adds a fixed coupon window with its own schedule and payment split (cash/PIK/split).
    #[must_use = "builder methods should be chained or terminated with .build_with_curves(...)"]
    pub fn add_fixed_coupon_window(
        &mut self,
        start: Date,
        end: Date,
        rate: f64,
        schedule: ScheduleParams,
        split: CouponType,
    ) -> &mut Self {
        // Convert f64 rate to Decimal for exact representation
        let rate_decimal = Decimal::try_from(rate).unwrap_or(Decimal::ZERO);
        self.coupon_program.push(CouponProgramPiece {
            window: DateWindow { start, end },
            schedule,
            coupon: CouponSpec::Fixed { rate: rate_decimal },
        });
        self.payment_program.push(PaymentProgramPiece {
            window: DateWindow { start, end },
            split,
        });
        self
    }

    /// Adds a floating coupon window with its own schedule and payment split.
    #[must_use = "builder methods should be chained or terminated with .build_with_curves(...)"]
    pub fn add_float_coupon_window(
        &mut self,
        start: Date,
        end: Date,
        params: FloatCouponParams,
        schedule: ScheduleParams,
        split: CouponType,
    ) -> &mut Self {
        self.coupon_program.push(CouponProgramPiece {
            window: DateWindow { start, end },
            schedule,
            coupon: CouponSpec::Float {
                index_id: params.index_id,
                margin_bp: params.margin_bp,
                gearing: params.gearing,
                reset_lag_days: params.reset_lag_days,
                gearing_includes_spread: params.gearing_includes_spread,
                floor_bp: params.floor_bp,
                cap_bp: params.cap_bp,
                all_in_floor_bp: params.all_in_floor_bp,
                index_cap_bp: params.index_cap_bp,
                fixing_calendar_id: params.fixing_calendar_id,
                overnight_compounding: params.overnight_compounding,
                fallback: params.fallback,
            },
        });
        self.payment_program.push(PaymentProgramPiece {
            window: DateWindow { start, end },
            split,
        });
        self
    }

    /// Adds/overrides a payment split (cash/PIK/split) over a window (PIK toggle support).
    #[must_use = "builder methods should be chained or terminated with .build_with_curves(...)"]
    pub fn add_payment_window(&mut self, start: Date, end: Date, split: CouponType) -> &mut Self {
        self.payment_program.push(PaymentProgramPiece {
            window: DateWindow { start, end },
            split,
        });
        self
    }

    /// Convenience: fixed step-up program using boundary dates.
    ///
    /// Creates a series of fixed-rate coupon windows where the rate changes at
    /// specified boundary dates. Common for step-up bonds where the coupon rate
    /// increases over time to compensate for credit deterioration risk.
    ///
    /// # Arguments
    ///
    /// * `steps` - Boundary dates and rates: `&[(end_date, rate)]`
    /// * `schedule` - Common schedule parameters (frequency, day count, etc.)
    /// * `default_split` - Payment type (Cash, PIK, or Split) for all windows
    ///
    /// # Example
    ///
    /// ```rust
    /// use finstack_core::currency::Currency;
    /// use finstack_core::dates::{Date, Tenor, DayCount, BusinessDayConvention, StubKind};
    /// use finstack_core::money::Money;
    /// use finstack_valuations::cashflow::builder::{CashFlowSchedule, ScheduleParams, CouponType};
    /// use time::Month;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let issue = Date::from_calendar_date(2025, Month::January, 1)?;
    /// let maturity = Date::from_calendar_date(2028, Month::January, 1)?;
    ///
    /// // Step-up bond: 4% for first year, 5% for second year, 6% thereafter
    /// let steps = [
    ///     (Date::from_calendar_date(2026, Month::January, 1)?, 0.04),
    ///     (Date::from_calendar_date(2027, Month::January, 1)?, 0.05),
    ///     (maturity, 0.06),
    /// ];
    ///
    /// let schedule = CashFlowSchedule::builder()
    ///     .principal(Money::new(1_000_000.0, Currency::USD), issue, maturity)
    ///     .fixed_stepup(
    ///         &steps,
    ///         ScheduleParams::quarterly_act360(),
    ///         CouponType::Cash,
    ///     )
    ///     .build_with_curves(None)?;
    ///
    /// assert!(schedule.flows.len() > 0);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Notes
    ///
    /// - Steps must be ordered by end date
    /// - If the last step doesn't reach maturity, the last rate is extended
    /// - All windows use the same schedule parameters
    #[must_use = "builder methods should be chained or terminated with .build_with_curves(...)"]
    pub fn fixed_stepup(
        &mut self,
        steps: &[(Date, f64)],
        schedule: ScheduleParams,
        default_split: CouponType,
    ) -> &mut Self {
        let Some((issue, maturity)) = self.issue_maturity_or_record_error("fixed_stepup") else {
            return self;
        };
        let mut prev = issue;
        for &(end, rate) in steps {
            let _ = self.add_fixed_coupon_window(prev, end, rate, schedule.clone(), default_split);
            prev = end;
        }
        if prev != maturity {
            // If the last step didn't reach maturity, extend using last rate
            if let Some(&(_, rate)) = steps.last() {
                let _ = self.add_fixed_coupon_window(prev, maturity, rate, schedule, default_split);
            }
        }
        self
    }

    /// Convenience: floating margin step-up program.
    ///
    /// Creates a series of floating-rate coupon windows where the margin over
    /// the floating index changes at specified boundary dates. Common for loans
    /// where the credit spread increases over time.
    ///
    /// # Arguments
    ///
    /// * `steps` - Boundary dates and margins: `&[(end_date, margin_bps)]`
    /// * `base_params` - Base floating parameters (index, gearing, reset lag)
    /// * `schedule` - Common schedule parameters
    /// * `default_split` - Payment type for all windows
    ///
    /// # Example
    ///
    /// ```rust
    /// use finstack_core::currency::Currency;
    /// use finstack_core::dates::{Date, Tenor, DayCount, BusinessDayConvention, StubKind};
    /// use finstack_core::money::Money;
    /// use finstack_core::types::CurveId;
    /// use finstack_valuations::cashflow::builder::{
    ///     CashFlowSchedule, ScheduleParams, FloatCouponParams, CouponType
    /// };
    /// use rust_decimal_macros::dec;
    /// use time::Month;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let issue = Date::from_calendar_date(2025, Month::January, 1)?;
    /// let maturity = Date::from_calendar_date(2028, Month::January, 1)?;
    ///
    /// // Floating rate loan: SOFR + 200bps, stepping up to +300bps, then +400bps
    /// let steps = [
    ///     (Date::from_calendar_date(2026, Month::January, 1)?, 200.0),
    ///     (Date::from_calendar_date(2027, Month::January, 1)?, 300.0),
    ///     (maturity, 400.0),
    /// ];
    ///
    /// let base = FloatCouponParams {
    ///     index_id: CurveId::new("USD-SOFR"),
    ///     margin_bp: dec!(0),  // Will be overridden by steps
    ///     gearing: dec!(1),
    ///     reset_lag_days: 2,
    ///     gearing_includes_spread: true,
    ///     floor_bp: None,
    ///     cap_bp: None,
    ///     all_in_floor_bp: None,
    ///     index_cap_bp: None,
    ///     fixing_calendar_id: None,
    ///     overnight_compounding: None,
    ///     fallback: Default::default(),
    /// };
    ///
    /// let schedule = CashFlowSchedule::builder()
    ///     .principal(Money::new(5_000_000.0, Currency::USD), issue, maturity)
    ///     .float_margin_stepup(
    ///         &steps,
    ///         base,
    ///         ScheduleParams::quarterly_act360(),
    ///         CouponType::Cash,
    ///     )
    ///     .build_with_curves(None)?;
    ///
    /// assert!(schedule.flows.len() > 0);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use = "builder methods should be chained or terminated with .build_with_curves(...)"]
    pub fn float_margin_stepup(
        &mut self,
        steps: &[(Date, f64)],
        base_params: FloatCouponParams,
        schedule: ScheduleParams,
        default_split: CouponType,
    ) -> &mut Self {
        let Some((issue, maturity)) = self.issue_maturity_or_record_error("float_margin_stepup")
        else {
            return self;
        };
        let mut prev = issue;
        for &(end, margin_bp) in steps {
            let mut params = base_params.clone();
            // Convert f64 margin_bp to Decimal
            params.margin_bp = Decimal::try_from(margin_bp).unwrap_or(Decimal::ZERO);
            let _ = self.add_float_coupon_window(
                prev,
                end,
                params.clone(),
                schedule.clone(),
                default_split,
            );
            prev = end;
        }
        if prev != maturity {
            let mut params = base_params.clone();
            if let Some(&(_, margin_bp)) = steps.last() {
                // Convert f64 margin_bp to Decimal
                params.margin_bp = Decimal::try_from(margin_bp).unwrap_or(Decimal::ZERO);
            }
            let _ = self.add_float_coupon_window(prev, maturity, params, schedule, default_split);
        }
        self
    }

    /// Convenience: fixed-to-float switch at `switch` date.
    ///
    /// Creates a hybrid instrument that pays fixed coupons until a switch date,
    /// then converts to floating coupons. Common for convertible/callable bonds
    /// and structured products with changing payment profiles.
    ///
    /// # Arguments
    ///
    /// * `switch` - Date when coupon switches from fixed to floating
    /// * `fixed_win` - Fixed rate and schedule for pre-switch period
    /// * `float_win` - Floating parameters and schedule for post-switch period
    /// * `default_split` - Payment type for both periods
    ///
    /// # Example
    ///
    /// ```rust
    /// use finstack_core::currency::Currency;
    /// use finstack_core::dates::{Date, Tenor, DayCount, BusinessDayConvention, StubKind};
    /// use finstack_core::money::Money;
    /// use finstack_core::types::CurveId;
    /// use finstack_valuations::cashflow::builder::{
    ///     CashFlowSchedule, ScheduleParams, FixedWindow, FloatWindow,
    ///     FloatCouponParams, CouponType
    /// };
    /// use rust_decimal_macros::dec;
    /// use time::Month;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let issue = Date::from_calendar_date(2025, Month::January, 1)?;
    /// let switch = Date::from_calendar_date(2027, Month::January, 1)?;
    /// let maturity = Date::from_calendar_date(2030, Month::January, 1)?;
    ///
    /// // Pay 5% fixed for 2 years, then SOFR + 250bps floating
    /// let fixed_win = FixedWindow {
    ///     rate: dec!(0.05),
    ///     schedule: ScheduleParams::semiannual_30360(),
    /// };
    ///
    /// let float_win = FloatWindow {
    ///     params: FloatCouponParams {
    ///         index_id: CurveId::new("USD-SOFR"),
    ///         margin_bp: dec!(250),
    ///         gearing: dec!(1),
    ///         reset_lag_days: 2,
    ///         gearing_includes_spread: true,
    ///         floor_bp: None,
    ///         cap_bp: None,
    ///         all_in_floor_bp: None,
    ///         index_cap_bp: None,
    ///         fixing_calendar_id: None,
    ///         overnight_compounding: None,
    ///         fallback: Default::default(),
    ///     },
    ///     schedule: ScheduleParams::quarterly_act360(),
    /// };
    ///
    /// let schedule = CashFlowSchedule::builder()
    ///     .principal(Money::new(10_000_000.0, Currency::USD), issue, maturity)
    ///     .fixed_to_float(switch, fixed_win, float_win, CouponType::Cash)
    ///     .build_with_curves(None)?;
    ///
    /// assert!(schedule.flows.len() > 0);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use = "builder methods should be chained or terminated with .build_with_curves(...)"]
    pub fn fixed_to_float(
        &mut self,
        switch: Date,
        fixed_win: FixedWindow,
        float_win: FloatWindow,
        default_split: CouponType,
    ) -> &mut Self {
        let Some((issue, maturity)) = self.issue_maturity_or_record_error("fixed_to_float") else {
            return self;
        };
        // Convert Decimal rate to f64 for add_fixed_coupon_window
        let rate_f64 = fixed_win.rate.to_f64().unwrap_or(0.0);
        let _ = self.add_fixed_coupon_window(
            issue,
            switch,
            rate_f64,
            fixed_win.schedule,
            default_split,
        );
        let _ = self.add_float_coupon_window(
            switch,
            maturity,
            float_win.params,
            float_win.schedule,
            default_split,
        );
        self
    }

    /// Convenience: payment split program with boundary dates (PIK toggle windows).
    ///
    /// Creates a payment profile where the coupon payment type (Cash, PIK, or Split)
    /// changes over time. Common for PIK toggle bonds and mezzanine loans where
    /// the borrower can elect to capitalize interest during specific periods.
    ///
    /// # Arguments
    ///
    /// * `steps` - Boundary dates and payment splits: `&[(end_date, split)]`
    ///
    /// # Example
    ///
    /// ```rust
    /// use finstack_core::currency::Currency;
    /// use finstack_core::dates::{Date, Tenor, DayCount, BusinessDayConvention, StubKind};
    /// use finstack_core::money::Money;
    /// use finstack_valuations::cashflow::builder::{
    ///     CashFlowSchedule, FixedCouponSpec, CouponType
    /// };
    /// use rust_decimal_macros::dec;
    /// use time::Month;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let issue = Date::from_calendar_date(2025, Month::January, 1)?;
    /// let maturity = Date::from_calendar_date(2030, Month::January, 1)?;
    ///
    /// // PIK toggle: 100% PIK for first 2 years, 50/50 split for next 2 years, then all cash
    /// let payment_steps = [
    ///     (Date::from_calendar_date(2027, Month::January, 1)?, CouponType::PIK),
    ///     (Date::from_calendar_date(2029, Month::January, 1)?, CouponType::Split {
    ///         cash_pct: dec!(0.5),
    ///         pik_pct: dec!(0.5)
    ///     }),
    ///     (maturity, CouponType::Cash),
    /// ];
    ///
    /// let fixed_spec = FixedCouponSpec {
    ///     coupon_type: CouponType::Cash,  // Will be overridden by payment program
    ///     rate: dec!(0.10),  // 10% PIK toggle
    ///     freq: Tenor::semi_annual(),
    ///     dc: DayCount::Thirty360,
    ///     bdc: BusinessDayConvention::Following,
    ///     calendar_id: "weekends_only".to_string(),
    ///     end_of_month: false,
    ///     payment_lag_days: 0,
    ///     stub: StubKind::None,
    /// };
    ///
    /// let schedule = CashFlowSchedule::builder()
    ///     .principal(Money::new(25_000_000.0, Currency::USD), issue, maturity)
    ///     .fixed_cf(fixed_spec)
    ///     .payment_split_program(&payment_steps)
    ///     .build_with_curves(None)?;
    ///
    /// // Check that PIK flows increase outstanding balance
    /// let outstanding_path = schedule.outstanding_path_per_flow()?;
    /// assert!(outstanding_path.len() > 0);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Notes
    ///
    /// - Periods not covered by steps default to `Cash`
    /// - Steps must be ordered by end date
    /// - Works with both fixed and floating coupons
    #[must_use = "builder methods should be chained or terminated with .build_with_curves(...)"]
    pub fn payment_split_program(&mut self, steps: &[(Date, CouponType)]) -> &mut Self {
        let Some((issue, maturity)) = self.issue_maturity_or_record_error("payment_split_program")
        else {
            return self;
        };
        let mut prev = issue;
        for &(end, split) in steps {
            if prev < end {
                let _ = self.add_payment_window(prev, end, split);
            }
            prev = end;
        }
        if prev < maturity {
            let _ = self.add_payment_window(prev, maturity, CouponType::Cash);
        }
        self
    }

    /// Build the cashflow schedule with optional market curves for floating rate computation.
    ///
    /// When curves are provided, floating rate coupons include the forward rate:
    /// `coupon = outstanding * (forward_rate * gearing + margin_bp * 1e-4) * year_fraction`
    ///
    /// Build the cashflow schedule without market curves.
    ///
    /// Equivalent to `build_with_curves(None)`. For floating-rate instruments
    /// that require projection curves, use [`build_with_curves`](Self::build_with_curves).
    pub fn build(&self) -> finstack_core::Result<CashFlowSchedule> {
        self.build_with_curves(None)
    }

    /// Without curves, only the margin is used:
    /// `coupon = outstanding * (margin_bp * 1e-4 * gearing) * year_fraction`
    pub fn build_with_curves(
        &self,
        curves: Option<&finstack_core::market_data::context::MarketContext>,
    ) -> finstack_core::Result<CashFlowSchedule> {
        if let Some(err) = &self.pending_error {
            return Err(err.clone());
        }
        // 1) Validate core inputs
        let (notional, issue, maturity) = validate_core_inputs(self)?;

        // 2) Compile schedules and fees
        let (
            CompiledSchedules {
                fixed_schedules,
                float_schedules,
                used_fixed_specs,
                used_float_specs,
            },
            periodic_fees,
            fixed_fees,
        ) = compile_schedules_and_fees(self, issue, maturity)?;

        // 2b) Normalize principal events (sorted) and validate currency/date bounds
        let mut principal_events = self.principal_events.clone();
        principal_events.sort_by_key(|ev| ev.date);

        // Reject principal events with currency different from notional.
        let expected_ccy = notional.initial.currency();
        if let Some(ev) = principal_events
            .iter()
            .find(|ev| ev.delta.currency() != expected_ccy)
        {
            return Err(finstack_core::Error::CurrencyMismatch {
                expected: expected_ccy,
                actual: ev.delta.currency(),
            });
        }

        // Reject principal events after maturity (would create post-maturity flows
        // after outstanding has been zeroed out, leading to undefined behavior).
        if let Some(ev) = principal_events.iter().find(|ev| ev.date > maturity) {
            return Err(InputError::DateOutOfRange {
                date: ev.date,
                range: (issue, maturity),
            }
            .into());
        }

        // 3) Collect all relevant dates
        let date_inputs = DateCollectionInputs {
            issue,
            maturity,
            fixed_schedules: &fixed_schedules,
            float_schedules: &float_schedules,
            periodic_fees: &periodic_fees,
            fixed_fees: &fixed_fees,
            notional: &notional,
            principal_events: &principal_events,
        };
        let dates = collect_all_dates(&date_inputs)?;

        // 4) Derive amortization setup
        let amort_setup = derive_amortization_setup(&notional, &fixed_schedules, &float_schedules)?;

        // 5) Initialize fold state and build context (processing issue-date principal events)
        let mut state = initialize_build_state(issue, &notional, dates.len(), &principal_events);
        let ccy = notional.initial.currency();
        let ctx = BuildContext {
            ccy,
            maturity,
            notional: &notional,
            fixed_schedules: &fixed_schedules,
            float_schedules: &float_schedules,
            periodic_fees: &periodic_fees,
            fixed_fees: &fixed_fees,
            principal_events: &principal_events,
        };

        // Resolve curves upfront and reuse across all payment dates.
        let resolved_curves: Vec<Option<Arc<ForwardCurve>>> = if let Some(mkt) = curves {
            float_schedules
                .iter()
                .map(|(spec, _, _)| mkt.get_forward(spec.rate_spec.index_id.as_str()).ok())
                .collect()
        } else {
            vec![None; float_schedules.len()]
        };

        // 6) Fold over dates producing flows deterministically
        for &d in dates.iter().skip(1) {
            state = process_one_date(d, state, &ctx, &amort_setup, &resolved_curves)?;
        }

        // 7) Finalize flows and produce meta/day count (use actual specs used)
        let (flows, meta, out_dc) =
            finalize_flows(state.flows, &used_fixed_specs, &used_float_specs);
        Ok(CashFlowSchedule {
            flows,
            notional,
            day_count: out_dc,
            meta,
        })
    }
}
