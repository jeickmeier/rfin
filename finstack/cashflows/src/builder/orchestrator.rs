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
use finstack_core::dates::Date;
use finstack_core::decimal::{decimal_to_f64, f64_to_decimal};
use finstack_core::market_data::term_structures::ForwardCurve;
use finstack_core::money::Money;
use finstack_core::InputError;
use rust_decimal::Decimal;
use std::sync::Arc;

use super::compiler::{
    build_fee_schedules, collect_dates, compute_coupon_schedules, CompiledSchedules,
    CouponProgramPiece, CouponSpec, DateWindow, FixedSchedule, FloatSchedule, PaymentProgramPiece,
    PeriodicFee,
};
use super::pipeline::{BuildContext, DateProcessor};
use super::specs::{
    CouponType, FeeSpec, FixedCouponSpec, FixedWindow, FloatingCouponSpec, ScheduleParams,
    StepUpCouponSpec,
};
use smallvec::SmallVec;
use tracing::debug;

/// Internal state accumulated during schedule building.
#[derive(Debug, Clone)]
pub(super) struct BuildState {
    pub(super) flows: Vec<CashFlow>,
    pub(super) outstanding_after: finstack_core::HashMap<Date, Decimal>,
    /// Outstanding balance tracked as `Decimal` for accounting-grade precision.
    ///
    /// Using `Decimal` eliminates f64 accumulation drift that can exceed 1 bp
    /// relative error on very long-dated instruments with many small cashflows
    /// (e.g., 600+ period amortizers). Converted to f64 only at API boundaries
    /// when passing to emission functions that operate in f64 space.
    pub(super) outstanding: Decimal,
}

/// Principal event applied during schedule build (draws/repays).
///
/// `delta` adjusts outstanding (positive increases, negative decreases).
/// `cash` represents the cash leg (e.g., net of OID/fees). If `delta` differs
/// from `cash`, the difference is interpreted as non-cash adjustments.
/// `kind` classifies the emitted cashflow. `CFKind::Amortization` emits a
/// positive principal-repayment cashflow; notional-like events emit as negative
/// borrower draw cashflows. In all cases, `delta` remains the source of truth
/// for outstanding balance movement.
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
pub(super) struct AmortizationSetup {
    pub(super) amort_dates: finstack_core::HashSet<Date>,
    pub(super) step_remaining_map: Option<finstack_core::HashMap<Date, Money>>, // for StepRemaining
    pub(super) linear_delta: Option<f64>,                                       // for LinearTo
    pub(super) percent_per: Option<f64>, // for PercentOfOriginalPerPeriod
}

/// Grouped inputs for collecting all relevant schedule dates.
#[derive(Clone, Copy)]
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
    // borrowing the first available coupon leg. For multi-leg instruments with
    // differing frequencies, amortization follows this first leg's cadence.
    let amort_base: Option<&[Date]> = match notional.amort {
        AmortizationSpec::LinearTo { .. } | AmortizationSpec::PercentOfOriginalPerPeriod { .. } => {
            if let Some((_, _, ds, _, _)) = fixed_schedules.first() {
                Some(ds.as_slice())
            } else if let Some((_, _, ds, _)) = float_schedules.first() {
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
) -> finstack_core::Result<BuildState> {
    let estimated_flows = estimated_dates * 3;
    let mut flows: Vec<CashFlow> = Vec::with_capacity(estimated_flows);

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

    let mut outstanding = f64_to_decimal(notional.initial.amount())?;

    for ev in principal_events.iter().filter(|ev| ev.date <= issue) {
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
            outstanding += f64_to_decimal(ev.delta.amount())?;
        }
    }

    let mut outstanding_after: finstack_core::HashMap<Date, Decimal> =
        finstack_core::HashMap::default();
    outstanding_after.reserve(estimated_dates);
    outstanding_after.insert(issue, outstanding);

    Ok(BuildState {
        flows,
        outstanding_after,
        outstanding,
    })
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
    for pf in inputs.periodic_fees {
        for period in pf.prev.values() {
            dates.push(period.accrual_start);
            dates.push(period.accrual_end);
        }
    }
    for ev in inputs.principal_events {
        dates.push(ev.date);
    }
    dates.sort_unstable();
    dates.dedup();
    if dates.len() < 2 {
        return Err(InputError::TooFewPoints.into());
    }
    Ok(dates)
}

/// Builder for constructing cashflow schedules with validation.
///
/// Provides a fluent API for building complex cashflow schedules with
/// proper validation and business day adjustments.
#[derive(Debug, Clone)]
pub struct CashFlowBuilder {
    pub(super) notional: Option<Notional>,
    pub(super) issue: Option<Date>,
    pub(super) maturity: Option<Date>,
    /// Fee specifications. SmallVec<4> avoids heap allocation for typical instruments
    /// with ≤4 fee specs (commitment fee, facility fee, usage fee, admin fee).
    pub(super) fees: SmallVec<[FeeSpec; 4]>,
    pub(super) principal_events: Vec<PrincipalEvent>,
    // Segmented programs (optional): coupon program and payment/PIK program
    pub(super) coupon_program: Vec<CouponProgramPiece>,
    pub(super) payment_program: Vec<PaymentProgramPiece>,
    // Sticky builder error for fluent APIs that cannot return Result.
    pub(super) pending_error: Option<finstack_core::Error>,
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

#[derive(Clone)]
struct CompiledCashFlowPlan {
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

impl CashFlowBuilder {
    /// Adds a fixed coupon specification.
    ///
    /// The coupon leg spans the full principal horizon set by
    /// [`principal`](Self::principal). The builder emits fixed, stub, cash,
    /// split, or PIK coupon flows according to the supplied spec and the
    /// schedule conventions inside it.
    ///
    /// # Arguments
    ///
    /// * `spec` - Fixed-rate coupon quote, payment split, and schedule
    ///   conventions.
    ///
    /// # Returns
    ///
    /// Mutable builder reference for fluent chaining.
    ///
    /// # Errors
    ///
    /// This method records a deferred error if principal dates have not been
    /// set. Schedule generation, day-count, calendar, and coupon-split errors
    /// are returned by [`build_with_curves`](Self::build_with_curves).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_cashflows::builder::{CashFlowSchedule, CouponType, FixedCouponSpec};
    /// use finstack_core::currency::Currency;
    /// use finstack_core::dates::{BusinessDayConvention, Date, DayCount, StubKind, Tenor};
    /// use finstack_core::money::Money;
    /// use rust_decimal_macros::dec;
    /// use time::Month;
    ///
    /// let issue = Date::from_calendar_date(2025, Month::January, 15).expect("valid date");
    /// let maturity = Date::from_calendar_date(2026, Month::January, 15).expect("valid date");
    /// let schedule = CashFlowSchedule::builder()
    ///     .principal(Money::new(1_000_000.0, Currency::USD), issue, maturity)
    ///     .fixed_cf(FixedCouponSpec {
    ///         coupon_type: CouponType::Cash,
    ///         rate: dec!(0.05),
    ///         freq: Tenor::semi_annual(),
    ///         dc: DayCount::Thirty360,
    ///         bdc: BusinessDayConvention::Following,
    ///         calendar_id: "weekends_only".to_string(),
    ///         stub: StubKind::None,
    ///         end_of_month: false,
    ///         payment_lag_days: 0,
    ///     })
    ///     .build_with_curves(None)
    ///     .expect("fixed schedule builds");
    ///
    /// assert!(!schedule.flows.is_empty());
    /// ```
    #[must_use = "builder methods should be chained or terminated with .build_with_curves(...)"]
    pub fn fixed_cf(&mut self, spec: FixedCouponSpec) -> &mut Self {
        self.push_full_horizon_coupon(
            "fixed_cf",
            spec.schedule_params(),
            CouponSpec::Fixed { rate: spec.rate },
            spec.coupon_type,
        )
    }

    /// Adds a floating coupon specification.
    ///
    /// The coupon leg spans the full principal horizon set by
    /// [`principal`](Self::principal). Floating-rate projection is deferred
    /// until [`build_with_curves`](Self::build_with_curves), where the forward
    /// curve or fallback policy is applied.
    ///
    /// # Arguments
    ///
    /// * `spec` - Floating-rate index, spread, caps/floors, payment split, and
    ///   schedule conventions.
    ///
    /// # Returns
    ///
    /// Mutable builder reference for fluent chaining.
    ///
    /// # Errors
    ///
    /// This method records a deferred error if principal dates have not been
    /// set. Floating spec validation, missing forward curves, calendar errors,
    /// and fallback-policy failures are returned by the terminal build or
    /// project step.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_cashflows::builder::{
    ///     CashFlowSchedule, CouponType, FloatingCouponSpec, FloatingRateFallback,
    ///     FloatingRateSpec,
    /// };
    /// use finstack_core::currency::Currency;
    /// use finstack_core::dates::{BusinessDayConvention, Date, DayCount, StubKind, Tenor};
    /// use finstack_core::money::Money;
    /// use finstack_core::types::CurveId;
    /// use rust_decimal_macros::dec;
    /// use time::Month;
    ///
    /// let issue = Date::from_calendar_date(2025, Month::January, 15).expect("valid date");
    /// let maturity = Date::from_calendar_date(2026, Month::January, 15).expect("valid date");
    /// let mut builder = CashFlowSchedule::builder();
    ///
    /// let _ = builder
    ///     .principal(Money::new(1_000_000.0, Currency::USD), issue, maturity)
    ///     .floating_cf(FloatingCouponSpec {
    ///         coupon_type: CouponType::Cash,
    ///         rate_spec: FloatingRateSpec {
    ///             index_id: CurveId::new("USD-SOFR-3M"),
    ///             spread_bp: dec!(200),
    ///             gearing: dec!(1),
    ///             gearing_includes_spread: true,
    ///             index_floor_bp: Some(dec!(0)),
    ///             all_in_floor_bp: None,
    ///             all_in_cap_bp: None,
    ///             index_cap_bp: None,
    ///             reset_freq: Tenor::quarterly(),
    ///             reset_lag_days: 2,
    ///             dc: DayCount::Act360,
    ///             bdc: BusinessDayConvention::ModifiedFollowing,
    ///             calendar_id: "weekends_only".to_string(),
    ///             fixing_calendar_id: None,
    ///             end_of_month: false,
    ///             payment_lag_days: 0,
    ///             overnight_compounding: None,
    ///             overnight_basis: None,
    ///             fallback: FloatingRateFallback::SpreadOnly,
    ///         },
    ///         freq: Tenor::quarterly(),
    ///         stub: StubKind::None,
    ///     });
    /// ```
    #[must_use = "builder methods should be chained or terminated with .build_with_curves(...)"]
    pub fn floating_cf(&mut self, spec: FloatingCouponSpec) -> &mut Self {
        self.push_full_horizon_coupon(
            "floating_cf",
            Self::schedule_from_floating_spec(&spec),
            CouponSpec::Float {
                rate_spec: spec.rate_spec,
            },
            spec.coupon_type,
        )
    }

    /// Adds a fixed coupon window with its own schedule and payment split (cash/PIK/split).
    ///
    /// Internal helper used by `fixed_stepup` / `fixed_to_float` etc. Prefer the
    /// spec-level entry points (`fixed_cf`, `fixed_stepup`, `fixed_to_float`).
    #[must_use = "builder methods should be chained or terminated with .build_with_curves(...)"]
    fn add_fixed_coupon_window(
        &mut self,
        start: Date,
        end: Date,
        rate: Decimal,
        schedule: ScheduleParams,
        split: CouponType,
    ) -> &mut Self {
        self.push_coupon_window(start, end, schedule, CouponSpec::Fixed { rate }, split)
    }

    /// Adds a floating coupon window with its own schedule and payment split.
    ///
    /// Internal helper used by `float_margin_stepup` / `fixed_to_float` etc.
    /// Prefer the spec-level entry points (`floating_cf`, `float_margin_stepup`,
    /// `fixed_to_float`).
    #[must_use = "builder methods should be chained or terminated with .build_with_curves(...)"]
    fn add_float_coupon_window(
        &mut self,
        start: Date,
        end: Date,
        spec: FloatingCouponSpec,
    ) -> &mut Self {
        self.push_coupon_window(
            start,
            end,
            Self::schedule_from_floating_spec(&spec),
            CouponSpec::Float {
                rate_spec: spec.rate_spec,
            },
            spec.coupon_type,
        )
    }
}

impl CashFlowBuilder {
    /// Adds a fee specification.
    ///
    /// Fixed fees emit a one-time `Fee` cashflow on their configured date.
    /// Periodic basis-point fees generate a schedule over the principal horizon
    /// and accrue against the configured [`crate::builder::FeeBase`].
    ///
    /// # Arguments
    ///
    /// * `spec` - Fixed or periodic fee specification to add to the schedule.
    ///
    /// # Returns
    ///
    /// Mutable builder reference for fluent chaining.
    ///
    /// # Errors
    ///
    /// This method does not return errors directly. Missing principal dates,
    /// invalid fee schedules, calendar lookup failures, and currency mismatches
    /// are returned by [`build_with_curves`](Self::build_with_curves).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_cashflows::builder::{CashFlowSchedule, FeeBase, FeeSpec};
    /// use finstack_core::currency::Currency;
    /// use finstack_core::dates::{BusinessDayConvention, Date, DayCount, StubKind, Tenor};
    /// use finstack_core::money::Money;
    /// use rust_decimal_macros::dec;
    /// use time::Month;
    ///
    /// let issue = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");
    /// let maturity = Date::from_calendar_date(2026, Month::January, 1).expect("valid date");
    /// let mut builder = CashFlowSchedule::builder();
    ///
    /// let _ = builder
    ///     .principal(Money::new(1_000_000.0, Currency::USD), issue, maturity)
    ///     .fee(FeeSpec::PeriodicBps {
    ///         base: FeeBase::Drawn,
    ///         bps: dec!(25),
    ///         freq: Tenor::quarterly(),
    ///         dc: DayCount::Act360,
    ///         bdc: BusinessDayConvention::ModifiedFollowing,
    ///         calendar_id: "weekends_only".to_string(),
    ///         stub: StubKind::None,
    ///         accrual_basis: Default::default(),
    ///     });
    /// ```
    #[must_use = "builder methods should be chained or terminated with .build_with_curves(...)"]
    pub fn fee(&mut self, spec: FeeSpec) -> &mut Self {
        self.fees.push(spec);
        self
    }
}

impl CashFlowBuilder {
    /// Adds (or overrides) a payment split over a single date window.
    ///
    /// The lower-level primitive behind [`payment_split_program`](Self::payment_split_program).
    /// Pushes a single payment-program piece covering `[start, end)` and uses
    /// `split` as the coupon settlement type within that window. Subsequent
    /// calls add additional pieces; later windows take precedence on overlap
    /// during compilation.
    ///
    /// Prefer [`payment_split_program`](Self::payment_split_program) for
    /// PIK-toggle scheduling, which sequences windows from a single
    /// boundary-step list. Use this method only when you need to wire up
    /// non-contiguous or hand-crafted payment windows.
    ///
    /// # Arguments
    ///
    /// * `start` - Inclusive start of the payment window.
    /// * `end` - Exclusive end of the payment window.
    /// * `split` - Coupon settlement type (Cash / PIK / Split) for the window.
    ///
    /// # Returns
    ///
    /// Mutable builder reference for fluent chaining.
    #[must_use = "builder methods should be chained or terminated with .build_with_curves(...)"]
    pub fn add_payment_window(&mut self, start: Date, end: Date, split: CouponType) -> &mut Self {
        self.payment_program.push(PaymentProgramPiece {
            window: DateWindow { start, end },
            split,
        });
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
    /// use finstack_cashflows::builder::{
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
}

impl CashFlowBuilder {
    /// Adds a step-up coupon specification.
    ///
    /// A step-up coupon starts at an initial rate and steps to different rates
    /// on specified dates. The compiler translates this into per-period fixed
    /// coupon schedules with the appropriate rate for each period.
    ///
    /// # Arguments
    ///
    /// * `spec` - Step-up coupon definition containing the initial rate, step
    ///   schedule, payment split, and schedule conventions.
    ///
    /// # Returns
    ///
    /// Mutable builder reference for fluent chaining.
    ///
    /// # Errors
    ///
    /// This method records a deferred error if principal dates have not been
    /// set. Date generation, calendar lookup, coupon split validation, and
    /// day-count failures are returned by [`build_with_curves`](Self::build_with_curves).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_cashflows::builder::{CashFlowSchedule, CouponType, StepUpCouponSpec};
    /// use finstack_core::currency::Currency;
    /// use finstack_core::dates::{BusinessDayConvention, Date, DayCount, StubKind, Tenor};
    /// use finstack_core::money::Money;
    /// use rust_decimal_macros::dec;
    /// use time::Month;
    ///
    /// let issue = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");
    /// let step = Date::from_calendar_date(2026, Month::January, 1).expect("valid date");
    /// let maturity = Date::from_calendar_date(2027, Month::January, 1).expect("valid date");
    /// let mut builder = CashFlowSchedule::builder();
    ///
    /// let _ = builder
    ///     .principal(Money::new(1_000_000.0, Currency::USD), issue, maturity)
    ///     .step_up_cf(StepUpCouponSpec {
    ///         coupon_type: CouponType::Cash,
    ///         initial_rate: dec!(0.04),
    ///         step_schedule: vec![(step, dec!(0.05))],
    ///         freq: Tenor::semi_annual(),
    ///         dc: DayCount::Thirty360,
    ///         bdc: BusinessDayConvention::Following,
    ///         calendar_id: "weekends_only".to_string(),
    ///         stub: StubKind::None,
    ///         end_of_month: false,
    ///         payment_lag_days: 0,
    ///     });
    /// ```
    #[must_use = "builder methods should be chained or terminated with .build_with_curves(...)"]
    pub fn step_up_cf(&mut self, spec: StepUpCouponSpec) -> &mut Self {
        self.push_full_horizon_coupon(
            "step_up_cf",
            spec.schedule_params(),
            CouponSpec::StepUp {
                initial_rate: spec.initial_rate,
                step_schedule: spec.step_schedule,
            },
            spec.coupon_type,
        )
    }

    /// Convenience: fixed-rate step-up program with Decimal rates.
    ///
    /// Creates consecutive fixed coupon windows whose rate changes at the
    /// supplied boundary dates.
    #[must_use = "builder methods should be chained or terminated with .build_with_curves(...)"]
    pub fn fixed_stepup_decimal(
        &mut self,
        steps: &[(Date, Decimal)],
        schedule: ScheduleParams,
        default_split: CouponType,
    ) -> &mut Self {
        let Some((issue, maturity)) = self.issue_maturity_or_record_error("fixed_stepup_decimal")
        else {
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

    /// Convenience: floating margin step-up program with Decimal margins.
    ///
    /// Creates consecutive floating coupon windows whose margin over the
    /// floating index changes at the supplied boundary dates.
    #[must_use = "builder methods should be chained or terminated with .build_with_curves(...)"]
    pub fn float_margin_stepup_decimal(
        &mut self,
        steps: &[(Date, Decimal)],
        base_spec: FloatingCouponSpec,
    ) -> &mut Self {
        let Some((issue, maturity)) =
            self.issue_maturity_or_record_error("float_margin_stepup_decimal")
        else {
            return self;
        };
        let mut prev = issue;
        for &(end, margin_decimal) in steps {
            let window_spec = Self::floating_spec_with_margin(&base_spec, margin_decimal);
            let _ = self.add_float_coupon_window(prev, end, window_spec);
            prev = end;
        }
        if prev != maturity {
            let mut margin_decimal = base_spec.rate_spec.spread_bp;
            if let Some(&(_, last_margin_decimal)) = steps.last() {
                margin_decimal = last_margin_decimal;
            }
            let _ = self.add_float_coupon_window(
                prev,
                maturity,
                Self::floating_spec_with_margin(&base_spec, margin_decimal),
            );
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
    /// * `float_spec` - Canonical floating coupon spec for the post-switch period
    /// * `fixed_split` - Payment type for the fixed pre-switch period
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use finstack_core::currency::Currency;
    /// use finstack_core::dates::{Date, Tenor, DayCount, BusinessDayConvention, StubKind};
    /// use finstack_core::money::Money;
    /// use finstack_core::types::CurveId;
    /// use finstack_cashflows::builder::{
    ///     CashFlowSchedule, CouponType, FixedWindow, FloatingCouponSpec, FloatingRateSpec,
    ///     ScheduleParams,
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
    /// let float_spec = FloatingCouponSpec {
    ///     coupon_type: CouponType::Cash,
    ///     rate_spec: FloatingRateSpec {
    ///         index_id: CurveId::new("USD-SOFR"),
    ///         spread_bp: dec!(250),
    ///         gearing: dec!(1),
    ///         gearing_includes_spread: true,
    ///         index_floor_bp: None,
    ///         all_in_cap_bp: None,
    ///         all_in_floor_bp: None,
    ///         index_cap_bp: None,
    ///         reset_freq: Tenor::quarterly(),
    ///         reset_lag_days: 2,
    ///         dc: DayCount::Act360,
    ///         bdc: BusinessDayConvention::ModifiedFollowing,
    ///         calendar_id: "weekends_only".to_string(),
    ///         fixing_calendar_id: None,
    ///         end_of_month: false,
    ///         payment_lag_days: 0,
    ///         overnight_compounding: None,
    ///         overnight_basis: None,
    ///         fallback: Default::default(),
    ///     },
    ///     freq: Tenor::quarterly(),
    ///     stub: StubKind::ShortFront,
    /// };
    ///
    /// let schedule = CashFlowSchedule::builder()
    ///     .principal(Money::new(10_000_000.0, Currency::USD), issue, maturity)
    ///     .fixed_to_float(switch, fixed_win, float_spec, CouponType::Cash)
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
        float_spec: FloatingCouponSpec,
        fixed_split: CouponType,
    ) -> &mut Self {
        let Some((issue, maturity)) = self.issue_maturity_or_record_error("fixed_to_float") else {
            return self;
        };
        let _ = self.add_fixed_coupon_window(
            issue,
            switch,
            fixed_win.rate,
            fixed_win.schedule,
            fixed_split,
        );
        let _ = self.add_float_coupon_window(switch, maturity, float_spec);
        self
    }
}

impl CashFlowBuilder {
    /// Build the cashflow schedule with optional market curves for floating rate projection.
    ///
    /// When curves are provided, floating rate coupons use forward rates:
    /// `coupon = outstanding * (forward_rate * gearing + margin_bp * 1e-4) * year_fraction`
    ///
    /// Without curves, the fallback policy on each floating spec controls behavior
    /// (default: error; `SpreadOnly` uses just margin; `FixedRate(r)` uses a fixed index).
    ///
    pub fn build_with_curves(
        &self,
        curves: Option<&finstack_core::market_data::context::MarketContext>,
    ) -> finstack_core::Result<CashFlowSchedule> {
        self.compile_plan()?.project(curves)
    }

    fn compile_plan(&self) -> finstack_core::Result<CompiledCashFlowPlan> {
        if let Some(err) = &self.pending_error {
            return Err(err.clone());
        }
        let (notional, issue, maturity) = validate_core_inputs(self)?;

        let (
            CompiledSchedules {
                fixed_schedules,
                float_schedules,
            },
            periodic_fees,
            fixed_fees,
        ) = {
            let compiled = compute_coupon_schedules(self, issue, maturity)?;
            let (periodic_fees, fixed_fees) = build_fee_schedules(issue, maturity, &self.fees)?;
            (compiled, periodic_fees, fixed_fees)
        };

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

        if let Some(ev) = principal_events.iter().find(|ev| ev.date > maturity) {
            return Err(InputError::DateOutOfRange {
                date: ev.date,
                range: (issue, maturity),
            }
            .into());
        }

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
        debug!(dates = dates.len(), %issue, %maturity, "cashflow schedule: dates collected");

        let amort_setup = derive_amortization_setup(&notional, &fixed_schedules, &float_schedules)?;

        Ok(CompiledCashFlowPlan {
            notional,
            issue,
            maturity,
            fixed_schedules,
            float_schedules,
            periodic_fees,
            fixed_fees,
            principal_events,
            dates,
            amort_setup,
        })
    }
}

impl CompiledCashFlowPlan {
    fn project(
        &self,
        curves: Option<&finstack_core::market_data::context::MarketContext>,
    ) -> finstack_core::Result<CashFlowSchedule> {
        let mut state = initialize_build_state(
            self.issue,
            &self.notional,
            self.dates.len(),
            &self.principal_events,
        )?;
        let ccy = self.notional.initial.currency();
        for (fee_date, amount) in &self.fixed_fees {
            if *fee_date == self.issue && amount.amount() != 0.0 {
                state.flows.push(CashFlow {
                    date: *fee_date,
                    reset_date: None,
                    amount: *amount,
                    kind: CFKind::Fee,
                    accrual_factor: 0.0,
                    rate: None,
                });
            }
        }
        let ctx = BuildContext {
            ccy,
            maturity: self.maturity,
            notional: &self.notional,
            fixed_schedules: &self.fixed_schedules,
            float_schedules: &self.float_schedules,
            periodic_fees: &self.periodic_fees,
            fixed_fees: &self.fixed_fees,
            principal_events: &self.principal_events,
        };

        // Resolve curves upfront and reuse across all payment dates.
        let resolved_curves: Vec<Option<Arc<ForwardCurve>>> = if let Some(mkt) = curves {
            self.float_schedules
                .iter()
                .map(|(spec, _, _, _)| mkt.get_forward(spec.rate_spec.index_id.as_str()).ok())
                .collect()
        } else {
            vec![None; self.float_schedules.len()]
        };

        let processor = DateProcessor::new(&ctx, &self.amort_setup, &resolved_curves);
        for &d in self.dates.iter().skip(1) {
            state = processor.process(d, state)?;
        }

        // Warn on material residual principal without rejecting flexible structures.
        let threshold = Decimal::new(1, 4); // 1e-4 = 1 bp relative
        let initial_amount = self.notional.initial.amount();
        if initial_amount.abs() > 0.0 {
            let initial_dec = f64_to_decimal(initial_amount)?;
            if initial_dec != Decimal::ZERO {
                let abs_outstanding = if state.outstanding < Decimal::ZERO {
                    -state.outstanding
                } else {
                    state.outstanding
                };
                let abs_initial = if initial_dec < Decimal::ZERO {
                    -initial_dec
                } else {
                    initial_dec
                };
                let relative_residual = abs_outstanding / abs_initial;
                if relative_residual > threshold {
                    let final_outstanding = decimal_to_f64(state.outstanding)?;
                    let relative_residual_f64 = decimal_to_f64(relative_residual)?;
                    tracing::warn!(
                        initial = initial_amount,
                        final_outstanding,
                        relative_residual = relative_residual_f64,
                        threshold_bps = 1.0,
                        "cashflow schedule: final outstanding balance deviates from zero; \
                         check amortization schedule or instrument terminal flow"
                    );
                }
            }
        }

        let (flows, meta, out_dc) = finalize_flows(
            state.flows,
            &self.fixed_schedules,
            &self.float_schedules,
            Some(self.issue),
        );
        debug!(flows = flows.len(), "cashflow schedule: project complete");
        Ok(CashFlowSchedule {
            flows,
            notional: self.notional.clone(),
            day_count: out_dc,
            meta,
        })
    }
}
