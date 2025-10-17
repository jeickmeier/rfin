//! Unified holiday rule enum replacing the previous micro-struct DSL.
#![allow(missing_docs)]
#![allow(clippy::unnecessary_map_or)]
//!
//! This module provides a single `Rule` enum with data-carrying variants that
//! can express the common holiday patterns used across market calendars.
//! Implementing `Rule::applies(&self, date)` once keeps the codebase compact
//! and avoids dozens of tiny helper structs.

use crate::dates::calendar::algo;
use crate::dates::calendar::business_days::HolidayCalendar;
use time::{Date, Duration, Month, Weekday};

// ---------------------------------------------------------------------------
// Supporting enums
// ---------------------------------------------------------------------------

/// Weekend-observance behaviour for fixed-date holidays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum Observed {
    /// No adjustment – holiday is **only** on the calendar date itself.
    None,
    /// If holiday falls on Saturday **or** Sunday, observe on following Monday.
    NextMonday,
    /// If Saturday ⇒ previous Friday, if Sunday ⇒ following Monday.
    FriIfSatMonIfSun,
}

/// Direction selector used by the `WeekdayShift` rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum Direction {
    /// Nearest **on/after** the reference date.
    After,
    /// Nearest **on/before** the reference date.
    Before,
}

// ---------------------------------------------------------------------------
// Rule enum
// ---------------------------------------------------------------------------

/// Single holiday rule covering the common patterns.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum Rule {
    /// Fixed calendar date (e.g. 1-Jan). Optional weekend observation logic.
    Fixed {
        month: Month,
        day: u8,
        observed: Observed,
    },

    /// *n*-th occurrence of `weekday` in `month`.
    ///  * n > 0 ⇒ nth from **start** (1 = first)
    ///  * n < 0 ⇒ nth from **end**   (-1 = last)
    NthWeekday {
        n: i8,
        weekday: Weekday,
        month: Month,
    },

    /// Shift to `weekday` **on/after** or **on/before** the given base date.
    WeekdayShift {
        weekday: Weekday,
        month: Month,
        day: u8,
        dir: Direction,
    },

    /// Relative offset (days) from Easter **Monday** (e.g. ‑3 = Good Friday).
    EasterOffset(i16),

    /// Consecutive multi-day block starting at `start` and spanning `len`
    /// calendar days (including the start day).
    /// Note: This variant cannot be serialized as it contains a static reference.
    /// It is only used in compiled calendar definitions.
    #[cfg_attr(feature = "serde", serde(skip))]
    Span { start: &'static Rule, len: u8 },

    /// Chinese New Year (Spring Festival) – uses generated lookup table (1970-2150).
    ChineseNewYear,

    /// Qing Ming festival (Tomb-Sweeping Day) – Chinese solar term around 4-Apr.
    QingMing,

    /// Buddha's Birthday (8th day of 4th Chinese lunar month – approx CNY+95d).
    BuddhasBirthday,

    /// Vernal Equinox Day per Japanese calendar approximation.
    VernalEquinoxJP,

    /// Autumnal Equinox Day per Japanese calendar approximation.
    AutumnalEquinoxJP,
}

// ---------------------------------------------------------------------------
// Public helper constructors for ergonomics
// ---------------------------------------------------------------------------
impl Rule {
    /// Convenience for `Rule::Fixed { … }` with no observation.
    #[inline]
    pub const fn fixed(month: Month, day: u8) -> Self {
        Rule::Fixed {
            month,
            day,
            observed: Observed::None,
        }
    }

    /// Convenience for fixed date with Monday substitution.
    #[inline]
    pub const fn fixed_next_monday(month: Month, day: u8) -> Self {
        Rule::Fixed {
            month,
            day,
            observed: Observed::NextMonday,
        }
    }

    /// Convenience for US-style Fri/Sat-Mon substitution.
    #[inline]
    pub const fn fixed_weekend(month: Month, day: u8) -> Self {
        Rule::Fixed {
            month,
            day,
            observed: Observed::FriIfSatMonIfSun,
        }
    }
}

// ---------------------------------------------------------------------------
// Internal helpers reused by applies()
// ---------------------------------------------------------------------------

// is_leap_year and add_months provided by shared utils

// Easter Monday is now provided by calendar::algo

// Chinese New Year helpers now provided by calendar::algo

/// Calculate Qing Ming (Tomb-Sweeping Day) based on solar term calculations.
///
/// Qing Ming is one of the 24 solar terms in the traditional Chinese calendar,
/// typically falling around April 4-5 when the sun reaches celestial longitude 15°.
///
/// The formula uses mean solar longitude calculations with epoch 1900.
/// Accurate for years 1900-2100.
///
/// # Constants
/// - Base offset: 5.59 days into April
/// - Slope: 0.2422 days per year (accounts for calendar drift)
/// - Epoch: 1900 (reference year for calculation)
fn qing_ming_day(year: i32) -> u8 {
    const QINGMING_BASE: f64 = 5.59;
    const QINGMING_SLOPE: f64 = 0.2422;
    const QINGMING_EPOCH: i32 = 1900;
    
    let y = (year - QINGMING_EPOCH) as f64;
    (QINGMING_BASE + QINGMING_SLOPE * y - (y / 4.0).floor()) as u8
}

// Helper for Buddha's Birthday approximation (CNY +95 days)
fn buddhas_birthday_date(year: i32) -> Option<Date> {
    algo::cny_date(year).map(|cny| cny + Duration::days(95))
}

/// Calculate Vernal Equinox Day for Japan.
///
/// Uses the formula from Japan's National Astronomical Observatory (NAO)
/// for approximating the date of the vernal (spring) equinox, which is
/// a national holiday in Japan.
///
/// The formula is based on astronomical calculations with epoch 1980.
/// Accurate for years 1900-2100.
///
/// # Constants
/// - Epoch: 1980 (reference year for NAO formula)
/// - Base: 20.8431 days into March
/// - Slope: 0.242194 days per year (accounts for precession)
///
/// # Reference
/// Japan National Astronomical Observatory (国立天文台)
fn vernal_equinox_jp(year: i32) -> Date {
    const VERNAL_EPOCH: i32 = 1980;
    const VERNAL_BASE: f64 = 20.8431;
    const VERNAL_SLOPE: f64 = 0.242194;
    
    let y = (year - VERNAL_EPOCH) as f64;
    let day = (VERNAL_BASE + VERNAL_SLOPE * y - (y / 4.0).floor()).floor() as u8;
    Date::from_calendar_date(year, Month::March, day).unwrap()
}

/// Calculate Autumnal Equinox Day for Japan.
///
/// Uses the formula from Japan's National Astronomical Observatory (NAO)
/// for approximating the date of the autumnal (fall) equinox, which is
/// a national holiday in Japan.
///
/// The formula is based on astronomical calculations with epoch 1980.
/// Accurate for years 1900-2100.
///
/// # Constants
/// - Epoch: 1980 (reference year for NAO formula)
/// - Base: 23.2488 days into September
/// - Slope: 0.242194 days per year (accounts for precession)
///
/// # Reference
/// Japan National Astronomical Observatory (国立天文台)
fn autumnal_equinox_jp(year: i32) -> Date {
    const AUTUMNAL_EPOCH: i32 = 1980;
    const AUTUMNAL_BASE: f64 = 23.2488;
    const AUTUMNAL_SLOPE: f64 = 0.242194;
    
    let y = (year - AUTUMNAL_EPOCH) as f64;
    let day = (AUTUMNAL_BASE + AUTUMNAL_SLOPE * y - (y / 4.0).floor()).floor() as u8;
    Date::from_calendar_date(year, Month::September, day).unwrap()
}

#[inline]
fn apply_observed(mut base: Date, observed: Observed) -> Date {
    match observed {
        Observed::None => {}
        Observed::NextMonday => {
            if matches!(base.weekday(), Weekday::Saturday) {
                base += Duration::days(2);
            } else if matches!(base.weekday(), Weekday::Sunday) {
                base += Duration::days(1);
            }
        }
        Observed::FriIfSatMonIfSun => {
            if matches!(base.weekday(), Weekday::Saturday) {
                base -= Duration::days(1);
            } else if matches!(base.weekday(), Weekday::Sunday) {
                base += Duration::days(1);
            }
        }
    }
    base
}

#[inline]
fn shift_to_weekday(mut d: Date, weekday: Weekday, dir: Direction) -> Date {
    match dir {
        Direction::After => {
            while d.weekday() != weekday {
                d += Duration::days(1);
            }
        }
        Direction::Before => {
            while d.weekday() != weekday {
                d -= Duration::days(1);
            }
        }
    }
    d
}

// ---------------------------------------------------------------------------
// Reusable span materialization helper
// ---------------------------------------------------------------------------
#[inline]
fn push_span_range<A: smallvec::Array<Item = Date>>(
    out: &mut smallvec::SmallVec<A>,
    starts: &[Date],
    len: u8,
) {
    if len == 0 {
        return;
    }
    let span_days = len as i64;
    for &sd in starts {
        for k in 0..span_days {
            out.push(sd + Duration::days(k));
        }
    }
}

// ---------------------------------------------------------------------------
// Core implementation – applies()
// ---------------------------------------------------------------------------
impl Rule {
    /// Returns `true` when the rule marks `date` a holiday.
    #[inline]
    pub fn applies(&self, date: Date) -> bool {
        match self {
            Rule::Fixed {
                month,
                day,
                observed,
            } => {
                let base = apply_observed(
                    Date::from_calendar_date(date.year(), *month, *day).unwrap(),
                    *observed,
                );
                base == date
            }
            Rule::NthWeekday { n, weekday, month } => {
                let target = crate::dates::calendar::generated::nth_weekday_of_month(
                    date.year(),
                    *month,
                    *weekday,
                    *n,
                );
                target == date
            }
            Rule::WeekdayShift {
                weekday,
                month,
                day,
                dir,
            } => {
                let base = Date::from_calendar_date(date.year(), *month, *day).unwrap();
                let d = shift_to_weekday(base, *weekday, *dir);
                d == date
            }
            Rule::EasterOffset(offset) => {
                let easter_mon = algo::easter_monday(date.year());
                let target = easter_mon + Duration::days(*offset as i64);
                target == date
            }
            Rule::Span { start, len } => {
                // Pre-compute start dates for this and previous year, then range-check.
                // Previous year is needed for spans that cross year boundaries.
                let y = date.year();
                let mut starts = smallvec::SmallVec::<[Date; 64]>::new();
                start.materialize_year(y, &mut starts);
                if *len > 1 {
                    start.materialize_year(y - 1, &mut starts);
                }
                let span_days = *len as i64;
                for sd in starts {
                    if date >= sd && date < sd + Duration::days(span_days) {
                        return true;
                    }
                }
                false
            }
            Rule::ChineseNewYear => algo::is_cny(date),
            Rule::QingMing => {
                date.month() == Month::April && date.day() == qing_ming_day(date.year())
            }
            Rule::BuddhasBirthday => {
                buddhas_birthday_date(date.year()).map_or(false, |d| d == date)
            }
            Rule::VernalEquinoxJP => date == vernal_equinox_jp(date.year()),
            Rule::AutumnalEquinoxJP => date == autumnal_equinox_jp(date.year()),
        }
    }
}

impl Rule {
    /// Append all dates in `year` that this rule marks as a holiday into `out`.
    /// No deduplication is performed.
    pub fn materialize_year<A: smallvec::Array<Item = Date>>(
        &self,
        year: i32,
        out: &mut smallvec::SmallVec<A>,
    ) {
        match self {
            Rule::Fixed {
                month,
                day,
                observed,
            } => {
                let base = apply_observed(
                    Date::from_calendar_date(year, *month, *day).unwrap(),
                    *observed,
                );
                out.push(base);
            }
            Rule::NthWeekday { n, weekday, month } => {
                let d = crate::dates::calendar::generated::nth_weekday_of_month(
                    year, *month, *weekday, *n,
                );
                out.push(d);
            }
            Rule::WeekdayShift {
                weekday,
                month,
                day,
                dir,
            } => {
                let base = Date::from_calendar_date(year, *month, *day).unwrap();
                out.push(shift_to_weekday(base, *weekday, *dir));
            }
            Rule::EasterOffset(offset) => {
                let em = algo::easter_monday(year);
                out.push(em + Duration::days(*offset as i64));
            }
            Rule::Span { start, len } => {
                let mut tmp = smallvec::SmallVec::<[Date; 64]>::new();
                start.materialize_year(year, &mut tmp);
                // Also materialize previous year starts for spans that may cross year boundaries
                if *len > 1 {
                    start.materialize_year(year - 1, &mut tmp);
                }
                push_span_range(out, &tmp, *len);
            }
            Rule::ChineseNewYear => {
                if let Some(d) = algo::cny_date(year) {
                    out.push(d);
                }
            }
            Rule::QingMing => {
                out.push(
                    Date::from_calendar_date(year, Month::April, qing_ming_day(year)).unwrap(),
                );
            }
            Rule::BuddhasBirthday => {
                if let Some(d) = buddhas_birthday_date(year) {
                    out.push(d);
                }
            }
            Rule::VernalEquinoxJP => out.push(vernal_equinox_jp(year)),
            Rule::AutumnalEquinoxJP => out.push(autumnal_equinox_jp(year)),
        }
    }
}

// ---------------------------------------------------------------------------
// Implement HolidayCalendar for &[Rule]
// ---------------------------------------------------------------------------
impl HolidayCalendar for &[Rule] {
    fn is_holiday(&self, date: Date) -> bool {
        self.iter().any(|r| r.applies(date))
    }
}

// ---------------------------------------------------------------------------
// Blanket impl for slices of holiday calendars (composite-union semantics)
// (removed: use CompositeCalendar for composition)
// ---------------------------------------------------------------------------
