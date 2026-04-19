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
//! use finstack_cashflows::builder::{CashFlowSchedule, FixedCouponSpec, CouponType};
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
use crate::builder::{AmortizationSpec, Notional};
use crate::primitives::{CFKind, CashFlow};
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
    CouponType, FeeSpec, FixedCouponSpec, FixedWindow, FloatingCouponSpec, ScheduleParams,
    StepUpCouponSpec,
};
use smallvec::SmallVec;
use tracing::debug;

mod build;
mod coupons;
mod fees;
mod principal;
mod splits;
mod stepup;

// -------------------------------------------------------------------------
// Pipeline scaffolding — pure-ish stages and fold state
// -------------------------------------------------------------------------

/// Internal state accumulated during schedule building.
#[derive(Debug, Clone)]
struct BuildState {
    flows: Vec<CashFlow>,
    outstanding_after: finstack_core::HashMap<Date, f64>,
    /// Outstanding balance tracked as `Decimal` for accounting-grade precision.
    ///
    /// Using `Decimal` eliminates f64 accumulation drift that can exceed 1 bp
    /// relative error on very long-dated instruments with many small cashflows
    /// (e.g., 600+ period amortizers). Converted to f64 only at API boundaries
    /// when passing to emission functions that operate in f64 space.
    outstanding: Decimal,
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

/// Convert an f64 to Decimal, returning `Decimal::ZERO` for non-finite values.
///
/// This is used for converting `Money::amount()` (always finite for valid Money)
/// into Decimal for outstanding balance tracking. The fallback to ZERO is a
/// defensive guard — valid Money values will always convert successfully.
fn f64_to_decimal_saturating(value: f64) -> Decimal {
    if !value.is_finite() {
        return Decimal::ZERO;
    }
    Decimal::try_from(value).unwrap_or(Decimal::ZERO)
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

    // Start with initial notional amount — use Decimal for precision
    let mut outstanding = f64_to_decimal_saturating(notional.initial.amount());

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
            outstanding += f64_to_decimal_saturating(ev.delta.amount());
        }
    }

    // Pre-allocate outstanding_after based on number of dates.
    // Convert Decimal outstanding to f64 for the history map (used by coupon/fee emission).
    let outstanding_f64 = outstanding.to_f64().unwrap_or(0.0);
    let mut outstanding_after: finstack_core::HashMap<Date, f64> =
        finstack_core::HashMap::default();
    outstanding_after.reserve(estimated_dates);
    outstanding_after.insert(issue, outstanding_f64);

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
    ///
    /// Converts Decimal outstanding to f64 at the emission boundary.
    fn emit_coupons(&self, d: Date, state: &mut BuildState) -> finstack_core::Result<f64> {
        let outstanding_f64 = state.outstanding.to_f64().unwrap_or(0.0);
        let pik_f = emit_fixed_coupons_on(
            d,
            self.ctx.fixed_schedules,
            &state.outstanding_after,
            outstanding_f64,
            self.ctx.ccy,
            &mut state.flows,
        )?;
        let pik_fl = emit_float_coupons_on(
            d,
            self.ctx.float_schedules,
            &state.outstanding_after,
            outstanding_f64,
            self.ctx.ccy,
            self.resolved_curves,
            &mut state.flows,
        )?;
        Ok(pik_f + pik_fl)
    }

    /// Emit amortization flows based on the amortization spec.
    ///
    /// Bridges the Decimal outstanding to the f64-based emission function by
    /// passing a temporary f64, then applying the delta back to the Decimal
    /// outstanding for precision-preserving accumulation.
    fn emit_amortization(&self, d: Date, state: &mut BuildState) -> finstack_core::Result<()> {
        let amort_params = AmortizationParams {
            ccy: self.ctx.ccy,
            amort_dates: &self.amort_setup.amort_dates,
            linear_delta: self.amort_setup.linear_delta,
            percent_per: self.amort_setup.percent_per,
            step_remaining_map: &self.amort_setup.step_remaining_map,
        };
        // Snapshot f64 outstanding before emission, then compute delta
        let before = state.outstanding.to_f64().unwrap_or(0.0);
        let mut outstanding_f64 = before;
        emit_amortization_on(
            d,
            self.ctx.notional,
            &mut outstanding_f64,
            &amort_params,
            d == self.ctx.maturity,
            &mut state.flows,
        )?;
        // Apply the amortization delta to the Decimal outstanding
        let delta = outstanding_f64 - before;
        if delta != 0.0 {
            state.outstanding += f64_to_decimal_saturating(delta);
        }
        Ok(())
    }

    /// Emit fee flows (periodic and fixed).
    ///
    /// Converts Decimal outstanding to f64 at the emission boundary.
    fn emit_fees(&self, d: Date, state: &mut BuildState) -> finstack_core::Result<()> {
        let outstanding_f64 = state.outstanding.to_f64().unwrap_or(0.0);
        emit_fees_on(
            d,
            self.ctx.periodic_fees,
            self.ctx.fixed_fees,
            outstanding_f64,
            &state.outstanding_after,
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
                state.outstanding += f64_to_decimal_saturating(ev.delta.amount());
            }
        }
    }

    /// Handle maturity redemption: emit final principal repayment if outstanding > 0.
    fn handle_maturity(&self, d: Date, state: &mut BuildState) {
        if d == self.ctx.maturity && state.outstanding > Decimal::ZERO {
            let outstanding_f64 = state.outstanding.to_f64().unwrap_or(0.0);
            state.flows.push(CashFlow {
                date: d,
                reset_date: None,
                amount: Money::new(outstanding_f64, self.ctx.ccy),
                kind: CFKind::Notional,
                accrual_factor: 0.0,
                rate: None,
            });
            state.outstanding = Decimal::ZERO;
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
            state.outstanding += f64_to_decimal_saturating(pik_to_add);
        }

        // 4. Fees
        self.emit_fees(d, &mut state)?;

        // 5. Principal events
        self.process_principal_events(d, &mut state);

        // 6. Maturity handling
        self.handle_maturity(d, &mut state);

        // Record outstanding for this date (convert Decimal to f64 for history map)
        let outstanding_f64 = state.outstanding.to_f64().unwrap_or(0.0);
        state.outstanding_after.insert(d, outstanding_f64);

        Ok(state)
    }
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
    fn record_pending_error(&mut self, error: finstack_core::Error) {
        if self.pending_error.is_none() {
            self.pending_error = Some(error);
        }
    }

    fn decimal_from_f64_or_record_error(
        &mut self,
        method_name: &str,
        field_name: &str,
        value: f64,
    ) -> Option<Decimal> {
        match Decimal::try_from(value) {
            Ok(decimal) => Some(decimal),
            Err(_) => {
                self.record_pending_error(finstack_core::Error::Validation(format!(
                    "CashFlowBuilder::{method_name} could not convert {field_name}={value} to Decimal"
                )));
                None
            }
        }
    }

    fn f64_from_decimal_or_record_error(
        &mut self,
        method_name: &str,
        field_name: &str,
        value: Decimal,
    ) -> Option<f64> {
        match value.to_f64() {
            Some(float) => Some(float),
            None => {
                self.record_pending_error(finstack_core::Error::Validation(format!(
                    "CashFlowBuilder::{method_name} could not convert {field_name}={value} to f64"
                )));
                None
            }
        }
    }

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

    fn push_coupon_window(
        &mut self,
        start: Date,
        end: Date,
        schedule: ScheduleParams,
        coupon: CouponSpec,
        split: CouponType,
    ) -> &mut Self {
        self.coupon_program.push(CouponProgramPiece {
            window: DateWindow { start, end },
            schedule,
            coupon,
        });
        self.payment_program.push(PaymentProgramPiece {
            window: DateWindow { start, end },
            split,
        });
        self
    }

    fn push_full_horizon_coupon(
        &mut self,
        method_name: &str,
        schedule: ScheduleParams,
        coupon: CouponSpec,
        split: CouponType,
    ) -> &mut Self {
        let Some((issue, maturity)) = self.issue_maturity_or_record_error(method_name) else {
            return self;
        };
        self.push_coupon_window(issue, maturity, schedule, coupon, split)
    }

    fn schedule_from_floating_spec(spec: &FloatingCouponSpec) -> ScheduleParams {
        ScheduleParams {
            freq: spec.freq,
            dc: spec.rate_spec.dc,
            bdc: spec.rate_spec.bdc,
            calendar_id: spec.rate_spec.calendar_id.clone(),
            stub: spec.stub,
            end_of_month: spec.rate_spec.end_of_month,
            payment_lag_days: spec.rate_spec.payment_lag_days,
        }
    }

    fn floating_spec_with_margin(
        spec: &FloatingCouponSpec,
        spread_bp: Decimal,
    ) -> FloatingCouponSpec {
        let mut next = spec.clone();
        next.rate_spec.spread_bp = spread_bp;
        next
    }
}

/// Immutable, curve-independent pre-computation for a cashflow schedule.
///
/// Produced by [`CashFlowBuilder::prepared`]. Holds the compiled schedules,
/// collected payment dates, amortization setup, and validated principal
/// events — everything needed to materialize a [`CashFlowSchedule`] except
/// for the floating-rate projection, which depends on market curves.
///
/// This is the canonical artifact for **repeated repricing of the same
/// instrument** under different market states: construct once, call
/// [`project`](Self::project) as many times as needed.
///
/// # Thread safety
///
/// `PreparedCashFlow` is `Send + Sync`-ready and designed to be shared
/// behind an `Arc` if concurrent projection across threads is needed.
#[derive(Debug, Clone)]
pub struct PreparedCashFlow {
    notional: Notional,
    issue: Date,
    maturity: Date,
    fixed_schedules: Vec<FixedSchedule>,
    float_schedules: Vec<FloatSchedule>,
    periodic_fees: Vec<PeriodicFee>,
    fixed_fees: Vec<(Date, Money)>,
    principal_events: Vec<PrincipalEvent>,
    dates: Vec<Date>,
    amort_setup: AmortizationSetup,
}
