use crate::dates::holiday::calendars::Nyse;
use time::Date;

/// CME calendar mirrors NYSE holidays.
#[derive(Debug, Clone, Copy, Default)]
pub struct Cme;

impl Cme {
    #[inline]
    pub const fn id(self) -> &'static str {
        "cme"
    }
}

impl crate::dates::calendar::HolidayCalendar for Cme {
    fn is_holiday(&self, date: Date) -> bool {
        Nyse.is_holiday(date)
    }
}
