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
use finstack_core::dates::calendar::calendar_by_id;
use finstack_core::dates::HolidayCalendar;
use finstack_core::dates::{Date, DayCount, DayCountCtx};
use finstack_core::money::Money;

/// Maximum reasonable accrual factor for deriving issue date from coupon periods.
///
/// When the schedule does not include an explicit issue date flow, we derive it
/// by working backwards from the first coupon date using the accrual factor.
/// This constant bounds that calculation to prevent unreasonable results.
///
/// A value of 1.5 accommodates:
/// - Standard periods: annual (1.0), semi-annual (0.5), quarterly (0.25)
/// - Long stub periods: up to 18 months (1.5) which covers most bond structures
///
/// If accrual_factor exceeds this bound, we log a warning and fall back to
/// using the first coupon date, which creates a zero-length first period.
const MAX_REASONABLE_ACCRUAL_FACTOR: f64 = 1.5;

/// Helper to advance a date by N business days.
///
/// # Performance
///
/// For large shifts (>5 business days), this function uses week-jumping
/// optimization: it advances by full weeks (7 calendar days ≈ 5 business days)
/// to reduce calendar lookups from O(N) to approximately O(N/5) + O(5).
/// This significantly improves performance for long-dated ex-coupon or
/// settlement calculations (e.g., 1 year shift becomes ~52 jumps instead of ~260 lookups).
fn advance_business_days<C: HolidayCalendar + ?Sized>(cal: &C, mut date: Date, days: i32) -> Date {
    if days == 0 {
        return date;
    }

    let forward = days > 0;
    let mut remaining = days.unsigned_abs();

    // Week-jumping optimization for large shifts.
    // A standard week has 5 business days (Mon-Fri), so 7 calendar days ≈ 5 business days.
    // We jump by full weeks to minimize calendar lookups.
    //
    // Note: This assumes a standard Monday-Friday business week. Markets with different
    // conventions (e.g., Sunday-Thursday in Middle East) will see suboptimal performance
    // but still produce correct results since we count actual business days after jumping.
    const BUSINESS_DAYS_PER_WEEK: u32 = 5;
    const CALENDAR_DAYS_PER_WEEK: i64 = 7;

    while remaining >= BUSINESS_DAYS_PER_WEEK {
        // Jump one week in the appropriate direction
        let jump_days = if forward {
            CALENDAR_DAYS_PER_WEEK
        } else {
            -CALENDAR_DAYS_PER_WEEK
        };
        date += time::Duration::days(jump_days);

        // Count actual business days in the week we jumped.
        // This handles weeks with holidays correctly.
        let mut week_business_days = 0u32;
        // For forward: check the 7 days we just traversed (from day after old position to new position)
        // For backward: check from new position to day before old position
        let check_start = if forward {
            date + time::Duration::days(-CALENDAR_DAYS_PER_WEEK + 1)
        } else {
            date
        };

        for i in 0..CALENDAR_DAYS_PER_WEEK {
            let check_date = check_start + time::Duration::days(i);
            if cal.is_business_day(check_date) {
                week_business_days += 1;
            }
        }

        // Guard: if no business days in the week (pathological calendar with 7+ consecutive
        // holidays), revert the jump and fall through to step-by-step iteration.
        if week_business_days == 0 {
            date += time::Duration::days(-jump_days);
            break;
        }

        // Deduct the business days we actually traversed
        remaining = remaining.saturating_sub(week_business_days);
    }

    // Handle remaining days with step-by-step iteration (at most 4 business days)
    let step = if forward { 1i64 } else { -1i64 };
    while remaining > 0 {
        date += time::Duration::days(step);
        if cal.is_business_day(date) {
            remaining -= 1;
        }
    }

    date
}

/// Generic accrual method usable across instruments.
///
/// This mirrors the semantics of bond accrual methods but is defined at the
/// cashflow layer so it can be reused by any instrument that exposes a
/// `CashFlowSchedule`.
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum AccrualMethod {
    /// Linear accrual (simple interest interpolation).
    ///
    /// `Accrued = Coupon × (elapsed / period)`
    #[default]
    Linear,

    /// Compounded accrual (ICMA-style).
    ///
    /// `Accrued = Notional × [(1 + period_rate)^(elapsed/period) - 1]`
    /// where `period_rate = coupon_amount / notional`.
    Compounded,
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
    /// Accrual method (Linear or Compounded).
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

// =============================================================================
// Helper functions for build_coupon_periods (extracted for clarity)
// =============================================================================

/// Check if a cashflow kind is a coupon that should be included in accrual.
fn is_coupon_kind(kind: CFKind, include_pik: bool) -> bool {
    matches!(kind, CFKind::Fixed | CFKind::Stub | CFKind::FloatReset)
        || (include_pik && kind == CFKind::PIK)
}

/// Try to merge a cashflow into the last bucket if dates match.
/// Returns true if merged, false if a new bucket is needed.
fn try_merge_into_last_bucket(
    buckets: &mut [CouponBucket],
    cf: &finstack_core::cashflow::CashFlow,
) -> bool {
    let Some(last) = buckets.last_mut() else {
        return false;
    };

    if last.date != cf.date {
        return false;
    }

    // Merge based on cashflow kind
    if cf.kind == CFKind::PIK {
        last.pik_amount += cf.amount.amount();
    } else {
        last.cash_amount += cf.amount.amount();
        if last.accrual_factor == 0.0 && cf.accrual_factor > 0.0 {
            last.accrual_factor = cf.accrual_factor;
        }
        if last.rate.is_none() {
            last.rate = cf.rate;
        }
    }

    true
}

/// Create a new coupon bucket from a cashflow.
fn create_bucket(cf: &finstack_core::cashflow::CashFlow) -> CouponBucket {
    if cf.kind == CFKind::PIK {
        CouponBucket {
            date: cf.date,
            cash_amount: 0.0,
            pik_amount: cf.amount.amount(),
            accrual_factor: 0.0,
            rate: None,
        }
    } else {
        CouponBucket {
            date: cf.date,
            cash_amount: cf.amount.amount(),
            pik_amount: 0.0,
            accrual_factor: cf.accrual_factor,
            rate: cf.rate,
        }
    }
}

/// Build coupon buckets grouped by date from the schedule.
fn build_coupon_periods(schedule: &CashFlowSchedule, include_pik: bool) -> Vec<Period> {
    let mut buckets: Vec<CouponBucket> = Vec::new();

    // Cash and PIK coupon flows are grouped by payment date.
    for cf in &schedule.flows {
        // Skip non-coupon flows
        if !is_coupon_kind(cf.kind, include_pik) {
            continue;
        }

        // Try to merge with existing bucket for same date
        if try_merge_into_last_bucket(&mut buckets, cf) {
            continue;
        }

        // Create new bucket
        buckets.push(create_bucket(cf));
    }

    if buckets.is_empty() {
        return Vec::new();
    }

    // Sort buckets by date to ensure deterministic period boundaries.
    buckets.sort_by_key(|b| b.date);

    let dc = schedule.day_count;

    // Derive the start of the first coupon period (issue date).
    //
    // Strategy:
    // 1. If schedule.dates().min() differs from the first coupon date, use that
    //    (this handles cases where issue date flow exists in the schedule)
    // 2. Otherwise, derive issue date from first coupon's accrual factor using
    //    an inverse day count approximation:
    //    issue_date ≈ first_coupon_date - (accrual_factor × days_per_year)
    //
    // Note on inverse day count approximation:
    // This is the *inverse* of the standard year fraction calculation. While
    // day counts compute (days → year_fraction), we need (year_fraction → days).
    // The approximation uses the day count's denominator as days_per_year:
    // - 30/360, ACT/360: 360 days per year
    // - ACT/365F, ACT/ACT: 365 days per year (approximation for ACT/ACT)
    //
    // This may produce dates that differ by 1-2 days from the true issue date
    // for instruments with non-standard accrual periods, but is sufficient for
    // establishing coupon period boundaries for accrual calculations.
    let first_bucket = &buckets[0];
    let schedule_min = schedule.dates().into_iter().min();
    let horizon_start = match schedule_min {
        Some(min_date) if min_date < first_bucket.date => min_date,
        _ => {
            // Derive issue date from first coupon's accrual factor using inverse
            // day count approximation. See comment above for rationale.
            let days_per_year = match dc {
                DayCount::Thirty360 => 360.0,
                DayCount::Act360 => 360.0,
                _ => 365.0, // ACT/365F, ACT/ACT, etc.
            };

            // Only derive if accrual_factor is positive and within bounds
            if first_bucket.accrual_factor > 0.0
                && first_bucket.accrual_factor <= MAX_REASONABLE_ACCRUAL_FACTOR
            {
                let days_to_subtract = (first_bucket.accrual_factor * days_per_year).round() as i64;
                first_bucket.date - time::Duration::days(days_to_subtract)
            } else {
                // Fallback: use first coupon date as start (degenerate case).
                // This creates a zero-length first period which will be skipped.
                //
                // Note: This fallback indicates a potentially problematic schedule
                // where accrual_factor is 0 or > 1.5. Callers should ensure schedules
                // have valid accrual factors for proper accrual calculation.
                first_bucket.date
            }
        }
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
///
/// # Notional Lookup
///
/// For each period, we find the outstanding balance at the period start date.
/// This is the correct base for compounded accrual calculations since it
/// represents the principal on which interest accrues during the period.
fn build_period_inputs(
    schedule: &CashFlowSchedule,
    periods: &[Period],
    outstanding_path: &[(Date, Money)],
) -> finstack_core::Result<Vec<PeriodInputs>> {
    let mut result = Vec::with_capacity(periods.len());

    for p in periods {
        // Find the outstanding at period start by looking for the latest entry
        // on or before p.start. If no entry exists, use initial notional.
        // Use rev().find() instead of filter().last() to avoid iterating the
        // entire collection when we only need the last matching element.
        let notional_start = outstanding_path
            .iter()
            .rev()
            .find(|(d, _)| *d <= p.start)
            .map(|(_, m)| m.amount())
            .unwrap_or_else(|| schedule.notional.initial.amount());

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
                    let cal = calendar_by_id(cal_id).ok_or_else(|| {
                        finstack_core::Error::Input(finstack_core::InputError::NotFound {
                            id: cal_id.clone(),
                        })
                    })?;
                    advance_business_days(cal, inputs.end, -(ex.days_before_coupon as i32))
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

/// Apply the chosen accrual method to a single period.
///
/// # Compounded Accrual
///
/// Uses the numerically stable formula: `(1+r)^f - 1 = expm1(f * ln1p(r))`
///
/// This approach:
/// - Avoids precision loss for small `r` via `ln1p` (log(1+r) accurate near 0)
/// - Avoids precision loss for small results via `expm1` (exp(x)-1 accurate near 0)
/// - Works correctly across all fraction values without threshold switching
///
/// The compounded accrual formula follows ICMA Rule 251.1 for calculating accrued
/// interest on securities with periodic interest payments:
///
/// `Accrued = Notional × [(1 + period_rate)^(elapsed/period) - 1]`
///
/// where `period_rate = coupon_amount / notional` is the yield per coupon period.
///
/// Reference: ICMA Primary Market Handbook, Rule 251 (Accrued Interest Calculations)
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

            // Numerically stable computation: (1+r)^f - 1 = expm1(f * ln1p(r))
            // This avoids precision loss for both small rates and small fractions.
            let compound_growth = (fraction * period_rate.ln_1p()).exp_m1();

            Ok(notional * compound_growth)
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::cashflow::builder::{CashFlowSchedule, Notional};
    use finstack_core::cashflow::CashFlow;
    use finstack_core::currency::Currency;
    use time::Month;

    fn make_date(y: i32, m: u8, d: u8) -> Date {
        Date::from_calendar_date(y, Month::try_from(m).unwrap(), d).unwrap()
    }

    /// Assert that two dates are within a tolerance (in days).
    fn assert_date_approx(actual: Date, expected: Date, tolerance_days: i64, msg: &str) {
        let diff = (actual - expected).whole_days().abs();
        assert!(
            diff <= tolerance_days,
            "{}: expected {} ± {} days, got {} (diff = {} days)",
            msg,
            expected,
            tolerance_days,
            actual,
            diff
        );
    }

    /// Create a minimal schedule with coupon flows for testing issue date derivation.
    fn make_test_schedule(
        coupon_dates: &[(Date, f64)], // (date, accrual_factor)
        day_count: DayCount,
    ) -> CashFlowSchedule {
        let flows: Vec<CashFlow> = coupon_dates
            .iter()
            .map(|(date, af)| CashFlow {
                date: *date,
                amount: Money::new(25000.0, Currency::USD), // $25k coupon
                kind: CFKind::Fixed,
                accrual_factor: *af,
                rate: Some(0.05),
                reset_date: None,
            })
            .collect();

        CashFlowSchedule {
            flows,
            notional: Notional::par(1_000_000.0, Currency::USD),
            day_count,
            meta: Default::default(),
        }
    }

    // =========================================================================
    // Issue date derivation tests
    //
    // Note: The inverse day count approximation may produce dates that differ
    // by 1-2 days from the true issue date. This is acceptable for establishing
    // coupon period boundaries for accrual calculations, as the error is small
    // relative to the coupon period length.
    // =========================================================================

    #[test]
    fn test_issue_date_derivation_semi_annual_30_360() {
        // Semi-annual bond with 30/360 day count
        // First coupon: July 1, 2025, accrual_factor = 0.5 (6 months)
        // Expected derived issue: ~Jan 1, 2025 (180 days before in 30/360)
        let schedule = make_test_schedule(
            &[
                (make_date(2025, 7, 1), 0.5), // First coupon
                (make_date(2026, 1, 1), 0.5), // Second coupon
            ],
            DayCount::Thirty360,
        );

        let periods = build_coupon_periods(&schedule, false);

        assert!(!periods.is_empty(), "Should have coupon periods");
        let first_period = &periods[0];

        // Derived issue date: July 1 - (0.5 × 360) = July 1 - 180 days ≈ Jan 2
        // (Calendar days differ slightly from 30/360 convention)
        assert_date_approx(
            first_period.start,
            make_date(2025, 1, 1),
            2, // Allow 2 days tolerance for inverse approximation
            "Derived issue date should be ~Jan 1, 2025",
        );
        assert_eq!(first_period.end, make_date(2025, 7, 1));
    }

    #[test]
    fn test_issue_date_derivation_quarterly_act365() {
        // Quarterly bond with ACT/365F day count
        // First coupon: April 1, 2025, accrual_factor = 0.25 (3 months)
        // Expected derived issue: ~Jan 1, 2025 (91 days before in ACT/365)
        let schedule = make_test_schedule(
            &[
                (make_date(2025, 4, 1), 0.25), // First coupon
                (make_date(2025, 7, 1), 0.25), // Second coupon
            ],
            DayCount::Act365F,
        );

        let periods = build_coupon_periods(&schedule, false);

        assert!(!periods.is_empty());
        let first_period = &periods[0];

        // Derived issue date: April 1 - (0.25 × 365) = April 1 - 91 days ≈ Dec 31 or Jan 1
        // (Actual Jan 1 to Apr 1 = 90 days, so 91 gives Dec 31)
        assert_date_approx(
            first_period.start,
            make_date(2025, 1, 1),
            2, // Allow 2 days tolerance
            "Derived issue date should be ~Jan 1, 2025",
        );
    }

    #[test]
    fn test_issue_date_derivation_long_stub() {
        // Bond with 18-month long first stub (accrual_factor = 1.5)
        // This is at the boundary of MAX_REASONABLE_ACCRUAL_FACTOR
        let schedule = make_test_schedule(
            &[
                (make_date(2026, 7, 1), 1.5), // Long stub (18 months)
                (make_date(2027, 1, 1), 0.5), // Regular coupon
            ],
            DayCount::Thirty360,
        );

        let periods = build_coupon_periods(&schedule, false);

        assert!(!periods.is_empty());
        let first_period = &periods[0];

        // Derived issue date: July 1, 2026 - (1.5 × 360) = July 1, 2026 - 540 days
        // ≈ January 2025 (exact date depends on calendar vs day count)
        assert_date_approx(
            first_period.start,
            make_date(2025, 1, 7),
            2, // Allow 2 days tolerance
            "Derived issue date for long stub",
        );
    }

    #[test]
    fn test_issue_date_derivation_fallback_zero_accrual_factor() {
        // Edge case: accrual_factor = 0 (invalid, should fallback)
        let schedule = make_test_schedule(
            &[
                (make_date(2025, 7, 1), 0.0), // Invalid accrual factor
                (make_date(2026, 1, 1), 0.5),
            ],
            DayCount::Thirty360,
        );

        let periods = build_coupon_periods(&schedule, false);

        // With zero accrual factor, we hit fallback: first_bucket.date = July 1
        // This creates a zero-length first period (start == end), which is skipped
        // So we should only have one period (July 1 to Jan 1)
        assert_eq!(
            periods.len(),
            1,
            "Zero-length first period should be skipped"
        );
        assert_eq!(periods[0].start, make_date(2025, 7, 1));
        assert_eq!(periods[0].end, make_date(2026, 1, 1));
    }

    #[test]
    fn test_issue_date_derivation_fallback_excessive_accrual_factor() {
        // Edge case: accrual_factor > MAX_REASONABLE_ACCRUAL_FACTOR (should fallback)
        let schedule = make_test_schedule(
            &[
                (make_date(2025, 7, 1), 2.0), // > 1.5, triggers fallback
                (make_date(2026, 1, 1), 0.5),
            ],
            DayCount::Thirty360,
        );

        let periods = build_coupon_periods(&schedule, false);

        // With excessive accrual factor, we hit fallback: first_bucket.date = July 1
        // This creates a zero-length first period, which is skipped
        assert_eq!(
            periods.len(),
            1,
            "First period should be skipped due to fallback"
        );
        assert_eq!(periods[0].start, make_date(2025, 7, 1));
    }

    #[test]
    fn test_issue_date_from_schedule_when_present() {
        // When schedule includes a flow before the first coupon (e.g., notional),
        // that date should be used instead of derivation
        let mut schedule = make_test_schedule(
            &[(make_date(2025, 7, 1), 0.5), (make_date(2026, 1, 1), 0.5)],
            DayCount::Thirty360,
        );

        // Add a notional flow on issue date
        schedule.flows.insert(
            0,
            CashFlow {
                date: make_date(2025, 1, 15), // Issue date (different from derived Jan 1)
                amount: Money::new(-1_000_000.0, Currency::USD),
                kind: CFKind::Notional,
                accrual_factor: 0.0,
                rate: None,
                reset_date: None,
            },
        );

        let periods = build_coupon_periods(&schedule, false);

        assert!(!periods.is_empty());
        let first_period = &periods[0];

        // Should use the explicit issue date from the schedule, not derived
        assert_eq!(
            first_period.start,
            make_date(2025, 1, 15),
            "Should use explicit issue date from schedule"
        );
    }

    #[test]
    fn test_accrued_interest_uses_derived_issue_date() {
        // Integration test: verify accrued interest calculation works with derived issue date
        let schedule = make_test_schedule(
            &[
                (make_date(2025, 7, 1), 0.5), // First coupon July 1
                (make_date(2026, 1, 1), 0.5), // Second coupon
            ],
            DayCount::Thirty360,
        );

        // Calculate accrued at April 1 (halfway through first period)
        let as_of = make_date(2025, 4, 1);
        let accrued = accrued_interest_amount(&schedule, as_of, &AccrualConfig::default()).unwrap();

        // With derived issue date Jan 1 and first coupon July 1:
        // - Period length: 180 days (30/360)
        // - Elapsed: 90 days (Jan 1 to Apr 1)
        // - Fraction: 90/180 = 0.5
        // - Accrued: $25,000 × 0.5 = $12,500
        assert!(
            accrued > 12_000.0 && accrued < 13_000.0,
            "Accrued should be approximately $12,500, got {}",
            accrued
        );
    }
}
