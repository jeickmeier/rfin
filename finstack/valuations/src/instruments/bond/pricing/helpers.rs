//! Bond pricing helpers (moved from bond/helpers.rs)

use finstack_core::dates::Date;
use finstack_core::dates::{BusinessDayConvention, DayCount, DayCountCtx, StubKind};
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;

/// Convert payment frequency to approximate periods per year.
///
/// **Important:** This function is for **frequency conversion only**, NOT day count conventions.
///
/// # Purpose
/// This helper determines how many payment periods occur in a year based on the
/// payment frequency. For example, semi-annual payments occur 2 times per year,
/// monthly payments occur 12 times per year.
///
/// # Day Count Conventions
/// Actual day count calculations (Actual/360, Actual/365, Actual/Actual, 30/360, etc.)
/// are handled separately via the `DayCount` enum and `year_fraction()` methods in
/// finstack-core. Those methods properly account for:
/// - Leap years (Actual/Actual)
/// - Different day count bases (360 vs 365)
/// - Month length variations (30/360)
///
/// # Examples
/// - Monthly payments (6 months): `12 / 6 = 2` periods/year (semi-annual frequency)
/// - Daily payments (90 days): `365 / 90 ≈ 4.06` periods/year (approximate)
///
/// # Note on Daily Frequency
/// For daily frequencies, this uses 365 as an approximation of annual periods.
/// This is appropriate for frequency calculations but should NOT be confused with
/// the Actual/365 day count convention used in accrual and discount factor calculations.
#[inline]
pub fn periods_per_year(freq: finstack_core::dates::Frequency) -> finstack_core::Result<f64> {
    match freq {
        finstack_core::dates::Frequency::Months(m) => {
            if m == 0 {
                return Err(finstack_core::error::InputError::Invalid.into());
            }
            Ok(12.0 / (m as f64))
        }
        finstack_core::dates::Frequency::Days(d) => {
            if d == 0 {
                return Err(finstack_core::error::InputError::Invalid.into());
            }
            // Use 365 as approximate annual basis for frequency calculations
            // Note: This is NOT a day count convention - actual day count is handled
            // via the DayCount enum (Actual/360, Actual/365, Actual/Actual, etc.)
            Ok(365.0 / (d as f64))
        }
        _ => Err(finstack_core::error::InputError::Invalid.into()),
    }
}

/// Fixed-leg annuity for a bond-style schedule using discount-curve discount factors.
///
/// This computes the standard swap-style annuity:
/// sum(alpha_i * P(as_of, T_i)) for i over future coupon dates, where
/// alpha_i is the year fraction between consecutive schedule dates under `dc`.
///
/// The `schedule` is expected to start at the valuation date (`as_of`) and
/// contain strictly increasing dates.
pub fn fixed_leg_annuity(disc: &DiscountCurve, dc: DayCount, schedule: &[Date]) -> f64 {
    if schedule.len() < 2 {
        return 0.0;
    }

    let mut ann = 0.0;
    let mut prev = schedule[0];
    for &d in &schedule[1..] {
        let alpha = dc
            .year_fraction(prev, d, DayCountCtx::default())
            .unwrap_or(0.0);
        let p = disc.df_on_date_curve(d);
        ann += alpha * p;
        prev = d;
    }
    ann
}

/// Par swap rate from discount-curve discount ratios and a fixed-leg annuity.
///
/// Uses the standard discount-ratio formula:
/// `par_rate = (P(as_of, T0) - P(as_of, Tn)) / sum(alpha_i * P(as_of, Ti))`
/// where the denominator is the fixed-leg annuity computed with `dc`.
///
/// Returns both the par rate and the annuity so callers can reuse the latter
/// in asset-swap formulas and related analytics.
pub fn par_rate_and_annuity_from_discount(
    disc: &DiscountCurve,
    dc: DayCount,
    schedule: &[Date],
) -> finstack_core::Result<(f64, f64)> {
    if schedule.len() < 2 {
        return Ok((0.0, 0.0));
    }

    let ann = fixed_leg_annuity(disc, dc, schedule);
    if ann == 0.0 {
        return Ok((0.0, 0.0));
    }

    let p0 = disc.df_on_date_curve(schedule[0]);
    let pn = disc.df_on_date_curve(*schedule.last().expect("Schedule should not be empty"));
    let num = p0 - pn;
    Ok((num / ann, ann))
}

/// Returns the default schedule parameters used across accrual/pricers to avoid duplication.
#[inline]
pub fn default_schedule_params() -> (StubKind, BusinessDayConvention, Option<&'static str>) {
    (StubKind::None, BusinessDayConvention::Following, None)
}
