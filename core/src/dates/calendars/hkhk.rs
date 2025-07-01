use crate::dates::calendar::HolidayCalendar;
use crate::dates::rules::{FixedDate, HolidaySpan, ChineseNewYear, QingMing, BuddhasBirthday, HolidayRule};
use time::{Date, Month};

/// Hong Kong banking calendar (code: HKHK).
/// NOTE: Lunar-based holidays like Tuen Ng, Mid-Autumn, Chung Yeung are omitted.
#[derive(Debug, Clone, Copy, Default)]
pub struct Hkhk;

impl HolidayCalendar for Hkhk {
    fn is_holiday(&self, date: Date) -> bool {
        // Fixed-date Gregorian holidays
        FixedDate::new(Month::January, 1).applies(date) // New Year
            || FixedDate::new(Month::May, 1).applies(date) // Labour Day
            || FixedDate::new(Month::July, 1).applies(date) // HKSAR Establishment Day
            || FixedDate::new(Month::October, 1).applies(date) // National Day
            || FixedDate::new(Month::December, 25).applies(date) // Christmas
            || FixedDate::new(Month::December, 26).applies(date) // Boxing
            // Lunar/solar term holidays via helper rules
            || HolidaySpan::new(ChineseNewYear, 3).applies(date) // 3 days CNY
            || QingMing.applies(date)
            || BuddhasBirthday.applies(date)
    }
} 