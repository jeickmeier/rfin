use crate::dates::calendar::HolidayCalendar;
use crate::dates::rules::{FixedDate, GoodFriday, EasterMonday, HolidayRule};
use time::{Date, Month};

/// German XETRA stock-exchange calendar (code: DEFR).
///
/// Closes on nationwide German public holidays except Ascension Day, Whit Monday
/// and Day of German Unity – these remain open.  No weekend substitution.
#[derive(Debug, Clone, Copy, Default)]
pub struct Defr;

impl Defr {
    #[inline]
    pub const fn new() -> Self {
        Self
    }
}

impl HolidayCalendar for Defr {
    fn is_holiday(&self, date: Date) -> bool {
        FixedDate::new(Month::January, 1).applies(date) // New Year
            || GoodFriday.applies(date)
            || EasterMonday.applies(date)
            || FixedDate::new(Month::May, 1).applies(date) // Labour Day
            || FixedDate::new(Month::December, 25).applies(date) // Christmas
            || FixedDate::new(Month::December, 26).applies(date) // St Stephen's
    }
} 