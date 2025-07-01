use crate::dates::calendar::HolidayCalendar;
use crate::dates::rules::{FixedDate, NthWeekday, GoodFriday, EasterMonday, HolidayRule};
use time::{Date, Month, Weekday};

/// U.K. inter-bank business calendar (code: GBLO).
/// Observes England & Wales Bank Holidays with Monday substitution rule.
#[derive(Debug, Clone, Copy, Default)]
pub struct Gblo;

impl Gblo {
    #[inline]
    pub const fn new() -> Self {
        Self
    }
}

impl HolidayCalendar for Gblo {
    fn is_holiday(&self, date: Date) -> bool {
        // Fixed-date holidays with Monday substitution
        FixedDate::new(Month::January, 1).observed_next_monday().applies(date)
            || FixedDate::new(Month::December, 25).observed_next_monday().applies(date)
            || FixedDate::new(Month::December, 26).observed_next_monday().applies(date)
            // Good Friday / Easter Monday
            || GoodFriday.applies(date)
            || EasterMonday.applies(date)
            // Early May BH – first Monday May
            || NthWeekday::first(Weekday::Monday, Month::May).applies(date)
            // Spring BH – last Monday May
            || NthWeekday::last(Weekday::Monday, Month::May).applies(date)
            // Summer BH – last Monday Aug
            || NthWeekday::last(Weekday::Monday, Month::August).applies(date)
    }
} 