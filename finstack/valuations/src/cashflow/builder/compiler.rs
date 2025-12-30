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

use crate::cashflow::builder::{AmortizationSpec, Notional};
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, StubKind, Tenor};
use finstack_core::money::Money;
use finstack_core::InputError;
use rust_decimal::Decimal;

use super::date_generation::{build_dates, build_dates_checked};
use super::specs::{
    CouponType, FeeBase, FeeSpec, FixedCouponSpec, FloatingCouponSpec, FloatingRateSpec,
    ScheduleParams,
};

/// Result type for schedule building with metadata.
type ScheduleWithMeta = (
    Vec<Date>,
    finstack_core::HashMap<Date, Date>,
    finstack_core::HashSet<Date>,
);

/// Build dates and metadata using the date_generation module.
///
/// This helper wraps `date_generation::build_dates` / `build_dates_checked` and
/// extracts the `prev` map and `first_or_last` set required by the cashflow compiler.
fn build_dates_with_meta(
    start: Date,
    end: Date,
    freq: Tenor,
    stub: StubKind,
    bdc: BusinessDayConvention,
    calendar_id: Option<&str>,
    strict: bool,
) -> finstack_core::Result<ScheduleWithMeta> {
    let sched = if strict {
        build_dates_checked(start, end, freq, stub, bdc, calendar_id)?
    } else {
        build_dates(start, end, freq, stub, bdc, calendar_id)
    };

    Ok((sched.dates, sched.prev, sched.first_or_last))
}

pub(super) type FixedSchedule = (
    FixedCouponSpec,
    Vec<Date>,
    finstack_core::HashMap<Date, Date>,
    finstack_core::HashSet<Date>,
);
pub(super) type FloatSchedule = (
    FloatingCouponSpec,
    Vec<Date>,
    finstack_core::HashMap<Date, Date>,
);

/// Periodic fee schedule prepared from fee specs.
///
/// Represents a normalized, per‑period fee defined by a base, annualized bps,
/// a day‑count, and the concrete schedule over which it accrues.
///
/// Fields:
/// - `base` (`FeeBase`): Notional/cash‑based fee base.
/// - `bps` (`Decimal`): Annualized basis points applied to the base. Uses Decimal for exact representation.
/// - `dc` (`DayCount`): Day‑count convention for accrual.
/// - `dates` (`Vec<Date>`): Inclusive/exclusive boundary dates for accrual periods.
/// - `prev` (`HashMap<Date, Date>`): Mapping of each period end to its start.
#[derive(Debug, Clone)]
pub(super) struct PeriodicFee {
    pub(super) base: FeeBase,
    pub(super) bps: Decimal,
    pub(super) dc: DayCount,
    pub(super) dates: Vec<Date>,
    pub(super) prev: finstack_core::HashMap<Date, Date>,
}

pub(super) type PeriodicFees = Vec<PeriodicFee>;
pub(super) type FixedFees = Vec<(Date, Money)>;

pub(super) fn build_fee_schedules(
    issue: Date,
    maturity: Date,
    fees: &[FeeSpec],
    strict: bool,
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
    //! - `InputError::TooFewPoints` if any derived schedule contains fewer than
    //!   two dates.
    //!
    //! Example:
    //! ```rust
    //! use finstack_core::dates::{Date, DayCount, Tenor, BusinessDayConvention};
    //! use finstack_valuations::cashflow::builder::{FeeSpec, FeeBase};
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
    //!         calendar_id: Some("usd".to_string()),
    //!         stub: StubKind::None,
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
            } => {
                let (dates, prev, _) = build_dates_with_meta(
                    issue,
                    maturity,
                    *freq,
                    *stub,
                    *bdc,
                    calendar_id.as_deref(),
                    strict,
                )?;
                if dates.len() < 2 {
                    return Err(InputError::TooFewPoints.into());
                }
                periodic_fees.push(PeriodicFee {
                    base: base.clone(),
                    bps: *bps,
                    dc: *dc,
                    dates,
                    prev,
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
        index_id: finstack_core::types::CurveId,
        margin_bp: Decimal,
        gearing: Decimal,
        reset_lag_days: i32,
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

#[derive(Debug, Clone)]
pub(super) struct CompiledSchedules {
    pub(super) fixed_schedules: Vec<FixedSchedule>,
    pub(super) float_schedules: Vec<FloatSchedule>,
    pub(super) used_fixed_specs: Vec<FixedCouponSpec>,
    pub(super) used_float_specs: Vec<FloatingCouponSpec>,
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

    // Collect all fixed coupon dates
    for (_, ds, _, _) in fixed_schedules {
        set.extend(ds.iter().copied());
    }

    // Collect all floating coupon dates
    for (_, ds, _) in float_schedules {
        set.extend(ds.iter().copied());
    }

    // Collect all periodic fee dates
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

pub(super) fn compute_coupon_schedules(
    builder: &crate::cashflow::builder::builder::CashFlowBuilder,
    issue: Date,
    maturity: Date,
    strict: bool,
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
    //! - `InputError::TooFewPoints` if any derived schedule has fewer than two dates.
    //!
    //! Example:
    //! ```rust
    //! use finstack_core::dates::{Date, Tenor, DayCount, BusinessDayConvention};
    //! use finstack_valuations::cashflow::builder::{FixedCouponSpec, CouponType};
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
    //!     calendar_id: Some("usny".to_string()),
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
            used_fixed_specs: Vec::new(),
            used_float_specs: Vec::new(),
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
    let mut used_fixed_specs: Vec<FixedCouponSpec> = Vec::new();
    let mut used_float_specs: Vec<FloatingCouponSpec> = Vec::new();

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
            chosen_coupon.schedule.calendar_id.as_deref(),
            strict,
        )?;
        if dates.len() < 2 {
            return Err(InputError::TooFewPoints.into());
        }

        match &chosen_coupon.coupon {
            CouponSpec::Fixed { rate } => {
                let spec = FixedCouponSpec {
                    coupon_type: split,
                    rate: *rate,
                    freq: chosen_coupon.schedule.freq,
                    dc: chosen_coupon.schedule.dc,
                    bdc: chosen_coupon.schedule.bdc,
                    calendar_id: chosen_coupon.schedule.calendar_id.clone(),
                    stub: chosen_coupon.schedule.stub,
                };
                used_fixed_specs.push(spec.clone());
                fixed_schedules.push((spec, dates.clone(), prev.clone(), first_or_last));
            }
            CouponSpec::Float {
                index_id,
                margin_bp,
                gearing,
                reset_lag_days,
            } => {
                let spec = FloatingCouponSpec {
                    rate_spec: FloatingRateSpec {
                        index_id: index_id.clone(),
                        spread_bp: *margin_bp,
                        gearing: *gearing,
                        gearing_includes_spread: true,
                        floor_bp: None,
                        cap_bp: None,
                        all_in_floor_bp: None,
                        index_cap_bp: None,
                        reset_freq: chosen_coupon.schedule.freq,
                        reset_lag_days: *reset_lag_days,
                        dc: chosen_coupon.schedule.dc,
                        bdc: chosen_coupon.schedule.bdc,
                        calendar_id: chosen_coupon.schedule.calendar_id.clone(),
                        fixing_calendar_id: None,
                    },
                    coupon_type: split,
                    freq: chosen_coupon.schedule.freq,
                    stub: chosen_coupon.schedule.stub,
                };
                used_float_specs.push(spec.clone());
                float_schedules.push((spec, dates, prev));
            }
        }
    }

    Ok(CompiledSchedules {
        fixed_schedules,
        float_schedules,
        used_fixed_specs,
        used_float_specs,
    })
}
