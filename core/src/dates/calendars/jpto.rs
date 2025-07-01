use crate::dates::calendar::HolidayCalendar;
use crate::dates::rules::*;
use time::{Date, Month, Weekday};

/// Japanese banking calendar (code: JPTO).
///
/// Implementation notes (simplified modern-era rules):
/// * Covers statutory National Holidays and Jan 2‒3 year-end bank holidays.
/// * Fixed-date holidays gain a **substitute Monday** when they fall on
///   a weekend (Sun ⇒ Mon; Sat handling ignored as banks already closed).
/// * Happy-Monday holidays use `NthWeekday` helpers.
/// * Astronomical equinoxes use the approximations from `VernalEquinoxJP`
///   and `AutumnalEquinoxJP` already in `rules`.
/// * "Citizen's-holiday" sandwich days and rare one-off imperial events are
///   NOT included in this basic version.
#[derive(Debug, Clone, Copy, Default)]
pub struct Jpto;

impl HolidayCalendar for Jpto {
    fn is_holiday(&self, date: Date) -> bool {
        // --- Year-end / New-year bank holidays --------------------------------
        FixedDate::new(Month::January, 1).applies(date) // New Year's Day
            || FixedDate::new(Month::January, 2).applies(date) // Bank holiday
            || FixedDate::new(Month::January, 3).applies(date) // Bank holiday

            // Substitute Monday for Jan-1 when on weekend
            || InLieuMonday::new(FixedDate::new(Month::January, 1)).applies(date)

            // --- Happy-Monday holidays ---------------------------------------
            || NthWeekday::new(2, Weekday::Monday, Month::January).applies(date) // Coming-of-Age
            || NthWeekday::new(3, Weekday::Monday, Month::July).applies(date)   // Marine Day
            || NthWeekday::new(3, Weekday::Monday, Month::September).applies(date) // Respect-for-Aged
            || NthWeekday::new(2, Weekday::Monday, Month::October).applies(date) // Sports Day

            // --- Fixed-date holidays with Monday substitution ---------------
            || InLieuMonday::new(FixedDate::new(Month::February, 11)).applies(date) // National Foundation
            || InLieuMonday::new(FixedDate::new(Month::February, 23)).applies(date) // Emperor's Birthday (since 2020)
            || InLieuMonday::new(FixedDate::new(Month::April, 29)).applies(date) // Showa Day
            || InLieuMonday::new(FixedDate::new(Month::May, 3)).applies(date) // Constitution Mem.
            || InLieuMonday::new(FixedDate::new(Month::May, 4)).applies(date) // Greenery Day
            || InLieuMonday::new(FixedDate::new(Month::May, 5)).applies(date) // Children's Day
            || InLieuMonday::new(FixedDate::new(Month::August, 11)).applies(date) // Mountain Day
            || InLieuMonday::new(FixedDate::new(Month::November, 3)).applies(date) // Culture Day
            || InLieuMonday::new(FixedDate::new(Month::November, 23)).applies(date) // Labour Thanks.

            // --- Equinoxes ---------------------------------------------------
            || VernalEquinoxJP.applies(date)
            || AutumnalEquinoxJP.applies(date)
    }
} 