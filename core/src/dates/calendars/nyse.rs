use crate::dates::calendar::HolidayCalendar;
use crate::dates::rules::{FixedDate, NthWeekday, GoodFriday, HolidayRule};
use time::{Date, Month, Weekday};

/// New York Stock Exchange full-day holiday calendar (code: NYSE).
///
/// Covers all U.S. federal holidays except Columbus Day & Veterans Day,
/// plus Good Friday.  Fixed-date holidays observe Fri/Monday rule.
#[derive(Debug, Clone, Copy, Default)]
pub struct Nyse;

impl Nyse {
    #[inline]
    pub const fn new() -> Self {
        Self
    }
}

impl HolidayCalendar for Nyse {
    fn is_holiday(&self, date: Date) -> bool {
        // Fixed-date holidays
        FixedDate::new(Month::January, 1).observed_weekend().applies(date) // New Year
            || FixedDate::new(Month::June, 19).observed_weekend().applies(date) // Juneteenth
            || FixedDate::new(Month::July, 4).observed_weekend().applies(date) // Independence
            || FixedDate::new(Month::December, 25).observed_weekend().applies(date) // Christmas
            // Floating weekdays
            || NthWeekday::new(3, Weekday::Monday, Month::January).applies(date) // MLK
            || NthWeekday::new(3, Weekday::Monday, Month::February).applies(date) // Presidents
            || NthWeekday::last(Weekday::Monday, Month::May).applies(date) // Memorial
            || NthWeekday::first(Weekday::Monday, Month::September).applies(date) // Labor
            || NthWeekday::new(4, Weekday::Thursday, Month::November).applies(date) // Thanksgiving
            // Good Friday
            || GoodFriday.applies(date)
    }
} 