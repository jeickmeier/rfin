use crate::dates::calendar::HolidayCalendar;
use crate::dates::rules::{FixedDate, GoodFriday, EasterMonday, HolidayRule};
use time::{Date, Month};

/// European TARGET2 settlement calendar (ECB).
#[derive(Debug, Clone, Copy, Default)]
pub struct Target2;

impl Target2 {
    /// Creates a new `Target2` calendar (zero-sized type).
    #[inline]
    pub const fn new() -> Self {
        Self
    }
}

impl HolidayCalendar for Target2 {
    fn is_holiday(&self, date: Date) -> bool {
        FixedDate::new(Month::January, 1).applies(date) // New Year's Day
            || GoodFriday.applies(date)                 // Good Friday
            || EasterMonday.applies(date)               // Easter Monday
            || FixedDate::new(Month::May, 1).applies(date) // Labour Day
            || FixedDate::new(Month::December, 25).applies(date) // Christmas Day
            || FixedDate::new(Month::December, 26).applies(date) // Boxing Day
    }
} 