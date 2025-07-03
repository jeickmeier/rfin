use crate::dates::holiday::calendars::Hkhk;
use time::Date;

/// Hong Kong Exchange calendar (code: HKEX) – mirrors HKHK.
#[derive(Debug, Clone, Copy, Default)]
pub struct Hkex;

impl Hkex {
    #[inline]
    pub const fn id(self) -> &'static str {
        "hkex"
    }
}

impl crate::dates::calendar::HolidayCalendar for Hkex {
    fn is_holiday(&self, date: Date) -> bool {
        Hkhk.is_holiday(date)
    }
}
