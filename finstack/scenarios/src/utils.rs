//! Utility helpers for converting tenor and period strings.
//!
//! Adapters rely on these parsing helpers to turn human-readable inputs such as
//! `"5Y"` or `"3M"` into normalised numeric representations. The functions
//! return [`Result`](crate::error::Result) so they can bubble up friendly error
//! messages into the higher-level adapters.
//!
//! # Calendar-Aware Parsing
//!
//! For market-standard calculations that respect business day conventions and
//! holiday calendars, use [`parse_tenor_to_years_with_context`]. For simple
//! approximations suitable for most scenarios, use [`parse_tenor_to_years`].

use crate::error::{Error, Result};
use crate::spec::RateBindingSpec;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, HolidayCalendar, Tenor};
use finstack_core::market_data::term_structures::{
    discount_curve::DiscountCurve, forward_curve::ForwardCurve,
};

/// Parse a tenor string to a fractional number of years using simple approximations.
///
/// This function uses fixed approximations for quick calculations:
/// - Days: 1D = 1/365 years
/// - Weeks: 1W = 7/365 years
/// - Months: 1M = 1/12 years
/// - Years: 1Y = 1 year
///
/// For calendar-aware calculations that respect business days and holidays,
/// use [`parse_tenor_to_years_with_context`].
///
/// # Arguments
/// - `tenor`: Tenor string in formats like "1D", "1W", "3M", "5Y".
///   Leading/trailing whitespace is ignored, and input is case-insensitive.
///
/// # Returns
/// Number of years represented by the tenor. For example `"6M"` produces
/// `0.5` and `"1W"` produces roughly `0.01918`.
///
/// # Errors
/// Returns [`Error::InvalidTenor`](crate::error::Error::InvalidTenor) if the
/// string is empty, lacks a unit component, contains a non-numeric value, or
/// specifies an unsupported unit.
///
/// # Performance
///
/// This function is `#[inline]` for optimal performance in hot paths.
///
/// # Examples
/// ```
/// # use finstack_scenarios::utils::parse_tenor_to_years;
/// assert!((parse_tenor_to_years("1Y").unwrap() - 1.0).abs() < 1e-6);
/// assert!((parse_tenor_to_years("6M").unwrap() - 0.5).abs() < 1e-6);
/// assert!((parse_tenor_to_years("1W").unwrap() - (7.0 / 365.0)).abs() < 1e-3);
/// ```
#[inline]
pub fn parse_tenor_to_years(tenor: &str) -> Result<f64> {
    let parsed = Tenor::parse(tenor).map_err(|e| Error::InvalidTenor(e.to_string()))?;
    Ok(parsed.to_years_simple())
}

/// Parse a tenor string to a year fraction using calendar-aware computation.
///
/// This function computes actual year fractions by:
/// 1. Adding the tenor to the as-of date using proper date arithmetic
/// 2. Applying business day adjustment if a calendar is provided
/// 3. Computing the year fraction using the supplied day-count convention
///
/// # Arguments
/// - `tenor`: Tenor string in formats like "1D", "1W", "3M", "5Y"
/// - `as_of`: Starting date for the calculation
/// - `calendar`: Optional holiday calendar for business day adjustment
/// - `bdc`: Business-day convention to apply when a calendar is supplied
/// - `day_count`: Day-count convention for year fraction calculation
///
/// # Returns
/// Actual year fraction computed using calendar-aware date arithmetic.
///
/// # Errors
/// Returns an error if the tenor string is invalid or date computation fails.
pub fn parse_tenor_to_years_with_context(
    tenor: &str,
    as_of: Date,
    calendar: Option<&dyn HolidayCalendar>,
    bdc: BusinessDayConvention,
    day_count: DayCount,
) -> Result<f64> {
    let parsed = Tenor::parse(tenor).map_err(|e| Error::InvalidTenor(e.to_string()))?;

    parsed
        .to_years_with_context(as_of, calendar, bdc, day_count)
        .map_err(|e| Error::Internal(e.to_string()))
}

/// Parse a day-count string override into a [`DayCount`] enum.
pub fn parse_day_count_override(raw: &str) -> Result<DayCount> {
    let normalised = raw.trim().to_lowercase();
    let parsed = match normalised.as_str() {
        "act360" | "act/360" | "actual/360" => DayCount::Act360,
        "act365f" | "act/365f" | "actual/365" | "actual/365f" | "act365" => DayCount::Act365F,
        "actact" | "act/act" | "actual/actual" => DayCount::ActAct,
        "30/360" | "thirty360" => DayCount::Thirty360,
        "30e/360" | "30/360e" | "thirtye360" => DayCount::ThirtyE360,
        other => {
            return Err(Error::Validation(format!(
                "Unsupported day count override '{}'",
                other
            )))
        }
    };
    Ok(parsed)
}

/// Calendar-aware tenor conversion using the conventions of a discount curve.
pub fn tenor_years_from_discount_curve(
    tenor: &str,
    curve: &DiscountCurve,
    calendar: Option<&dyn HolidayCalendar>,
    bdc: BusinessDayConvention,
) -> Result<f64> {
    parse_tenor_to_years_with_context(tenor, curve.base_date(), calendar, bdc, curve.day_count())
}

/// Calendar-aware tenor conversion using the conventions of a forward curve.
pub fn tenor_years_from_forward_curve(
    tenor: &str,
    curve: &ForwardCurve,
    calendar: Option<&dyn HolidayCalendar>,
    bdc: BusinessDayConvention,
) -> Result<f64> {
    parse_tenor_to_years_with_context(tenor, curve.base_date(), calendar, bdc, curve.day_count())
}

/// Resolve the effective day-count and tenor length for a rate binding.
pub fn tenor_years_from_binding(
    binding: &RateBindingSpec,
    base_date: Date,
    default_day_count: DayCount,
    calendar: Option<&dyn HolidayCalendar>,
    bdc: BusinessDayConvention,
) -> Result<(f64, DayCount)> {
    let effective_dc = if let Some(dc) = &binding.day_count {
        parse_day_count_override(dc)?
    } else {
        default_day_count
    };

    let years =
        parse_tenor_to_years_with_context(&binding.tenor, base_date, calendar, bdc, effective_dc)?;
    Ok((years, effective_dc))
}

/// Parse a period string to an integer number of days.
///
/// Supports formats like:
/// - "1D", "7D" → days
/// - "1W" → 7 days
/// - "1M" → 30 days
/// - "1Y" → 365 days
///
/// # Arguments
/// - `period`: Period string matching one of the supported formats.
///
/// # Returns
/// Number of days represented by the period.
///
/// # Errors
/// Returns [`Error::InvalidPeriod`](crate::error::Error::InvalidPeriod) if the
/// string cannot be parsed.
///
/// # Examples
/// ```
/// # use finstack_scenarios::utils::parse_period_to_days;
/// assert_eq!(parse_period_to_days("1D").unwrap(), 1);
/// assert_eq!(parse_period_to_days("1W").unwrap(), 7);
/// assert_eq!(parse_period_to_days("1M").unwrap(), 30);
/// assert_eq!(parse_period_to_days("1Y").unwrap(), 365);
/// ```
pub fn parse_period_to_days(period: &str) -> Result<i64> {
    let parsed = Tenor::parse(period).map_err(|e| Error::InvalidPeriod(e.to_string()))?;

    let days = match parsed.unit {
        finstack_core::dates::TenorUnit::Days => i64::from(parsed.count),
        finstack_core::dates::TenorUnit::Weeks => i64::from(parsed.count) * 7,
        finstack_core::dates::TenorUnit::Months => i64::from(parsed.count) * 30,
        finstack_core::dates::TenorUnit::Years => i64::from(parsed.count) * 365,
    };

    Ok(days)
}

/// Calculate weights for distributing a bump at `target` year to adjacent curve pillars.
///
/// This provides standard linear interpolation weights to distribute a shock at `target`
/// onto the nearest knot points `t0` and `t1` such that the weighted average time matches
/// `target`.
///
/// # Arguments
/// - `target`: The time (in years) where the shock is applied.
/// - `knots`: Sorted slice of knot times (in years).
///
/// # Returns
/// A vector of `(index, weight)` tuples. Usually contains 1 (exact match or extrapolation)
/// or 2 (interpolation) elements.
pub fn calculate_interpolation_weights(target: f64, knots: &[f64]) -> Vec<(usize, f64)> {
    if knots.is_empty() {
        return vec![];
    }

    let pos = knots
        .iter()
        .position(|&t| t >= target)
        .unwrap_or(knots.len() - 1);

    if pos == 0 {
        return vec![(0, 1.0)];
    }

    let i0 = pos - 1;
    let i1 = pos.min(knots.len() - 1);
    let t0 = knots[i0];
    let t1 = knots[i1];

    if (t1 - t0).abs() < 1e-12 {
        // Coincident points, distribute evenly to avoiding div/0
        // (Should not happen in valid curves)
        vec![(i0, 0.5), (i1, 0.5)]
    } else {
        let w1 = (target - t0) / (t1 - t0);
        let w0 = 1.0 - w1;
        vec![(i0, w0), (i1, w1)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::Month;

    #[test]
    fn test_parse_tenor_years() {
        assert!((parse_tenor_to_years("1Y").expect("valid tenor") - 1.0).abs() < 1e-6);
        assert!((parse_tenor_to_years("5Y").expect("valid tenor") - 5.0).abs() < 1e-6);
        assert!((parse_tenor_to_years("6M").expect("valid tenor") - 0.5).abs() < 1e-6);
        assert!((parse_tenor_to_years("3M").expect("valid tenor") - 0.25).abs() < 1e-6);
    }

    #[test]
    fn test_parse_period_days() {
        assert_eq!(parse_period_to_days("1D").expect("valid period"), 1);
        assert_eq!(parse_period_to_days("7D").expect("valid period"), 7);
        assert_eq!(parse_period_to_days("1W").expect("valid period"), 7);
        assert_eq!(parse_period_to_days("1M").expect("valid period"), 30);
        assert_eq!(parse_period_to_days("1Y").expect("valid period"), 365);
    }

    #[test]
    fn test_invalid_tenor() {
        assert!(parse_tenor_to_years("").is_err());
        assert!(parse_tenor_to_years("XYZ").is_err());
        assert!(parse_tenor_to_years("1X").is_err());
    }

    #[test]
    fn test_parse_tenor_with_context() {
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");

        // Without calendar, should still work
        let years = parse_tenor_to_years_with_context(
            "1Y",
            as_of,
            None,
            BusinessDayConvention::ModifiedFollowing,
            DayCount::ActAct,
        )
        .expect("should parse 1Y");
        // 2025 is not a leap year, so should be close to 1.0
        assert!((years - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_parse_tenor_months_with_context() {
        let as_of = Date::from_calendar_date(2025, Month::January, 15).expect("valid date");

        let years = parse_tenor_to_years_with_context(
            "1M",
            as_of,
            None,
            BusinessDayConvention::ModifiedFollowing,
            DayCount::ActAct,
        )
        .expect("should parse 1M");
        // 1M from Jan 15 to Feb 15 = 31 days / 365 ≈ 0.0849
        assert!(years > 0.08 && years < 0.09);
    }

    #[test]
    fn test_parse_tenor_end_of_month() {
        // Jan 31 + 1M should go to Feb 28 in non-leap year
        let as_of = Date::from_calendar_date(2025, Month::January, 31).expect("valid date");

        let years = parse_tenor_to_years_with_context(
            "1M",
            as_of,
            None,
            BusinessDayConvention::ModifiedFollowing,
            DayCount::ActAct,
        )
        .expect("should parse 1M");
        // Jan 31 to Feb 28 = 28 days / 365 ≈ 0.0767
        assert!(years > 0.07 && years < 0.08);
    }
}
