//! Unified holiday rule enum replacing the previous micro-struct DSL.
#![allow(missing_docs, dead_code)]
#![allow(clippy::assign_op_pattern, clippy::unnecessary_map_or)]
//!
//! This module provides a single `Rule` enum with data-carrying variants that
//! can express the common holiday patterns used across market calendars.
//! Implementing `Rule::applies(&self, date)` once keeps the codebase compact
//! and avoids dozens of tiny helper structs.

use crate::dates::calendar::HolidayCalendar;
use time::{Date, Duration, Month, Weekday};

// ---------------------------------------------------------------------------
// Supporting enums
// ---------------------------------------------------------------------------

/// Weekend-observance behaviour for fixed-date holidays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
#[derive(Debug, Clone)]
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
    Span { start: &'static Rule, len: u8 },

    /// Chinese New Year (Spring Festival) – uses pre-computed lookup table.
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

/// Compute Easter Monday using the algorithm from the previous `easter_offset` module.
fn easter_monday(year: i32) -> Date {
    let a = year % 19;
    let b = year / 100;
    let c = year % 100;
    let d = b / 4;
    let e = b % 4;
    let f = (b + 8) / 25;
    let g = (b - f + 1) / 3;
    let h = (19 * a + b - d - g + 15) % 30;
    let i = c / 4;
    let k = c % 4;
    let l = (32 + 2 * e + 2 * i - h - k) % 7;
    let m = (a + 11 * h + 22 * l) / 451;
    let month_num = (h + l - 7 * m + 114) / 31; // 3=March 4=April
    let day = ((h + l - 7 * m + 114) % 31) + 1; // Easter Sunday
    let month = if month_num == 3 {
        Month::March
    } else {
        Month::April
    };
    let easter_sunday = Date::from_calendar_date(year, month, day as u8).unwrap();
    easter_sunday + Duration::DAY // Easter Monday = Sunday +1
}

// Pre-computed Chinese New Year (Spring Festival) Gregorian dates 1990-2100.
// TODO: move into external CSV + PHF in build.rs (future step).
const CNY_DATES: &[(i32, u8, u8)] = &[
    (1990, 1, 27),
    (1991, 2, 15),
    (1992, 2, 4),
    (1993, 1, 23),
    (1994, 2, 10),
    (1995, 1, 31),
    (1996, 2, 19),
    (1997, 2, 7),
    (1998, 1, 28),
    (1999, 2, 16),
    (2000, 2, 5),
    (2001, 1, 24),
    (2002, 2, 12),
    (2003, 2, 1),
    (2004, 1, 22),
    (2005, 2, 9),
    (2006, 1, 29),
    (2007, 2, 18),
    (2008, 2, 7),
    (2009, 1, 26),
    (2010, 2, 14),
    (2011, 2, 3),
    (2012, 1, 23),
    (2013, 2, 10),
    (2014, 1, 31),
    (2015, 2, 19),
    (2016, 2, 8),
    (2017, 1, 28),
    (2018, 2, 16),
    (2019, 2, 5),
    (2020, 1, 25),
    (2021, 2, 12),
    (2022, 2, 1),
    (2023, 1, 22),
    (2024, 2, 10),
    (2025, 1, 29),
    (2026, 2, 17),
    (2027, 2, 6),
    (2028, 1, 26),
    (2029, 2, 13),
    (2030, 2, 3),
    (2031, 1, 23),
    (2032, 2, 11),
    (2033, 1, 31),
    (2034, 2, 19),
    (2035, 2, 8),
    (2036, 1, 28),
    (2037, 2, 15),
    (2038, 2, 4),
    (2039, 1, 24),
    (2040, 2, 12),
    (2041, 2, 1),
    (2042, 1, 22),
    (2043, 2, 10),
    (2044, 1, 30),
    (2045, 2, 17),
    (2046, 2, 6),
    (2047, 1, 26),
    (2048, 2, 14),
    (2049, 2, 2),
    (2050, 1, 23),
    (2051, 2, 11),
    (2052, 1, 31),
    (2053, 2, 19),
    (2054, 2, 8),
    (2055, 1, 28),
    (2056, 2, 15),
    (2057, 2, 5),
    (2058, 1, 24),
    (2059, 2, 12),
    (2060, 2, 2),
    (2061, 1, 21),
    (2062, 2, 9),
    (2063, 1, 29),
    (2064, 2, 17),
    (2065, 2, 5),
    (2066, 1, 26),
    (2067, 2, 14),
    (2068, 2, 3),
    (2069, 1, 23),
    (2070, 2, 11),
    (2071, 1, 31),
    (2072, 2, 19),
    (2073, 2, 7),
    (2074, 1, 27),
    (2075, 2, 15),
    (2076, 2, 5),
    (2077, 1, 24),
    (2078, 2, 12),
    (2079, 2, 2),
    (2080, 1, 22),
    (2081, 2, 9),
    (2082, 1, 29),
    (2083, 2, 17),
    (2084, 2, 6),
    (2085, 1, 26),
    (2086, 2, 14),
    (2087, 2, 3),
    (2088, 1, 24),
    (2089, 2, 10),
    (2090, 1, 30),
    (2091, 2, 18),
    (2092, 2, 7),
    (2093, 1, 27),
    (2094, 2, 15),
    (2095, 2, 5),
    (2096, 1, 25),
    (2097, 2, 12),
    (2098, 2, 1),
    (2099, 1, 22),
    (2100, 2, 9),
];

#[inline]
fn is_cny(date: Date) -> bool {
    CNY_DATES
        .iter()
        .any(|&(y, m, d)| y == date.year() && m == date.month() as u8 && d == date.day())
}

// Add helper to compute Qing Ming day
fn qing_ming_day(year: i32) -> u8 {
    let y = year - 1900;
    (5.59 + 0.2422 * y as f64 - ((y / 4) as f64).floor()) as u8
}

// Helper for Buddha's Birthday approximation (CNY +95 days)
fn buddhas_birthday_date(year: i32) -> Option<Date> {
    CNY_DATES
        .iter()
        .find(|&&(y, _, _)| y == year)
        .and_then(|&(_, m, d)| Date::from_calendar_date(year, Month::try_from(m).ok()?, d).ok())
        .map(|cny| cny + Duration::days(95))
}

fn vernal_equinox_jp(year: i32) -> Date {
    let y = year - 1980;
    let day = (20.8431 + 0.242194 * y as f64 - ((y / 4) as f64).floor()).floor() as u8;
    Date::from_calendar_date(year, Month::March, day).unwrap()
}

fn autumnal_equinox_jp(year: i32) -> Date {
    let y = year - 1980;
    let day = (23.2488 + 0.242194 * y as f64 - ((y / 4) as f64).floor()).floor() as u8;
    Date::from_calendar_date(year, Month::September, day).unwrap()
}

// ---------------------------------------------------------------------------
// Core implementation – applies()
// ---------------------------------------------------------------------------
impl Rule {
    /// Returns `true` when the rule marks `date` a holiday.
    pub fn applies(&self, date: Date) -> bool {
        match self {
            Rule::Fixed {
                month,
                day,
                observed,
            } => {
                let mut base = Date::from_calendar_date(date.year(), *month, *day).unwrap();
                match observed {
                    Observed::None => { /* keep base */ }
                    Observed::NextMonday => {
                        if matches!(base.weekday(), Weekday::Saturday) {
                            base = base + Duration::DAY * 2;
                        } else if matches!(base.weekday(), Weekday::Sunday) {
                            base = base + Duration::DAY;
                        }
                    }
                    Observed::FriIfSatMonIfSun => {
                        if matches!(base.weekday(), Weekday::Saturday) {
                            base = base - Duration::DAY;
                        } else if matches!(base.weekday(), Weekday::Sunday) {
                            base = base + Duration::DAY;
                        }
                    }
                }
                base == date
            }
            Rule::NthWeekday { n, weekday, month } => {
                let target = if *n > 0 {
                    // forward search
                    let mut d = Date::from_calendar_date(date.year(), *month, 1).unwrap();
                    while d.weekday() != *weekday {
                        d = d + Duration::DAY;
                    }
                    d + Duration::weeks((*n as i64) - 1)
                } else {
                    // backward search from last day of month
                    let (ny, nm) = if *month == Month::December {
                        (date.year() + 1, Month::January)
                    } else {
                        (date.year(), Month::try_from(*month as u8 + 1).unwrap())
                    };
                    let mut d = Date::from_calendar_date(ny, nm, 1).unwrap() - Duration::DAY;
                    while d.weekday() != *weekday {
                        d = d - Duration::DAY;
                    }
                    let pos = (-*n) as i64; // 1 = last, 2 = second-last…
                    d - Duration::weeks(pos - 1)
                };
                target == date
            }
            Rule::WeekdayShift {
                weekday,
                month,
                day,
                dir,
            } => {
                let mut d = Date::from_calendar_date(date.year(), *month, *day).unwrap();
                match dir {
                    Direction::After => {
                        while d.weekday() != *weekday {
                            d = d + Duration::DAY;
                        }
                    }
                    Direction::Before => {
                        while d.weekday() != *weekday {
                            d = d - Duration::DAY;
                        }
                    }
                }
                d == date
            }
            Rule::EasterOffset(offset) => {
                let easter_mon = easter_monday(date.year());
                let target = easter_mon + Duration::days(*offset as i64);
                target == date
            }
            Rule::Span { start, len } => {
                for offset in 0..*len as i64 {
                    if start.applies(date - Duration::days(offset)) {
                        return true;
                    }
                }
                false
            }
            Rule::ChineseNewYear => is_cny(date),
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
// ---------------------------------------------------------------------------
impl<'a> HolidayCalendar for &'a [&'a dyn HolidayCalendar] {
    fn is_holiday(&self, date: Date) -> bool {
        self.iter().any(|c| c.is_holiday(date))
    }
}
