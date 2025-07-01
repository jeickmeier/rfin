use crate::dates::calendar::HolidayCalendar;
use crate::dates::rules::{FixedDate, HolidayRule};
use time::{Date, Month};

use crate::dates::calendars::Jpto;

/// Tokyo/Osaka exchange calendar (code: JPX).
/// Follows JPTO plus 31 December market holiday.
#[derive(Debug, Clone, Copy, Default)]
pub struct Jpx;

impl HolidayCalendar for Jpx {
    fn is_holiday(&self, date: Date) -> bool {
        if crate::dates::calendar::HolidayCalendar::is_holiday(&Jpto, date) {
            return true;
        }
        // Additional JPX market holiday: 31-Dec (no substitution)
        FixedDate::new(Month::December, 31).applies(date)
    }
} 