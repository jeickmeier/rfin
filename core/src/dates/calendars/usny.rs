use crate::dates::calendar::HolidayCalendar;
use crate::dates::rules::{FixedDate, NthWeekday, HolidayRule};
use time::{Date, Month, Weekday};

/// U.S. Fedwire / Government Securities settlement calendar (code: USNY).
///
/// Follows the Federal Reserve Board "K.8" holiday schedule with weekend
/// observation (Friday if Saturday, Monday if Sunday) for fixed-date
/// holidays.
#[derive(Debug, Clone, Copy, Default)]
pub struct Usny;

impl Usny {
    /// Convenience constructor (zero-sized type).
    #[inline]
    pub const fn new() -> Self {
        Self
    }
}

impl HolidayCalendar for Usny {
    fn is_holiday(&self, date: Date) -> bool {
        // Fixed-date holidays with weekend observation.
        FixedDate::new(Month::January, 1)  // New Year's Day
            .observed_weekend()
            .applies(date)
            || FixedDate::new(Month::June, 19) // Juneteenth
                .observed_weekend()
                .applies(date)
            || FixedDate::new(Month::July, 4) // Independence Day
                .observed_weekend()
                .applies(date)
            || FixedDate::new(Month::November, 11) // Veterans Day
                .observed_weekend()
                .applies(date)
            || FixedDate::new(Month::December, 25) // Christmas Day
                .observed_weekend()
                .applies(date)
            // Week-day (floating) holidays.
            || NthWeekday::new(3, Weekday::Monday, Month::January)  // Martin Luther King Jr. Day
                .applies(date)
            || NthWeekday::new(3, Weekday::Monday, Month::February) // Presidents' Day
                .applies(date)
            || NthWeekday::last(Weekday::Monday, Month::May)        // Memorial Day
                .applies(date)
            || NthWeekday::first(Weekday::Monday, Month::September) // Labor Day
                .applies(date)
            || NthWeekday::new(2, Weekday::Monday, Month::October)  // Columbus Day
                .applies(date)
            || NthWeekday::new(4, Weekday::Thursday, Month::November) // Thanksgiving Day
                .applies(date)
    }
} 