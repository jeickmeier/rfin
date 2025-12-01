//! Cashflow builder API and build orchestration.
//!
//! This module contains the main `CashflowBuilder` API that accumulates principal,
//! amortization, coupon windows, and fees, then orchestrates the compilation into
//! a deterministic `CashFlowSchedule`.
//!
//! ## Responsibilities
//!
//! - `CashflowBuilder` struct and its public builder methods
//! - Build orchestration (validation, compilation, date collection, state management)
//! - Pipeline stages: validate inputs, compile schedules, initialize state, process dates
//! - Amortization setup and parameter derivation
//! - Integration with emission, compiler, and date generation modules
//!
//! Quick start
//! -----------
//! ```rust
//! use finstack_core::currency::Currency;
//! use finstack_core::dates::{Date, Frequency, DayCount, BusinessDayConvention};
//! use finstack_core::dates::StubKind;
//! use finstack_core::money::Money;
//! use finstack_valuations::cashflow::builder::{CashFlowSchedule, FixedCouponSpec, CouponType};
//! use time::Month;
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//!
//! let issue = Date::from_calendar_date(2025, Month::January, 15).expect("valid date");
//! let maturity = Date::from_calendar_date(2026, Month::January, 15).expect("valid date");
//! let mut b = CashFlowSchedule::builder();
//! b.principal(Money::new(1_000.0, Currency::USD), issue, maturity)
//!  .fixed_cf(FixedCouponSpec{
//!      coupon_type: CouponType::Cash,
//!      rate: 0.05,
//!      freq: Frequency::semi_annual(),
//!      dc: DayCount::Act365F,
//!      bdc: BusinessDayConvention::Following,
//!      calendar_id: None,
//!      stub: StubKind::None,
//!  });
//! let schedule = b.build()
//!     .map_err(|e| format!("Failed to build cashflow schedule: {}", e))?;
//! assert!(!schedule.flows.is_empty());
//! # Ok(())
//! # }
//! ```

use super::schedule::{finalize_flows, CashFlowSchedule};
use crate::cashflow::builder::{AmortizationSpec, Notional};
use crate::cashflow::primitives::{CFKind, CashFlow};
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::error::InputError;
use finstack_core::market_data::term_structures::ForwardCurve;
use finstack_core::money::Money;
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
    FloatingCouponSpec, ScheduleParams,
};
use smallvec::SmallVec;

// -------------------------------------------------------------------------
// Pipeline scaffolding — pure-ish stages and fold state
// -------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct BuildState {
    flows: Vec<CashFlow>,
    outstanding_after: hashbrown::HashMap<Date, f64>,
    outstanding: f64,
}

#[derive(Debug, Clone)]
struct AmortizationSetup {
    amort_dates: hashbrown::HashSet<Date>,
    step_remaining_map: Option<hashbrown::HashMap<Date, Money>>, // for StepRemaining
    linear_delta: Option<f64>,                                   // for LinearTo
    percent_per: Option<f64>,                                    // for PercentPerPeriod
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
}

fn validate_core_inputs(b: &CashflowBuilder) -> finstack_core::Result<(Notional, Date, Date)> {
    let notional = b
        .notional
        .clone()
        .ok_or_else(|| finstack_core::Error::from(InputError::Invalid))?;
    let issue = b
        .issue
        .ok_or_else(|| finstack_core::Error::from(InputError::Invalid))?;
    let maturity = b
        .maturity
        .ok_or_else(|| finstack_core::Error::from(InputError::Invalid))?;
    Ok((notional, issue, maturity))
}

type CompiledAndFees = (CompiledSchedules, Vec<PeriodicFee>, Vec<(Date, Money)>);

fn compile_schedules_and_fees(
    b: &CashflowBuilder,
    issue: Date,
    maturity: Date,
    strict: bool,
) -> finstack_core::Result<CompiledAndFees> {
    let compiled = compute_coupon_schedules(b, issue, maturity, strict)?;
    let (periodic_fees, fixed_fees) = build_fee_schedules(issue, maturity, &b.fees, strict)?;
    Ok((compiled, periodic_fees, fixed_fees))
}

fn derive_amortization_setup(
    notional: &Notional,
    fixed_schedules: &[FixedSchedule],
    float_schedules: &[FloatSchedule],
) -> finstack_core::Result<AmortizationSetup> {
    // Determine base cadence schedule for linear/percent amortization
    let amort_base_schedule: Option<Vec<Date>> = if matches!(
        notional.amort,
        AmortizationSpec::LinearTo { .. } | AmortizationSpec::PercentPerPeriod { .. }
    ) {
        if let Some((_, ds, _, _)) = fixed_schedules.first() {
            Some(ds.clone())
        } else if let Some((_, ds, _)) = float_schedules.first() {
            Some(ds.clone())
        } else {
            None
        }
    } else {
        None
    };

    if amort_base_schedule.is_none()
        && matches!(
            notional.amort,
            AmortizationSpec::LinearTo { .. } | AmortizationSpec::PercentPerPeriod { .. }
        )
    {
        return Err(InputError::Invalid.into());
    }

    // Precompute helpers depending on amort spec
    let step_remaining_map: Option<hashbrown::HashMap<Date, Money>> = match &notional.amort {
        AmortizationSpec::StepRemaining { schedule } => {
            let mut m = hashbrown::HashMap::with_capacity(schedule.len());
            for (d, mny) in schedule {
                m.insert(*d, *mny);
            }
            Some(m)
        }
        _ => None,
    };

    let (linear_delta, percent_per) = match &notional.amort {
        AmortizationSpec::LinearTo { final_notional } => {
            let base = amort_base_schedule.as_ref().ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                    id: "amortization_base_schedule".to_string(),
                })
            })?;
            let steps = (base.len() - 1) as f64;
            (
                Some(((notional.initial.amount() - final_notional.amount()) / steps).max(0.0)),
                None,
            )
        }
        AmortizationSpec::PercentPerPeriod { pct } => {
            (None, Some((notional.initial.amount() * *pct).max(0.0)))
        }
        _ => (None, None),
    };

    let amort_dates: hashbrown::HashSet<Date> = amort_base_schedule
        .as_ref()
        .map(|v| v.iter().copied().skip(1).collect())
        .unwrap_or_default();

    Ok(AmortizationSetup {
        amort_dates,
        step_remaining_map,
        linear_delta,
        percent_per,
    })
}

fn initialize_build_state(issue: Date, notional: &Notional, estimated_dates: usize) -> BuildState {
    // Pre-allocate flows: estimate 2-3 flows per date (coupon + potential amort + fee)
    let estimated_flows = estimated_dates * 3;
    let mut flows: Vec<CashFlow> = Vec::with_capacity(estimated_flows);
    flows.push(CashFlow {
        date: issue,
        reset_date: None,
        amount: notional.initial * -1.0,
        kind: CFKind::Notional,
        accrual_factor: 0.0,
        rate: None,
    });

    // Pre-allocate outstanding_after based on number of dates
    let mut outstanding_after: hashbrown::HashMap<Date, f64> =
        hashbrown::HashMap::with_capacity(estimated_dates);
    outstanding_after.insert(issue, notional.initial.amount());

    BuildState {
        flows,
        outstanding_after,
        outstanding: notional.initial.amount(),
    }
}

fn collect_all_dates(
    issue: Date,
    maturity: Date,
    fixed_schedules: &[FixedSchedule],
    float_schedules: &[FloatSchedule],
    periodic_fees: &[PeriodicFee],
    fixed_fees: &[(Date, Money)],
    notional: &Notional,
) -> finstack_core::Result<Vec<Date>> {
    let periodic_date_slices: Vec<&[Date]> =
        periodic_fees.iter().map(|pf| pf.dates.as_slice()).collect();
    let dates: Vec<Date> = collect_dates(
        issue,
        maturity,
        fixed_schedules,
        float_schedules,
        &periodic_date_slices,
        fixed_fees,
        notional,
    );
    if dates.len() < 2 {
        return Err(InputError::TooFewPoints.into());
    }
    Ok(dates)
}

fn process_one_date(
    d: Date,
    mut state: BuildState,
    ctx: &BuildContext,
    amort_setup: &AmortizationSetup,
    curves: Option<&finstack_core::market_data::context::MarketContext>,
    resolved_curves: &[Option<Arc<ForwardCurve>>],
) -> finstack_core::Result<BuildState> {
    // Coupons
    let (pik_f, mut fixed_flows) = emit_fixed_coupons_on(
        d,
        ctx.fixed_schedules,
        &state.outstanding_after,
        state.outstanding,
        ctx.ccy,
    )?;
    let (pik_fl, mut float_flows) = emit_float_coupons_on(
        d,
        ctx.float_schedules,
        &state.outstanding_after,
        state.outstanding,
        ctx.ccy,
        curves,
        resolved_curves,
    )?;
    let pik_to_add = pik_f + pik_fl;
    state.flows.append(&mut fixed_flows);
    state.flows.append(&mut float_flows);

    // Amortization
    let amort_params = AmortizationParams {
        ccy: ctx.ccy,
        amort_dates: &amort_setup.amort_dates,
        linear_delta: amort_setup.linear_delta,
        percent_per: amort_setup.percent_per,
        step_remaining_map: &amort_setup.step_remaining_map,
    };
    let mut amort_flows = emit_amortization_on(
        d,
        ctx.notional,
        &mut state.outstanding,
        &amort_params,
        d == ctx.maturity,
    )?;
    state.flows.append(&mut amort_flows);

    // PIK capitalization
    if pik_to_add > 0.0 {
        state.outstanding += pik_to_add;
    }

    // Fees
    let mut fee_flows = emit_fees_on(
        d,
        ctx.periodic_fees,
        ctx.fixed_fees,
        state.outstanding,
        ctx.ccy,
    )?;
    state.flows.append(&mut fee_flows);

    // Redemption at maturity
    if d == ctx.maturity && state.outstanding > 0.0 {
        state.flows.push(CashFlow {
            date: d,
            reset_date: None,
            amount: Money::new(state.outstanding, ctx.ccy),
            kind: CFKind::Notional,
            accrual_factor: 0.0,
            rate: None,
        });
        state.outstanding = 0.0;
    }

    // Record outstanding for this date
    state.outstanding_after.insert(d, state.outstanding);

    Ok(state)
}

// -------------------------------------------------------------------------
// Segmented coupon program primitives (references from compile.rs)
// -------------------------------------------------------------------------

/// Builder for constructing cashflow schedules with validation.
///
/// Provides a fluent API for building complex cashflow schedules with
/// proper validation and business day adjustments.
#[derive(Debug, Default, Clone)]
pub struct CashflowBuilder {
    notional: Option<Notional>,
    issue: Option<Date>,
    maturity: Option<Date>,
    fees: SmallVec<[FeeSpec; 4]>,
    // Segmented programs (optional): coupon program and payment/PIK program
    pub(super) coupon_program: Vec<CouponProgramPiece>,
    pub(super) payment_program: Vec<PaymentProgramPiece>,
    /// When true, schedule generation errors (e.g., unknown calendar) are propagated
    /// instead of falling back to unadjusted schedules. Default: false (graceful).
    schedule_strict: bool,
}

impl CashflowBuilder {
    /// Creates a new composable cashflow builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Helper: Validate that issue and maturity are set.
    ///
    /// Returns an error if either issue or maturity is not set. This is used by
    /// builder methods that require the instrument horizon to be established.
    fn ensure_dates_set(&self) -> finstack_core::Result<()> {
        if self.issue.is_none() || self.maturity.is_none() {
            return Err(InputError::Invalid.into());
        }
        Ok(())
    }

    /// Helper: Get issue and maturity with clear panic message if not set.
    ///
    /// Builder methods that add coupon windows require issue/maturity to be set first
    /// via `principal()`. This helper provides a single validation point with clear
    /// error messages indicating which method was called.
    fn get_issue_maturity(&self, method_name: &str) -> (Date, Date) {
        let issue = self.issue.unwrap_or_else(|| {
            panic!(
                "CashflowBuilder::{}: issue date must be set via principal() before calling this method",
                method_name
            )
        });
        let maturity = self.maturity.unwrap_or_else(|| {
            panic!(
                "CashflowBuilder::{}: maturity date must be set via principal() before calling this method",
                method_name
            )
        });
        (issue, maturity)
    }
    /// Sets principal details and instrument horizon.
    pub fn principal(&mut self, initial: Money, issue_date: Date, maturity: Date) -> &mut Self {
        self.notional = Some(Notional {
            initial,
            amort: AmortizationSpec::None,
        });
        self.issue = Some(issue_date);
        self.maturity = Some(maturity);
        self
    }

    /// Convenience helper to set principal by amount and currency.
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
    pub fn amortization(&mut self, spec: AmortizationSpec) -> &mut Self {
        if let Some(n) = &mut self.notional {
            n.amort = spec;
        }
        self
    }

    /// Enable strict schedule generation (propagate errors instead of graceful fallback).
    ///
    /// When strict mode is enabled, schedule generation will return errors if:
    /// - A specified calendar is not found
    /// - Schedule building fails for any reason
    ///
    /// Default is graceful mode (strict = false), which falls back to unadjusted
    /// schedules when calendar lookup or adjustment fails.
    ///
    /// # Example
    /// ```ignore
    /// let schedule = CashFlowSchedule::builder()
    ///     .principal(Money::new(1_000_000.0, Currency::USD), issue, maturity)
    ///     .strict_schedules(true)  // Error on unknown calendar
    ///     .fixed_cf(spec)
    ///     .build()?;
    /// ```
    pub fn strict_schedules(&mut self, strict: bool) -> &mut Self {
        self.schedule_strict = strict;
        self
    }

    /// Adds a fixed coupon specification.
    pub fn fixed_cf(&mut self, spec: FixedCouponSpec) -> &mut Self {
        let (issue, maturity) = self.get_issue_maturity("fixed_cf");
        self.coupon_program.push(CouponProgramPiece {
            window: DateWindow {
                start: issue,
                end: maturity,
            },
            schedule: ScheduleParams {
                freq: spec.freq,
                dc: spec.dc,
                bdc: spec.bdc,
                calendar_id: spec.calendar_id,
                stub: spec.stub,
            },
            coupon: CouponSpec::Fixed { rate: spec.rate },
        });
        self.payment_program.push(PaymentProgramPiece {
            window: DateWindow {
                start: issue,
                end: maturity,
            },
            split: spec.coupon_type,
        });
        self
    }

    /// Non-panicking variant of `fixed_cf`. Returns error if principal not set.
    ///
    /// Preferred for library code to avoid panics in production.
    pub fn try_fixed_cf(&mut self, spec: FixedCouponSpec) -> finstack_core::Result<&mut Self> {
        self.ensure_dates_set()?;
        Ok(self.fixed_cf(spec))
    }

    /// Adds a floating coupon specification.
    pub fn floating_cf(&mut self, spec: FloatingCouponSpec) -> &mut Self {
        let (issue, maturity) = self.get_issue_maturity("floating_cf");
        self.coupon_program.push(CouponProgramPiece {
            window: DateWindow {
                start: issue,
                end: maturity,
            },
            schedule: ScheduleParams {
                freq: spec.freq,
                dc: spec.rate_spec.dc,
                bdc: spec.rate_spec.bdc,
                calendar_id: spec.rate_spec.calendar_id.clone(),
                stub: spec.stub,
            },
            coupon: CouponSpec::Float {
                index_id: spec.rate_spec.index_id,
                margin_bp: spec.rate_spec.spread_bp,
                gearing: spec.rate_spec.gearing,
                reset_lag_days: spec.rate_spec.reset_lag_days,
            },
        });
        self.payment_program.push(PaymentProgramPiece {
            window: DateWindow {
                start: issue,
                end: maturity,
            },
            split: spec.coupon_type,
        });
        self
    }

    /// Non-panicking variant of `floating_cf`. Returns error if principal not set.
    ///
    /// Preferred for library code to avoid panics in production.
    pub fn try_floating_cf(
        &mut self,
        spec: FloatingCouponSpec,
    ) -> finstack_core::Result<&mut Self> {
        self.ensure_dates_set()?;
        Ok(self.floating_cf(spec))
    }

    /// Adds a fee specification.
    pub fn fee(&mut self, spec: FeeSpec) -> &mut Self {
        self.fees.push(spec);
        self
    }

    /// Adds a fixed coupon window with its own schedule and payment split (cash/PIK/split).
    pub fn add_fixed_coupon_window(
        &mut self,
        start: Date,
        end: Date,
        rate: f64,
        schedule: ScheduleParams,
        split: CouponType,
    ) -> &mut Self {
        self.coupon_program.push(CouponProgramPiece {
            window: DateWindow { start, end },
            schedule,
            coupon: CouponSpec::Fixed { rate },
        });
        self.payment_program.push(PaymentProgramPiece {
            window: DateWindow { start, end },
            split,
        });
        self
    }

    /// Adds a floating coupon window with its own schedule and payment split.
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
            },
        });
        self.payment_program.push(PaymentProgramPiece {
            window: DateWindow { start, end },
            split,
        });
        self
    }

    /// Adds/overrides a payment split (cash/PIK/split) over a window (PIK toggle support).
    pub fn add_payment_window(&mut self, start: Date, end: Date, split: CouponType) -> &mut Self {
        self.payment_program.push(PaymentProgramPiece {
            window: DateWindow { start, end },
            split,
        });
        self
    }

    /// Non-panicking variant of `fixed_stepup`. Returns error if principal not set.
    ///
    /// Preferred for library code to avoid panics in production.
    pub fn try_fixed_stepup(
        &mut self,
        steps: &[(Date, f64)],
        schedule: ScheduleParams,
        default_split: CouponType,
    ) -> finstack_core::Result<&mut Self> {
        self.ensure_dates_set()?;
        Ok(self.fixed_stepup(steps, schedule, default_split))
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
    /// use finstack_core::dates::{Date, Frequency, DayCount, BusinessDayConvention, StubKind};
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
    ///     .build()?;
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
    pub fn fixed_stepup(
        &mut self,
        steps: &[(Date, f64)],
        schedule: ScheduleParams,
        default_split: CouponType,
    ) -> &mut Self {
        let (issue, maturity) = self.get_issue_maturity("fixed_stepup");
        let mut prev = issue;
        for &(end, rate) in steps {
            self.add_fixed_coupon_window(prev, end, rate, schedule.clone(), default_split);
            prev = end;
        }
        if prev != maturity {
            // If the last step didn't reach maturity, extend using last rate
            if let Some(&(_, rate)) = steps.last() {
                self.add_fixed_coupon_window(prev, maturity, rate, schedule, default_split);
            }
        }
        self
    }

    /// Non-panicking variant of `float_margin_stepup`. Returns error if principal not set.
    ///
    /// Preferred for library code to avoid panics in production.
    pub fn try_float_margin_stepup(
        &mut self,
        steps: &[(Date, f64)],
        base_params: FloatCouponParams,
        schedule: ScheduleParams,
        default_split: CouponType,
    ) -> finstack_core::Result<&mut Self> {
        self.ensure_dates_set()?;
        Ok(self.float_margin_stepup(steps, base_params, schedule, default_split))
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
    /// use finstack_core::dates::{Date, Frequency, DayCount, BusinessDayConvention, StubKind};
    /// use finstack_core::money::Money;
    /// use finstack_core::types::CurveId;
    /// use finstack_valuations::cashflow::builder::{
    ///     CashFlowSchedule, ScheduleParams, FloatCouponParams, CouponType
    /// };
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
    ///     margin_bp: 0.0,  // Will be overridden by steps
    ///     gearing: 1.0,
    ///     reset_lag_days: 2,
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
    ///     .build()?;
    ///
    /// assert!(schedule.flows.len() > 0);
    /// # Ok(())
    /// # }
    /// ```
    pub fn float_margin_stepup(
        &mut self,
        steps: &[(Date, f64)],
        base_params: FloatCouponParams,
        schedule: ScheduleParams,
        default_split: CouponType,
    ) -> &mut Self {
        let (issue, maturity) = self.get_issue_maturity("float_margin_stepup");
        let mut prev = issue;
        for &(end, margin_bp) in steps {
            let mut params = base_params.clone();
            params.margin_bp = margin_bp;
            self.add_float_coupon_window(
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
                params.margin_bp = margin_bp;
            }
            self.add_float_coupon_window(prev, maturity, params, schedule, default_split);
        }
        self
    }

    /// Non-panicking variant of `fixed_to_float`. Returns error if principal not set.
    ///
    /// Preferred for library code to avoid panics in production.
    pub fn try_fixed_to_float(
        &mut self,
        switch: Date,
        fixed_win: FixedWindow,
        float_win: FloatWindow,
        default_split: CouponType,
    ) -> finstack_core::Result<&mut Self> {
        self.ensure_dates_set()?;
        Ok(self.fixed_to_float(switch, fixed_win, float_win, default_split))
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
    /// use finstack_core::dates::{Date, Frequency, DayCount, BusinessDayConvention, StubKind};
    /// use finstack_core::money::Money;
    /// use finstack_core::types::CurveId;
    /// use finstack_valuations::cashflow::builder::{
    ///     CashFlowSchedule, ScheduleParams, FixedWindow, FloatWindow,
    ///     FloatCouponParams, CouponType
    /// };
    /// use time::Month;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let issue = Date::from_calendar_date(2025, Month::January, 1)?;
    /// let switch = Date::from_calendar_date(2027, Month::January, 1)?;
    /// let maturity = Date::from_calendar_date(2030, Month::January, 1)?;
    ///
    /// // Pay 5% fixed for 2 years, then SOFR + 250bps floating
    /// let fixed_win = FixedWindow {
    ///     rate: 0.05,
    ///     schedule: ScheduleParams::semiannual_30360(),
    /// };
    ///
    /// let float_win = FloatWindow {
    ///     params: FloatCouponParams {
    ///         index_id: CurveId::new("USD-SOFR"),
    ///         margin_bp: 250.0,
    ///         gearing: 1.0,
    ///         reset_lag_days: 2,
    ///     },
    ///     schedule: ScheduleParams::quarterly_act360(),
    /// };
    ///
    /// let schedule = CashFlowSchedule::builder()
    ///     .principal(Money::new(10_000_000.0, Currency::USD), issue, maturity)
    ///     .fixed_to_float(switch, fixed_win, float_win, CouponType::Cash)
    ///     .build()?;
    ///
    /// assert!(schedule.flows.len() > 0);
    /// # Ok(())
    /// # }
    /// ```
    pub fn fixed_to_float(
        &mut self,
        switch: Date,
        fixed_win: FixedWindow,
        float_win: FloatWindow,
        default_split: CouponType,
    ) -> &mut Self {
        let (issue, maturity) = self.get_issue_maturity("fixed_to_float");
        self.add_fixed_coupon_window(
            issue,
            switch,
            fixed_win.rate,
            fixed_win.schedule,
            default_split,
        );
        self.add_float_coupon_window(
            switch,
            maturity,
            float_win.params,
            float_win.schedule,
            default_split,
        );
        self
    }

    /// Non-panicking variant of `payment_split_program`. Returns error if principal not set.
    ///
    /// Preferred for library code to avoid panics in production.
    pub fn try_payment_split_program(
        &mut self,
        steps: &[(Date, CouponType)],
    ) -> finstack_core::Result<&mut Self> {
        self.ensure_dates_set()?;
        Ok(self.payment_split_program(steps))
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
    /// use finstack_core::dates::{Date, Frequency, DayCount, BusinessDayConvention, StubKind};
    /// use finstack_core::money::Money;
    /// use finstack_valuations::cashflow::builder::{
    ///     CashFlowSchedule, FixedCouponSpec, CouponType
    /// };
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
    ///         cash_pct: 0.5,
    ///         pik_pct: 0.5
    ///     }),
    ///     (maturity, CouponType::Cash),
    /// ];
    ///
    /// let fixed_spec = FixedCouponSpec {
    ///     coupon_type: CouponType::Cash,  // Will be overridden by payment program
    ///     rate: 0.10,  // 10% PIK toggle
    ///     freq: Frequency::semi_annual(),
    ///     dc: DayCount::Thirty360,
    ///     bdc: BusinessDayConvention::Following,
    ///     calendar_id: None,
    ///     stub: StubKind::None,
    /// };
    ///
    /// let schedule = CashFlowSchedule::builder()
    ///     .principal(Money::new(25_000_000.0, Currency::USD), issue, maturity)
    ///     .fixed_cf(fixed_spec)
    ///     .payment_split_program(&payment_steps)
    ///     .build()?;
    ///
    /// // Check that PIK flows increase outstanding balance
    /// let outstanding_path = schedule.outstanding_path()?;
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
    pub fn payment_split_program(&mut self, steps: &[(Date, CouponType)]) -> &mut Self {
        let (issue, maturity) = self.get_issue_maturity("payment_split_program");
        let mut prev = issue;
        for &(end, split) in steps {
            if prev < end {
                self.add_payment_window(prev, end, split);
            }
            prev = end;
        }
        if prev < maturity {
            self.add_payment_window(prev, maturity, CouponType::Cash);
        }
        self
    }

    /// Builds the complete cashflow schedule.
    pub fn build(&self) -> finstack_core::Result<CashFlowSchedule> {
        self.build_with_curves(None)
    }

    /// Build the cashflow schedule with optional market curves for floating rate computation.
    ///
    /// When curves are provided, floating rate coupons include the forward rate:
    /// `coupon = outstanding * (forward_rate * gearing + margin_bp * 1e-4) * year_fraction`
    ///
    /// Without curves, only the margin is used:
    /// `coupon = outstanding * (margin_bp * 1e-4 * gearing) * year_fraction`
    pub fn build_with_curves(
        &self,
        curves: Option<&finstack_core::market_data::context::MarketContext>,
    ) -> finstack_core::Result<CashFlowSchedule> {
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
        ) = compile_schedules_and_fees(self, issue, maturity, self.schedule_strict)?;

        // 3) Collect all relevant dates
        let dates = collect_all_dates(
            issue,
            maturity,
            &fixed_schedules,
            &float_schedules,
            &periodic_fees,
            &fixed_fees,
            &notional,
        )?;

        // 4) Derive amortization setup
        let amort_setup = derive_amortization_setup(&notional, &fixed_schedules, &float_schedules)?;

        // 5) Initialize fold state and build context
        let mut state = initialize_build_state(issue, &notional, dates.len());
        let ccy = notional.initial.currency();
        let ctx = BuildContext {
            ccy,
            maturity,
            notional: &notional,
            fixed_schedules: &fixed_schedules,
            float_schedules: &float_schedules,
            periodic_fees: &periodic_fees,
            fixed_fees: &fixed_fees,
        };

        // Resolve curves upfront
        let resolved_curves: Vec<Option<Arc<ForwardCurve>>> = if let Some(mkt) = curves {
            float_schedules
                .iter()
                .map(|(spec, _, _)| {
                    mkt.get_forward_ref(spec.rate_spec.index_id.as_str())
                        .ok()
                        .map(|curve| Arc::new(curve.clone()))
                })
                .collect()
        } else {
            vec![None; float_schedules.len()]
        };

        // 6) Fold over dates producing flows deterministically
        for &d in dates.iter().skip(1) {
            state = process_one_date(d, state, &ctx, &amort_setup, curves, &resolved_curves)?;
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
