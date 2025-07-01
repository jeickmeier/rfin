use crate::dates::calendar::HolidayCalendar;
use crate::dates::rules::{FixedDate, GoodFriday, EasterMonday, AscensionThursday, PentecostMonday, HolidayRule};
use time::{Date, Month};

/// Swiss SIX exchange calendar (code: CHZH).
#[derive(Debug, Clone, Copy, Default)]
pub struct Chzh;

impl HolidayCalendar for Chzh {
    fn is_holiday(&self, date: Date) -> bool {
        FixedDate::new(Month::January, 1).applies(date) // New Year
            || FixedDate::new(Month::January, 2).applies(date) // Berchtoldstag
            || GoodFriday.applies(date)
            || EasterMonday.applies(date)
            || FixedDate::new(Month::May, 1).applies(date) // Labour
            || AscensionThursday.applies(date)
            || PentecostMonday.applies(date)
            || FixedDate::new(Month::August, 1).applies(date) // National Day
            || FixedDate::new(Month::December, 25).applies(date) // Christmas
            || FixedDate::new(Month::December, 26).applies(date) // St Stephen
    }
} 