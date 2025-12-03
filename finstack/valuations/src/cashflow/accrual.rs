//! Generic schedule-driven interest accrual engine.
//!
//! This module provides reusable logic to compute accrued interest from a
//! canonical [`CashFlowSchedule`] only, without depending on instrument
//! specifications. Any instrument that can expose a `CashFlowSchedule`
//! (via `CashflowProvider::build_full_schedule` or otherwise) can use this
//! engine for consistent Linear / Compounded accrual, including:
//!
//! - Fixed, floating, fixed-to-float, and float-to-fixed coupons
//! - PIK and cash/PIK split coupons
//! - Amortization schedules and notional draws/repays
//! - Ex-coupon conventions
//! - Fully custom schedules built directly from the cashflow builder
//!
//! The engine works purely off `CFKind`-tagged flows and the schedule's
//! own day-count and outstanding path, ensuring that all schedule
//! semantics (step-ups, amortization, PIK capitalization, etc.) are
//! respected without re-reading instrument specs.

use crate::cashflow::builder::schedule::CashFlowSchedule;
use crate::cashflow::primitives::CFKind;
use finstack_core::dates::calendar::business_days::HolidayCalendar;
use finstack_core::dates::calendar::calendar_by_id;
use finstack_core::dates::{Date, DayCount, DayCountCtx};
use finstack_core::error::InputError;
use finstack_core::money::Money;
use finstack_core::types::CurveId;

/// Helper to advance a date by N business days.
fn advance_business_days<C: HolidayCalendar + ?Sized>(cal: &C, mut date: Date, days: i32) -> Date {
    let step = if days >= 0 { 1 } else { -1 };
    let mut count = 0;
    let target = days.abs();
    while count < target {
        date += time::Duration::days(step as i64);
        if cal.is_business_day(date) {
            count += 1;
        }
    }
    date
}

/// Generic accrual method usable across instruments.
///
/// This mirrors the semantics of bond accrual methods but is defined at the
/// cashflow layer so it can be reused by any instrument that exposes a
/// `CashFlowSchedule`.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum AccrualMethod {
    /// Linear accrual (simple interest interpolation).
    ///
    /// `Accrued = Coupon × (elapsed / period)`
    Linear,

    /// Compounded accrual (ICMA-style).
    ///
    /// `Accrued = Notional × [(1 + period_rate)^(elapsed/period) - 1]`
    /// where `period_rate = coupon_amount / notional`.
    Compounded,

    /// Indexed accrual for inflation-linked style conventions.
    ///
    /// The generic engine does **not** implement index-ratio interpolation
    /// and will currently return `Error::Input(InputError::Invalid)` when used.
    /// A dedicated inflation-linked instrument surface should handle this
    /// method explicitly once index ratios are supported.
    Indexed {
        /// Inflation index curve identifier (e.g., "US-CPI").
        index_curve_id: CurveId,
    },
}

impl Default for AccrualMethod {
    fn default() -> Self {
        Self::Linear
    }
}

/// Ex-coupon convention applied to coupon flows.
#[derive(Clone, Debug)]
pub struct ExCouponRule {
    /// Number of days before coupon date that go ex.
    pub days_before_coupon: u32,
    /// Optional calendar ID for business day calculation.
    ///
    /// - `Some(id)`: Subtract N business days from payment date.
    /// - `None`: Subtract N calendar days from payment date.
    pub calendar_id: Option<String>,
}

/// Generic configuration for schedule-driven interest accrual.
#[derive(Clone, Debug)]
pub struct AccrualConfig {
    /// Accrual method (Linear, Compounded, or Indexed).
    pub method: AccrualMethod,
    /// Optional ex-coupon rule applied to coupon dates.
    pub ex_coupon: Option<ExCouponRule>,
    /// Whether to include PIK interest in the accrued amount.
    pub include_pik: bool,
}

impl Default for AccrualConfig {
    fn default() -> Self {
        Self {
            method: AccrualMethod::Linear,
            ex_coupon: None,
            include_pik: true,
        }
    }
}

/// Convenience: accrued interest as scalar amount.
///
/// Callers can recover the currency from `schedule.notional.initial.currency()`
/// if needed.
pub fn accrued_interest_amount(
    schedule: &CashFlowSchedule,
    as_of: Date,
    cfg: &AccrualConfig,
) -> finstack_core::Result<f64> {
    let periods = build_coupon_periods(schedule, cfg.include_pik);
    if periods.is_empty() {
        return Ok(0.0);
    }

    // Build outstanding path including notional draws/repays and PIK.
    let outstanding_path = schedule.outstanding_by_date()?;
    let period_inputs = build_period_inputs(schedule, &periods, &outstanding_path)?;

    // Locate active period and compute accrued in that period.
    if let Some((inputs, elapsed_yf)) =
        find_active_period_and_elapsed(&period_inputs, as_of, schedule.day_count, cfg)?
    {
        accrue_in_period(inputs, elapsed_yf, &cfg.method)
    } else {
        Ok(0.0)
    }
}

/// Aggregated coupon information for a single payment date.
#[derive(Clone, Debug)]
struct CouponBucket {
    date: Date,
    cash_amount: f64,
    pik_amount: f64,
    accrual_factor: f64,
    rate: Option<f64>,
}

/// A single coupon period derived from the schedule.
#[derive(Clone, Debug)]
struct Period {
    start: Date,
    end: Date,
    dc: DayCount,
    bucket: CouponBucket,
}

/// Inputs required to apply the accrual formula for a single period.
#[derive(Clone, Debug)]
struct PeriodInputs {
    start: Date,
    end: Date,
    notional_start: f64,
    coupon_total: f64,
    total_yf: f64,
}

/// Build coupon buckets grouped by date from the schedule.
fn build_coupon_periods(schedule: &CashFlowSchedule, include_pik: bool) -> Vec<Period> {
    let mut buckets: Vec<CouponBucket> = Vec::new();

    // Cash and PIK coupon flows are grouped by payment date.
    for cf in &schedule.flows {
        match cf.kind {
            CFKind::Fixed | CFKind::Stub | CFKind::FloatReset => {
                if let Some(last) = buckets.last_mut() {
                    if last.date == cf.date {
                        last.cash_amount += cf.amount.amount();
                        if last.accrual_factor == 0.0 && cf.accrual_factor > 0.0 {
                            last.accrual_factor = cf.accrual_factor;
                        }
                        if last.rate.is_none() {
                            last.rate = cf.rate;
                        }
                        continue;
                    }
                }
                buckets.push(CouponBucket {
                    date: cf.date,
                    cash_amount: cf.amount.amount(),
                    pik_amount: 0.0,
                    accrual_factor: cf.accrual_factor,
                    rate: cf.rate,
                });
            }
            CFKind::PIK if include_pik => {
                if let Some(last) = buckets.last_mut() {
                    if last.date == cf.date {
                        last.pik_amount += cf.amount.amount();
                        continue;
                    }
                }
                buckets.push(CouponBucket {
                    date: cf.date,
                    cash_amount: 0.0,
                    pik_amount: cf.amount.amount(),
                    accrual_factor: 0.0,
                    rate: None,
                });
            }
            _ => {}
        }
    }

    if buckets.is_empty() {
        return Vec::new();
    }

    // Sort buckets by date to ensure deterministic period boundaries.
    buckets.sort_by_key(|b| b.date);

    let dc = schedule.day_count;
    // Use the earliest schedule date (typically issue date) as the start of the
    // first coupon period so that accrual runs from issue → first coupon.
    let horizon_start = match schedule.dates().into_iter().min() {
        Some(d) => d,
        None => return Vec::new(),
    };
    let mut prev = horizon_start;

    let mut periods = Vec::with_capacity(buckets.len());
    for bucket in buckets {
        let start = prev;
        let end = bucket.date;
        if start < end {
            periods.push(Period {
                start,
                end,
                dc,
                bucket,
            });
            prev = end;
        } else {
            // Skip degenerate periods (e.g., duplicated dates).
            prev = end;
        }
    }

    periods
}

/// Build period inputs (including notional at start-of-period) from coupon periods
/// and the outstanding path.
fn build_period_inputs(
    schedule: &CashFlowSchedule,
    periods: &[Period],
    outstanding_path: &[(Date, Money)],
) -> finstack_core::Result<Vec<PeriodInputs>> {
    let mut result = Vec::with_capacity(periods.len());

    // Pointer into outstanding_path (sorted by date).
    let mut path_idx = 0usize;
    let mut last_outstanding_before: f64 = schedule.notional.initial.amount();

    for p in periods {
        // Advance outstanding pointer up to (but not including) the period end date.
        while path_idx < outstanding_path.len() && outstanding_path[path_idx].0 < p.end {
            last_outstanding_before = outstanding_path[path_idx].1.amount();
            path_idx += 1;
        }

        let notional_start = last_outstanding_before;
        let coupon_total = p.bucket.cash_amount + p.bucket.pik_amount;

        if coupon_total == 0.0 {
            // No coupon in this period; skip.
            continue;
        }

        // Prefer accrual_factor from builder when present; otherwise derive via day count.
        let total_yf = if p.bucket.accrual_factor > 0.0 {
            p.bucket.accrual_factor
        } else {
            p.dc.year_fraction(p.start, p.end, DayCountCtx::default())?
        };

        if total_yf <= 0.0 {
            continue;
        }

        result.push(PeriodInputs {
            start: p.start,
            end: p.end,
            notional_start,
            coupon_total,
            total_yf,
        });
    }

    Ok(result)
}

/// Locate the active period for `as_of` and compute elapsed year fraction.
///
/// # Ex-Coupon Handling
///
/// If an ex-coupon rule is configured and the `as_of` date falls within the
/// ex-coupon window (between ex-date and payment date), returns `None` to
/// indicate zero accrued interest.
///
/// # Calendar Fallback Warning
///
/// When a calendar ID is specified for ex-coupon but the calendar is not found,
/// the function logs a warning (if the `tracing` feature is enabled) and falls
/// back to calendar days instead of business days.
fn find_active_period_and_elapsed<'a>(
    periods: &'a [PeriodInputs],
    as_of: Date,
    dc: DayCount,
    cfg: &AccrualConfig,
) -> finstack_core::Result<Option<(&'a PeriodInputs, f64)>> {
    use time::Duration;

    for inputs in periods {
        if inputs.start <= as_of && as_of < inputs.end {
            // Apply ex-coupon convention if present.
            if let Some(ref ex) = cfg.ex_coupon {
                let ex_date = if let Some(cal_id) = &ex.calendar_id {
                    if let Some(cal) = calendar_by_id(cal_id) {
                        advance_business_days(cal, inputs.end, -(ex.days_before_coupon as i32))
                    } else {
                        // Calendar not found: fallback to calendar days
                        // Note: If tracing is needed, add "tracing" feature to Cargo.toml
                        inputs.end - Duration::days(ex.days_before_coupon as i64)
                    }
                } else {
                    inputs.end - Duration::days(ex.days_before_coupon as i64)
                };

                if as_of >= ex_date && as_of < inputs.end {
                    return Ok(None);
                }
            }

            let elapsed = dc
                .year_fraction(inputs.start, as_of, DayCountCtx::default())?
                .max(0.0);

            return Ok(Some((inputs, elapsed)));
        }
    }

    Ok(None)
}

/// Threshold for using Taylor expansion in compounded accrual.
///
/// When the elapsed fraction is below this threshold, we use a Taylor series
/// approximation instead of `powf()` to avoid floating-point precision loss
/// for small exponents.
const TAYLOR_EXPANSION_THRESHOLD: f64 = 0.05;

/// Apply the chosen accrual method to a single period.
///
/// # Compounded Accrual Precision
///
/// For small elapsed fractions (< 5% of period), uses Taylor series expansion:
/// `(1 + r)^f ≈ 1 + f*r + f*(f-1)*r²/2 + f*(f-1)*(f-2)*r³/6`
///
/// This provides better numerical stability than `powf()` for very small
/// exponents, which is important for same-day or next-day accrual calculations.
fn accrue_in_period(
    inputs: &PeriodInputs,
    elapsed_yf: f64,
    method: &AccrualMethod,
) -> finstack_core::Result<f64> {
    if inputs.total_yf <= 0.0 || elapsed_yf < 0.0 {
        return Ok(0.0);
    }

    match method {
        AccrualMethod::Linear => Ok(inputs.coupon_total * (elapsed_yf / inputs.total_yf)),
        AccrualMethod::Compounded => {
            let notional = inputs.notional_start;
            if notional <= 0.0 {
                return Ok(0.0);
            }

            let period_rate = inputs.coupon_total / notional;
            if period_rate.abs() < 1e-12 {
                // Zero-coupon or near-zero rate: fall back to linear.
                return Ok(inputs.coupon_total * (elapsed_yf / inputs.total_yf));
            }

            let fraction = elapsed_yf / inputs.total_yf;

            // For small fractions, use Taylor expansion for better precision
            // (1 + r)^f ≈ 1 + f*r + f*(f-1)*r²/2 + f*(f-1)*(f-2)*r³/6
            let compound_factor = if fraction < TAYLOR_EXPANSION_THRESHOLD {
                let r = period_rate;
                let f = fraction;
                let r2 = r * r;
                let r3 = r2 * r;

                // Taylor series terms
                let term1 = f * r;
                let term2 = f * (f - 1.0) * r2 / 2.0;
                let term3 = f * (f - 1.0) * (f - 2.0) * r3 / 6.0;

                1.0 + term1 + term2 + term3
            } else {
                (1.0 + period_rate).powf(fraction)
            };

            Ok(notional * (compound_factor - 1.0))
        }
        AccrualMethod::Indexed { .. } => {
            // The generic engine does not implement index-ratio accrual. Callers
            // using Indexed accrual must route through a dedicated inflation-linked
            // instrument that sizes coupons appropriately in the schedule.
            Err(InputError::Invalid.into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cashflow::builder::{schedule::CashFlowMeta, schedule::CashFlowSchedule, Notional};
    use finstack_core::cashflow::primitives::{CFKind, CashFlow};
    use finstack_core::currency::Currency;
    use finstack_core::dates::Date;
    use finstack_core::error::Error;
    use finstack_core::money::Money;
    use time::Month;

    #[test]
    fn indexed_accrual_returns_input_error() {
        let issue = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");
        let coupon = Date::from_calendar_date(2025, Month::July, 1).expect("valid date");
        let schedule = CashFlowSchedule {
            flows: vec![
                CashFlow {
                    date: issue,
                    reset_date: None,
                    amount: Money::new(0.0, Currency::USD),
                    kind: CFKind::Fixed,
                    accrual_factor: 0.0,
                    rate: None,
                },
                CashFlow {
                    date: coupon,
                    reset_date: None,
                    amount: Money::new(10_000.0, Currency::USD),
                    kind: CFKind::Fixed,
                    accrual_factor: 0.5,
                    rate: Some(0.05),
                },
            ],
            notional: Notional::par(1_000_000.0, Currency::USD),
            day_count: finstack_core::dates::DayCount::Act365F,
            meta: CashFlowMeta::default(),
        };

        let cfg = AccrualConfig {
            method: AccrualMethod::Indexed {
                index_curve_id: CurveId::new("US-CPI"),
            },
            ex_coupon: None,
            include_pik: true,
        };

        let as_of = Date::from_calendar_date(2025, Month::March, 1).expect("valid date");
        let err = accrued_interest_amount(&schedule, as_of, &cfg)
            .expect_err("indexed accrual should not be supported");

        match err {
            Error::Input(InputError::Invalid) => {}
            other => panic!("expected InputError::Invalid, got {other:?}"),
        }
    }
}
