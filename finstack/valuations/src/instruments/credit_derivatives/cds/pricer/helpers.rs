use super::config::max_deliverable_maturity;
use crate::constants::{credit, numerical};
use crate::instruments::credit_derivatives::cds::{CdsDocClause, CreditDefaultSwap};
use finstack_core::dates::{Date, DateExt, DayCount, HolidayCalendar};
use finstack_core::market_data::term_structures::{DiscountCurve, HazardCurve};
use finstack_core::Result;
use time::Duration;

// ----- Time-axis helpers -----
//
// These helpers ensure we use the correct day-count conventions:
// - For discounting: use the discount curve's day-count convention
// - For survival: use the hazard curve's day-count convention
// - For accrual: use the instrument's premium leg day-count convention

/// Compute time from hazard curve's base date using its day-count convention.
#[inline]
pub(super) fn haz_t(surv: &HazardCurve, date: Date) -> Result<f64> {
    surv.day_count().year_fraction(
        surv.base_date(),
        date,
        finstack_core::dates::DayCountContext::default(),
    )
}

/// Approximate inverse mapping from hazard-curve time (years) to a calendar date.
///
/// This is exact for ACT/365F and ACT/360 hazard curve day-counts (since the forward
/// mapping uses actual day counts), and a reasonable approximation for other
/// conventions. The resulting date is used only for discounting on actual dates.
#[inline]
pub(crate) fn date_from_hazard_time(surv: &HazardCurve, t: f64) -> Date {
    let t = t.max(0.0);
    let days_per_year = match surv.day_count() {
        DayCount::Act360 => 360.0,
        DayCount::Act365F => 365.0,
        DayCount::Act365L | DayCount::ActAct | DayCount::ActActIsma => 365.25,
        DayCount::Thirty360 | DayCount::ThirtyE360 => 360.0,
        DayCount::Bus252 => 252.0,
        // Fallback for less common conventions; used only for discount-date mapping.
        _ => 365.25,
    };
    let days = (t * days_per_year).round() as i64;
    surv.base_date() + Duration::days(days)
}

/// Resolve settlement date for a default occurring on `default_date`.
#[inline]
pub(super) fn settlement_date(
    default_date: Date,
    settlement_delay: u16,
    calendar: Option<&dyn HolidayCalendar>,
    business_days_per_year: f64,
) -> Result<Date> {
    if settlement_delay == 0 {
        return Ok(default_date);
    }

    if let Some(cal) = calendar {
        return default_date.add_business_days(settlement_delay as i32, cal);
    }

    // Fallback: approximate business days into calendar days.
    let delay_days = ((settlement_delay as f64) * credit::CALENDAR_DAYS_PER_YEAR
        / business_days_per_year)
        .round() as i64;
    Ok(default_date + Duration::days(delay_days))
}

pub(super) fn isda_standard_model_boundaries(
    t_start: f64,
    t_end: f64,
    surv: &HazardCurve,
    disc: &DiscountCurve,
) -> Vec<f64> {
    let mut boundaries = Vec::with_capacity(surv.len() + disc.knots().len() + 2);
    boundaries.push(t_start);
    boundaries.push(t_end);
    boundaries.extend(
        surv.knot_points()
            .map(|(t, _)| t)
            .filter(|&t| t > t_start && t < t_end),
    );
    boundaries.extend(
        disc.knots()
            .iter()
            .copied()
            .filter(|&t| t > t_start && t < t_end),
    );
    boundaries.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    boundaries.dedup_by(|a, b| (*a - *b).abs() <= numerical::ZERO_TOLERANCE);
    boundaries
}

/// Compute discount factor from as_of to date using curve's time axis.
/// This returns df(date) / df(as_of) = exp(-r*(t_date - t_asof))
#[inline]
pub(super) fn df_asof_to(disc: &DiscountCurve, as_of: Date, date: Date) -> Result<f64> {
    disc.df_between_dates(as_of, date)
}

/// Compute conditional survival probability: S(date | survived to as_of).
/// Returns S(t_date) / S(t_asof) where times are computed using hazard curve's day-count.
///
/// Uses `credit::SURVIVAL_PROBABILITY_FLOOR` to prevent division by near-zero
/// values that could produce inf/NaN results.
#[inline]
pub(super) fn sp_cond_to(surv: &HazardCurve, as_of: Date, date: Date) -> Result<f64> {
    let t_asof = haz_t(surv, as_of)?;
    let t_date = haz_t(surv, date)?;
    let sp_asof = surv.sp(t_asof);
    let sp_date = surv.sp(t_date);
    // Conditional survival: S(date) / S(as_of)
    // Use floor constant to prevent division by near-zero producing inf/NaN
    if sp_asof > credit::SURVIVAL_PROBABILITY_FLOOR {
        Ok(sp_date / sp_asof)
    } else {
        Ok(0.0) // Already defaulted (or effectively defaulted) by as_of
    }
}

/// Compute a multiplicative adjustment factor for the protection leg PV
/// based on the effective documentation clause.
///
/// Restructuring credit events increase the probability of a payout (more
/// event types can trigger protection). The factor represents how much
/// additional protection value the restructuring clause provides relative
/// to the base default-only protection.
///
/// The factor is calibrated to approximate market practice:
///
/// | Clause | Factor | Rationale |
/// |--------|--------|-----------|
/// | `Xr14` | 1.00 | Baseline: default events only |
/// | `Mr14` | 1.02 | Small uplift: limited deliverables (30 months) |
/// | `Mm14` | 1.03 | Moderate uplift: longer deliverable window (60 months) |
/// | `Cr14` | 1.05 | Full uplift: unrestricted deliverables |
/// | `Custom`| 1.00 | Conservative: no restructuring benefit assumed |
///
/// These factors are first-order approximations. In production, a full
/// restructuring model would separate the restructuring hazard rate from
/// the default hazard rate.
///
/// # Status: opt-in
///
/// This adjustment is **disabled by default** (see
/// [`CDSPricerConfig::default`]) and applied only when callers explicitly
/// set `enable_restructuring_approximation = true`. The factor is
/// preserved as a documented first-order heuristic so that:
///
/// 1. Desks that need a quick proxy for restructuring uplift can opt in
///    without taking the cost of a full restructuring-hazard model;
/// 2. Future replacement with a calibrated restructuring-hazard model
///    has a clear seam — callers continue to pass the same opt-in flag.
///
/// Do not enable this for production marks without sign-off from the
/// credit-modelling owner.
pub(super) fn restructuring_adjustment_factor(
    clause: CdsDocClause,
    cds: &CreditDefaultSwap,
) -> f64 {
    let cap = max_deliverable_maturity(clause);
    match cap {
        Some(0) => {
            // No restructuring benefit (Xr14 or Custom)
            1.0
        }
        Some(months) => {
            // Limited restructuring: scale based on how much of the CDS tenor
            // the restructuring cap covers. If the cap exceeds the remaining
            // tenor, the full restructuring benefit applies.
            let tenor_months = {
                let start = cds.premium.start;
                let end = cds.premium.end;
                // Approximate tenor in months
                let days = (end - start).whole_days();
                days as f64 / 30.44 // average days per month
            };
            // Coverage ratio: what fraction of the CDS tenor is covered by the cap
            let coverage = (months as f64 / tenor_months).min(1.0);
            // Base restructuring premium scaled by coverage
            // MR14 (30 months) has ~2% base uplift, MM14 (60 months) has ~3%
            let base_uplift = if months <= 30 { 0.02 } else { 0.03 };
            1.0 + base_uplift * coverage
        }
        None => {
            // Full restructuring (Cr14): uncapped deliverable maturity
            1.05
        }
    }
}
