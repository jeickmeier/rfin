use crate::dates::calendar::HolidayCalendar;
use time::Date;

/// Shanghai/Shenzhen exchange calendar (code: SSE).
#[derive(Debug, Clone, Copy, Default)]
pub struct Sse;

impl HolidayCalendar for Sse {
    fn is_holiday(&self, date: Date) -> bool {
        crate::dates::calendar::HolidayCalendar::is_holiday(&crate::dates::calendars::Cnbe, date)
    }
} 