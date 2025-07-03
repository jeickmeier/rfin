use crate::dates::holiday::calendars::Jpto;
use crate::dates::holiday::rule::Rule;
use time::{Date, Month};

/// Tokyo/Osaka exchange calendar (code: JPX) – JPTO plus 31-Dec market holiday.
const JPX_EXTRA: &[Rule] = &[Rule::fixed(Month::December, 31)];

#[derive(Debug, Clone, Copy, Default)]
pub struct Jpx;

impl Jpx {
    #[inline]
    pub const fn id(self) -> &'static str {
        "jpx"
    }
}

impl crate::dates::calendar::HolidayCalendar for Jpx {
    fn is_holiday(&self, date: Date) -> bool {
        if JPX_EXTRA.is_holiday(date) {
            return true;
        }
        Jpto.is_holiday(date)
    }
}
