use crate::dates::calendar::HolidayCalendar;
use crate::dates::rules::{FixedDate, NthWeekday, HolidayRule};
use time::{Date, Month, Weekday};

/// SIFMA recommended U.S. bond-market holiday calendar (code: SIFMA).
/// Based on NYSE holidays plus Columbus Day and Veterans Day (when weekday).
#[derive(Debug, Clone, Copy, Default)]
pub struct Sifma;

impl Sifma {
    #[inline]
    pub const fn new() -> Self {
        Self
    }
}

impl HolidayCalendar for Sifma {
    fn is_holiday(&self, date: Date) -> bool {
        // Reuse NYSE base list first
        if crate::dates::calendars::Nyse::new().is_holiday(date) {
            return true;
        }

        // Columbus Day – 2nd Monday October
        if NthWeekday::new(2, Weekday::Monday, Month::October).applies(date) {
            return true;
        }

        // Veterans Day – observed only if weekday (no substitution)
        if FixedDate::new(Month::November, 11).applies(date)
            && !matches!(date.weekday(), Weekday::Saturday | Weekday::Sunday)
        {
            return true;
        }

        false
    }
} 