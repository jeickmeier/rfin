use crate::dates::holiday::calendars::Cnbe;
use time::Date;

/// Shanghai/Shenzhen exchange calendar (code: SSE) – mirrors CNBE.
#[derive(Debug, Clone, Copy, Default)]
pub struct Sse;

impl Sse {
    #[inline]
    pub const fn id(self) -> &'static str {
        "sse"
    }
}

impl crate::dates::calendar::HolidayCalendar for Sse {
    fn is_holiday(&self, date: Date) -> bool {
        Cnbe.is_holiday(date)
    }
}
