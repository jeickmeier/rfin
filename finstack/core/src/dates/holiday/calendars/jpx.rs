use crate::dates::holiday::calendars::Jpto;
use crate::dates::holiday::rule::Rule;
use time::{Date, Month};

/// Tokyo/Osaka exchange calendar (code: JPX).
///
/// **Source**: Tokyo Stock Exchange (TSE) and Osaka Exchange holiday schedule.
///
/// **Observance Policy**:
/// - Extends JPTO banking calendar with additional market-specific holiday
/// - Additional holiday: December 31st (year-end market closure)
/// - All JPTO holidays: Year-end holidays, Happy Monday holidays, fixed holidays with weekend substitution, equinox holidays
/// - Weekend substitution: Holidays falling on weekends are moved to the following Monday
///
/// **Coverage**: Full year range supported (1970-2150).
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
