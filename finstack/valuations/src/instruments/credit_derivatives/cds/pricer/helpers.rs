use crate::constants::{credit, numerical};
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

/// Bloomberg DOCS 2057273 §3 protection-leg integration spec: "the
/// timeline from T to TM is discretized into segments that are
/// sufficiently small to justify constant forward discounting rates and
/// constant hazard rate on each segment (and in no case longer than any
/// accrual period of the premium leg)."
///
/// We use ~25 sub-steps per year (matching FinancePy's
/// `GLOB_NUM_STEPS_PER_YEAR`), giving ~14-day resolution. This is finer
/// than any coupon period (~91 days) and finer than typical
/// discount-curve knot spacings, so within each segment both `r` and `λ`
/// are effectively constant under any reasonable interpolation. Curve
/// knots remain as boundaries so piecewise-constant hazard is honoured.
const PROTECTION_LEG_SUB_STEPS_PER_YEAR: f64 = 25.0;

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
    // Sub-step subdivision per DOCS 2057273 §3.
    let dt = 1.0 / PROTECTION_LEG_SUB_STEPS_PER_YEAR;
    let n_steps = ((t_end - t_start) * PROTECTION_LEG_SUB_STEPS_PER_YEAR).ceil() as usize;
    if n_steps > 0 {
        for i in 1..n_steps {
            let t = t_start + (i as f64) * dt;
            if t > t_start && t < t_end {
                boundaries.push(t);
            }
        }
    }
    // Times come from finite year-fractions on the curve day-counts; NaN here
    // would indicate a corrupt curve and produce silently-wrong PV. Fail fast.
    #[allow(clippy::expect_used)]
    // NaN here implies corrupt curve data; loud failure beats silent drift
    {
        boundaries.sort_by(|a, b| {
            a.partial_cmp(b)
                .expect("hazard/discount knot times must be finite for ISDA boundary integration")
        });
    }
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
