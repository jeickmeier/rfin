use crate::dates::calendar::HolidayCalendar;
use time::Date;

/// Hong Kong Exchange calendar (code: HKEX).
#[derive(Debug, Clone, Copy, Default)]
pub struct Hkex;

impl HolidayCalendar for Hkex {
    fn is_holiday(&self, date: Date) -> bool {
        crate::dates::calendar::HolidayCalendar::is_holiday(&crate::dates::calendars::Hkhk, date)
    }
} 