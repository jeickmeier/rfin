//! Cashflow schedule compiler: transforms high-level programs into concrete schedules.
//!
//! This module takes the cashflow builder inputs (fixed/floating coupon specs,
//! optional coupon/payment programs, fees, and notional behavior) and produces
//! deterministic, validated schedules that downstream cashflow generation uses.
//!
//! Responsibilities:
//! - Partition the `[issue, maturity]` horizon into windows using coupon/payment
//!   program boundaries.
//! - For each window, derive date schedules according to frequency, stub, BDC,
//!   calendar, and day‑count conventions.
//! - Materialize per‑window coupon specs (fixed or floating) and selected
//!   payment split (Cash | PIK | Split) with precise coverage semantics.
//! - Compile fee specifications into periodic and fixed fee schedules.
//! - Collect a stable, de‑duplicated set of relevant dates (issue, maturity,
//!   coupon/payment/fee dates, and custom amortization dates).
//!
//! Determinism and validation follow the project invariants: windows must be
//! within `[issue, maturity]`, coverage must be unique (no overlapping coupon
//! pieces without containment), and every produced schedule must contain at
//! least two dates.

use crate::builder::{AmortizationSpec, Notional};
use finstack_core::dates::{
    BusinessDayConvention, Date, DayCount, HolidayCalendar, StubKind, Tenor,
};
use finstack_core::money::Money;
use finstack_core::InputError;
use rust_decimal::Decimal;

use super::calendar::resolve_calendar_strict;
use super::date_generation::{build_dates, index_period_schedule, SchedulePeriod};
use super::rate_helpers::ResolvedFloatingRateSpec;
use super::specs::{
    CouponType, FeeAccrualBasis, FeeBase, FeeSpec, FixedCouponSpec, FloatingCouponSpec,
    FloatingRateSpec, ScheduleParams,
};

/// Result type for schedule building with metadata.
type ScheduleWithMeta = (
    Vec<Date>,
    finstack_core::HashMap<Date, SchedulePeriod>,
    finstack_core::HashSet<Date>,
);

/// Build dates and metadata using the date_generation module.
///
/// This helper wraps `date_generation::build_dates` / `build_dates` and
/// extracts the `prev` map and `first_or_last` set required by the cashflow compiler.
#[allow(clippy::too_many_arguments)]
fn build_dates_with_meta(
    start: Date,
    end: Date,
    freq: Tenor,
    stub: StubKind,
    bdc: BusinessDayConvention,
    end_of_month: bool,
    payment_lag_days: i32,
    calendar_id: &str,
) -> finstack_core::Result<ScheduleWithMeta> {
    let schedule = build_dates(
        start,
        end,
        freq,
        stub,
        bdc,
        end_of_month,
        payment_lag_days,
        calendar_id,
    )?;
    Ok(index_period_schedule(schedule))
}

/// Compiled fixed-coupon schedule produced by [`compute_coupon_schedules`].
#[derive(Clone)]
pub(crate) struct FixedSchedule {
    pub(crate) spec: FixedCouponSpec,
    pub(crate) calendar: &'static dyn HolidayCalendar,
    pub(crate) dates: Vec<Date>,
    pub(crate) prev: finstack_core::HashMap<Date, SchedulePeriod>,
    pub(crate) first_last: finstack_core::HashSet<Date>,
}

/// Compiled floating-coupon schedule produced by [`compute_coupon_schedules`].
#[derive(Clone)]
pub(crate) struct FloatSchedule {
    pub(crate) spec: FloatingCouponSpec,
    pub(crate) calendar: &'static dyn HolidayCalendar,
    pub(crate) fixing_calendar: &'static dyn HolidayCalendar,
    pub(crate) runtime_spec: ResolvedFloatingRateSpec,
    pub(crate) dates: Vec<Date>,
    pub(crate) prev: finstack_core::HashMap<Date, SchedulePeriod>,
}

/// Periodic fee schedule prepared from fee specs.
///
/// Represents a normalized, per‑period fee defined by a base, annualized bps,
/// a day‑count, and the concrete schedule over which it accrues.
///
/// Fields:
/// - `base` (`FeeBase`): Notional/cash‑based fee base.
/// - `bps` (`Decimal`): Annualized basis points applied to the base. Uses Decimal for exact representation.
/// - `dc` (`DayCount`): Day‑count convention for accrual.
/// - `freq` (`Tenor`): Payment frequency (needed for Act/Act ISMA day count context).
/// - `calendar` (`HolidayCalendar`): Resolved calendar used for Bus/252 and Act/Act ISMA contexts.
/// - `dates` (`Vec<Date>`): Inclusive/exclusive boundary dates for accrual periods.
/// - `prev` (`HashMap<Date, SchedulePeriod>`): Period details keyed by payment date.
#[derive(Clone)]
pub(super) struct PeriodicFee {
    pub(super) base: FeeBase,
    pub(super) bps: Decimal,
    pub(super) dc: DayCount,
    pub(super) freq: Tenor,
    pub(super) calendar: &'static dyn HolidayCalendar,
    pub(super) dates: Vec<Date>,
    pub(super) prev: finstack_core::HashMap<Date, SchedulePeriod>,
    pub(super) accrual_basis: FeeAccrualBasis,
}

/// Convenience alias for a list of compiled periodic-fee schedules.
pub(super) type PeriodicFees = Vec<PeriodicFee>;
/// Convenience alias for a list of one-off `(date, amount)` fixed fees.
pub(super) type FixedFees = Vec<(Date, Money)>;

pub(super) fn build_fee_schedules(
    issue: Date,
    maturity: Date,
    fees: &[FeeSpec],
) -> finstack_core::Result<(PeriodicFees, FixedFees)> {
    //! Build periodic and fixed fee schedules from input `FeeSpec`s.
    //!
    //! Arguments:
    //! - `issue` (`Date`): Instrument start date (inclusive).
    //! - `maturity` (`Date`): Instrument end date (inclusive horizon endpoint).
    //! - `fees` (`&[FeeSpec]`): Collection of fee specifications to compile.
    //!
    //! Returns:
    //! - `Ok((PeriodicFees, FixedFees))` where `PeriodicFees` contains
    //!   normalized periodic fee programs (bps/day‑count/schedule), and
    //!   `FixedFees` contains explicit (`Date`, `Money`) pairs.
    //!
    //! Errors:
    //! - `InputError::TooFewPoints` if any derived schedule contains no dates.
    //!
    //! Example:
    //! ```rust
    //! use finstack_core::dates::{Date, DayCount, Tenor, BusinessDayConvention};
    //! use finstack_cashflows::builder::{FeeSpec, FeeBase};
    //! use finstack_core::dates::StubKind;
    //! use rust_decimal_macros::dec;
    //! use time::Month;
    //!
    //! let issue = Date::from_calendar_date(2024, Month::January, 1).expect("valid date");
    //! let maturity = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");
    //! let fees = vec![
    //!     FeeSpec::PeriodicBps {
    //!         base: FeeBase::Drawn,
    //!         bps: dec!(50),
    //!         freq: Tenor::quarterly(),
    //!         dc: DayCount::Act360,
    //!         bdc: BusinessDayConvention::Following,
    //!         calendar_id: "weekends_only".to_string(),
    //!         stub: StubKind::None,
    //!         accrual_basis: Default::default(),
    //!     }
    //! ];
    //! // Note: build_fee_schedules would be called here
    //! ```
    let mut periodic_fees: PeriodicFees = Vec::new();
    let mut fixed_fees: FixedFees = Vec::new();
    for fee in fees {
        match fee {
            FeeSpec::Fixed { date, amount } => {
                if amount.amount() != 0.0 {
                    fixed_fees.push((*date, *amount))
                }
            }
            FeeSpec::PeriodicBps {
                base,
                bps,
                freq,
                dc,
                bdc,
                calendar_id,
                stub,
                accrual_basis,
            } => {
                let (dates, prev, _) = build_dates_with_meta(
                    issue,
                    maturity,
                    *freq,
                    *stub,
                    *bdc,
                    false,
                    0,
                    calendar_id,
                )?;
                if dates.is_empty() {
                    return Err(InputError::TooFewPoints.into());
                }
                let calendar = resolve_calendar_strict(calendar_id)?;
                periodic_fees.push(PeriodicFee {
                    base: base.clone(),
                    bps: *bps,
                    dc: *dc,
                    freq: *freq,
                    calendar,
                    dates,
                    prev,
                    accrual_basis: accrual_basis.clone(),
                });
            }
        }
    }
    Ok((periodic_fees, fixed_fees))
}

#[derive(Debug, Clone, Copy)]
pub(super) struct DateWindow {
    pub(super) start: Date,
    pub(super) end: Date, // exclusive
}

#[derive(Debug, Clone)]
pub(super) enum CouponSpec {
    Fixed {
        rate: Decimal,
    },
    Float {
        rate_spec: FloatingRateSpec,
    },
    StepUp {
        initial_rate: Decimal,
        step_schedule: Vec<(finstack_core::dates::Date, Decimal)>,
    },
}

#[derive(Debug, Clone)]
pub(super) struct CouponProgramPiece {
    pub(super) window: DateWindow,
    pub(super) schedule: ScheduleParams,
    pub(super) coupon: CouponSpec,
}

#[derive(Debug, Clone)]
pub(super) struct PaymentProgramPiece {
    pub(super) window: DateWindow,
    pub(super) split: CouponType, // Cash | PIK | Split
}

#[derive(Clone)]
pub(super) struct CompiledSchedules {
    pub(super) fixed_schedules: Vec<FixedSchedule>,
    pub(super) float_schedules: Vec<FloatSchedule>,
}

pub(super) fn collect_dates(
    issue: Date,
    maturity: Date,
    fixed_schedules: &[FixedSchedule],
    float_schedules: &[FloatSchedule],
    periodic_fee_date_slices: &[&[Date]],
    fixed_fees: &[(Date, Money)],
    notional: &Notional,
) -> Vec<Date> {
    /// Collect a de‑duplicated, ordered set of all relevant dates.
    ///
    /// Aggregates `issue`, `maturity`, all coupon schedule dates (fixed and
    /// floating), fee dates (periodic slices and fixed fee dates), and any
    /// custom amortization dates from notional into a single ascending vector.
    ///
    /// Arguments:
    /// - `issue` (`Date`): Start date (included).
    /// - `maturity` (`Date`): End date (included in horizon; may or may not appear in schedules).
    /// - `fixed_schedules` (`&[FixedSchedule]`): Compiled fixed coupon schedules.
    /// - `float_schedules` (`&[FloatSchedule]`): Compiled floating coupon schedules.
    /// - `periodic_fee_date_slices` (`&[&[Date]]`): Borrowed slices of periodic fee dates.
    /// - `fixed_fees` (`&[(Date, Money)]`): Explicit fixed fees by date.
    /// - `notional` (`&Notional`): Notional specification including custom amortization.
    ///
    /// Returns: `Vec<Date>` sorted and de‑duplicated.
    use std::collections::BTreeSet;
    let mut set: BTreeSet<Date> = BTreeSet::new();
    set.insert(issue);
    set.insert(maturity);

    // Collect all fixed coupon dates (accrual boundaries + payment dates)
    for schedule in fixed_schedules {
        for period in schedule.prev.values() {
            set.insert(period.accrual_start);
            set.insert(period.accrual_end);
            set.insert(period.payment_date);
        }
    }

    // Collect all floating coupon dates (accrual boundaries + payment dates)
    for schedule in float_schedules {
        for period in schedule.prev.values() {
            set.insert(period.accrual_start);
            set.insert(period.accrual_end);
            set.insert(period.payment_date);
        }
    }

    // Collect all periodic fee dates (accrual boundaries + payment dates)
    for dates in periodic_fee_date_slices {
        set.extend(dates.iter().copied());
    }

    // Collect all fixed fee dates
    set.extend(fixed_fees.iter().map(|(d, _)| *d));

    // Collect amortization dates
    match &notional.amort {
        AmortizationSpec::CustomPrincipal { items } => {
            set.extend(items.iter().map(|(d, _)| *d));
        }
        AmortizationSpec::StepRemaining { schedule } => {
            set.extend(schedule.iter().map(|(d, _)| *d));
        }
        _ => {}
    }

    set.into_iter().collect()
}

struct StepUpCompileInput<'a> {
    split: CouponType,
    initial_rate: Decimal,
    step_schedule: &'a [(Date, Decimal)],
    schedule: &'a ScheduleParams,
    calendar: &'static dyn HolidayCalendar,
    dates: &'a [Date],
    prev: &'a finstack_core::HashMap<Date, SchedulePeriod>,
    first_last: &'a finstack_core::HashSet<Date>,
}

fn compile_step_up_schedules(input: StepUpCompileInput<'_>) -> Vec<FixedSchedule> {
    type RateGroup = (
        Decimal,
        Vec<Date>,
        finstack_core::HashMap<Date, SchedulePeriod>,
        finstack_core::HashSet<Date>,
    );

    let rate_for = |period_start: Date| -> Decimal {
        let mut rate = input.initial_rate;
        for (step_date, step_rate) in input.step_schedule {
            if *step_date <= period_start {
                rate = *step_rate;
            } else {
                break;
            }
        }
        rate
    };

    let mut rate_groups: Vec<RateGroup> = Vec::new();
    for &payment_date in input.dates {
        if let Some(period) = input.prev.get(&payment_date) {
            let period_rate = rate_for(period.accrual_start);
            let extend_last = rate_groups
                .last()
                .map(|(rate, _, _, _)| *rate == period_rate)
                .unwrap_or(false);

            if extend_last {
                if let Some(last) = rate_groups.last_mut() {
                    last.1.push(payment_date);
                    last.2.insert(payment_date, *period);
                    if input.first_last.contains(&payment_date) {
                        last.3.insert(payment_date);
                    }
                }
            } else {
                let mut period_map = finstack_core::HashMap::default();
                period_map.insert(payment_date, *period);
                let mut first_last = finstack_core::HashSet::default();
                if input.first_last.contains(&payment_date) {
                    first_last.insert(payment_date);
                }
                rate_groups.push((period_rate, vec![payment_date], period_map, first_last));
            }
        }
    }

    rate_groups
        .into_iter()
        .map(|(rate, dates, prev, first_last)| FixedSchedule {
            spec: FixedCouponSpec::from_parts(input.split, rate, input.schedule.clone()),
            calendar: input.calendar,
            dates,
            prev,
            first_last,
        })
        .collect()
}

pub(super) fn compute_coupon_schedules(
    builder: &crate::builder::orchestrator::CashFlowBuilder,
    issue: Date,
    maturity: Date,
) -> finstack_core::Result<CompiledSchedules> {
    //! Compile coupon and payment programs into concrete date schedules.
    //!
    //! This function processes the programmatic windowing model to generate
    //! concrete schedules. Payment windows can sparsely override the split policy;
    //! missing windows default to `Cash`.
    //!
    //! Arguments:
    //! - `builder` (`&CashFlowBuilder`): Source of coupon/payment programs.
    //! - `issue` (`Date`): Start date (inclusive).
    //! - `maturity` (`Date`): End date (inclusive horizon endpoint).
    //!
    //! Returns:
    //! - `Ok(CompiledSchedules)` with per‑window fixed and floating schedules
    //!   and the exact specs used for each schedule.
    //!
    //! Errors:
    //! - `InputError::Invalid` for out‑of‑range windows or overlapping windows
    //!   without containment, or if coverage selection is ambiguous.
    //! - `InputError::TooFewPoints` if any derived schedule has no dates.
    //!
    //! Example:
    //! ```rust
    //! use finstack_core::dates::{Date, Tenor, DayCount, BusinessDayConvention};
    //! use finstack_cashflows::builder::{FixedCouponSpec, CouponType};
    //! use finstack_core::dates::StubKind;
    //! use rust_decimal_macros::dec;
    //! use time::Month;
    //!
    //! let issue = Date::from_calendar_date(2024, Month::January, 1).expect("valid date");
    //! let maturity = Date::from_calendar_date(2026, Month::January, 1).expect("valid date");
    //! // Note: CashFlowBuilder would be created here
    //! let fixed_spec = FixedCouponSpec {
    //!     coupon_type: CouponType::Cash,
    //!     rate: dec!(0.05),
    //!     freq: Tenor::semi_annual(),
    //!     dc: DayCount::Thirty360,
    //!     bdc: BusinessDayConvention::Following,
    //!     calendar_id: "usny".to_string(),
    //!     end_of_month: false,
    //!     payment_lag_days: 0,
    //!     stub: StubKind::None,
    //! };
    //! // Note: compute_coupon_schedules would be called here
    //! ```
    use std::collections::BTreeSet;

    let coupon_pieces: &[CouponProgramPiece] = &builder.coupon_program;

    // If there are no coupon pieces at all and no payment windows, return empty schedules
    if coupon_pieces.is_empty() && builder.payment_program.is_empty() {
        return Ok(CompiledSchedules {
            fixed_schedules: Vec::new(),
            float_schedules: Vec::new(),
        });
    }

    // Payment pieces (PIK toggles) — may be sparse; missing windows default to Cash
    let payment_pieces: &[PaymentProgramPiece] = &builder.payment_program;

    // Validate windows are within [issue, maturity] and build boundary grid
    let within =
        |w: &DateWindow| -> bool { w.start >= issue && w.end <= maturity && w.start < w.end };
    let mut bounds: BTreeSet<Date> = BTreeSet::new();
    bounds.insert(issue);
    bounds.insert(maturity);
    for p in coupon_pieces {
        if !within(&p.window) {
            return Err(InputError::Invalid.into());
        }
        bounds.insert(p.window.start);
        bounds.insert(p.window.end);
    }
    for p in payment_pieces {
        if !within(&p.window) {
            return Err(InputError::Invalid.into());
        }
        bounds.insert(p.window.start);
        bounds.insert(p.window.end);
    }
    let grid: Vec<Date> = bounds.into_iter().collect();
    if grid.len() < 2 {
        return Err(InputError::TooFewPoints.into());
    }

    let mut fixed_schedules: Vec<FixedSchedule> = Vec::new();
    let mut float_schedules: Vec<FloatSchedule> = Vec::new();

    for w in grid.windows(2) {
        let s = w[0];
        let e = w[1];
        if s >= e {
            continue;
        }

        // Select single covering coupon piece
        let mut chosen_coupon: Option<&CouponProgramPiece> = None;
        for p in coupon_pieces {
            if p.window.start <= s && e <= p.window.end {
                if chosen_coupon.is_some() {
                    return Err(InputError::Invalid.into());
                }
                chosen_coupon = Some(p);
            }
        }
        let chosen_coupon = chosen_coupon.ok_or(InputError::Invalid)?;

        // Select payment split. Allow nested override windows: prefer most specific covering window.
        // If two covering windows are neither nested (i.e., overlapping without containment), it's invalid.
        let mut chosen: Option<(&DateWindow, CouponType)> = None;
        for p in payment_pieces {
            if p.window.start <= s && e <= p.window.end {
                match chosen {
                    None => chosen = Some((&p.window, p.split)),
                    Some((win, _)) => {
                        let p_within_chosen =
                            p.window.start >= win.start && p.window.end <= win.end;
                        let chosen_within_p =
                            win.start >= p.window.start && win.end <= p.window.end;
                        if p_within_chosen {
                            chosen = Some((&p.window, p.split)); // prefer more specific
                        } else if chosen_within_p {
                            // keep current chosen
                        } else {
                            return Err(InputError::Invalid.into());
                        }
                    }
                }
            }
        }
        let split = chosen.map(|(_, sp)| sp).unwrap_or(CouponType::Cash);

        let (dates, prev, first_or_last) = build_dates_with_meta(
            s,
            e,
            chosen_coupon.schedule.freq,
            chosen_coupon.schedule.stub,
            chosen_coupon.schedule.bdc,
            chosen_coupon.schedule.end_of_month,
            chosen_coupon.schedule.payment_lag_days,
            &chosen_coupon.schedule.calendar_id,
        )?;
        if dates.is_empty() {
            return Err(InputError::TooFewPoints.into());
        }
        let calendar = resolve_calendar_strict(&chosen_coupon.schedule.calendar_id)?;

        match &chosen_coupon.coupon {
            CouponSpec::Fixed { rate } => {
                let spec =
                    FixedCouponSpec::from_parts(split, *rate, chosen_coupon.schedule.clone());
                fixed_schedules.push(FixedSchedule {
                    spec,
                    calendar,
                    dates: dates.clone(),
                    prev: prev.clone(),
                    first_last: first_or_last.clone(),
                });
            }
            CouponSpec::StepUp {
                initial_rate,
                step_schedule,
            } => {
                fixed_schedules.extend(compile_step_up_schedules(StepUpCompileInput {
                    split,
                    initial_rate: *initial_rate,
                    step_schedule,
                    schedule: &chosen_coupon.schedule,
                    calendar,
                    dates: &dates,
                    prev: &prev,
                    first_last: &first_or_last,
                }));
            }
            CouponSpec::Float { rate_spec } => {
                let spec = FloatingCouponSpec {
                    rate_spec: rate_spec.clone(),
                    coupon_type: split,
                    freq: chosen_coupon.schedule.freq,
                    stub: chosen_coupon.schedule.stub,
                };
                let runtime_spec = ResolvedFloatingRateSpec::try_from(&spec.rate_spec)?;
                let calendar = resolve_calendar_strict(&spec.rate_spec.calendar_id)?;
                let fixing_calendar_id = spec
                    .rate_spec
                    .fixing_calendar_id
                    .as_deref()
                    .unwrap_or(&spec.rate_spec.calendar_id);
                let fixing_calendar = resolve_calendar_strict(fixing_calendar_id)?;
                float_schedules.push(FloatSchedule {
                    spec,
                    calendar,
                    fixing_calendar,
                    runtime_spec,
                    dates,
                    prev,
                });
            }
        }
    }

    Ok(CompiledSchedules {
        fixed_schedules,
        float_schedules,
    })
}
