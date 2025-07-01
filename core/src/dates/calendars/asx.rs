use crate::dates::calendar::HolidayCalendar;
use crate::dates::rules::{FixedDate, WeekdayShift, GoodFriday, EasterMonday, HolidayRule};
use time::{Date, Month, Weekday};

/// Australian Securities Exchange holiday calendar (code: ASX).
/// Excludes NSW Bank Holiday and Labour Day.
#[derive(Debug, Clone, Copy, Default)]
pub struct Asx;

impl HolidayCalendar for Asx {
    fn is_holiday(&self, date: Date) -> bool {
        // New Year's Day (Mon substitution)
        FixedDate::new(Month::January, 1).observed_next_monday().applies(date)
            // Australia Day – Monday on/after 26 Jan
            || WeekdayShift::on_or_after(Weekday::Monday, Month::January, 26).applies(date)
            // Good Friday / Easter Monday
            || GoodFriday.applies(date)
            || EasterMonday.applies(date)
            // Anzac Day – 25 Apr (no substitution)
            || FixedDate::new(Month::April, 25).applies(date)
            // Christmas / Boxing (Mon substitution)
            || FixedDate::new(Month::December, 25).observed_next_monday().applies(date)
            || FixedDate::new(Month::December, 26).observed_next_monday().applies(date)
    }
} 