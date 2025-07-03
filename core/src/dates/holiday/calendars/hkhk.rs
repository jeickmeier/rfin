use crate::dates::holiday::rule::Rule;
use time::{Date, Month};

const CNY: Rule = Rule::ChineseNewYear;

/// Hong Kong banking calendar (code: HKHK).
const HKHK_RULES: &[Rule] = &[
    // Fixed-date Gregorian holidays
    Rule::fixed(Month::January, 1),
    Rule::fixed(Month::May, 1),
    Rule::fixed(Month::July, 1),
    Rule::fixed(Month::October, 1),
    Rule::fixed(Month::December, 25),
    Rule::fixed(Month::December, 26),
    // Lunar/solar term holidays
    Rule::Span {
        start: &CNY,
        len: 3,
    },
    Rule::QingMing,
    Rule::BuddhasBirthday,
];

#[derive(Debug, Clone, Copy, Default)]
pub struct Hkhk;

impl Hkhk {
    #[inline]
    pub const fn id(self) -> &'static str {
        "hkhk"
    }
}

impl crate::dates::calendar::HolidayCalendar for Hkhk {
    fn is_holiday(&self, date: Date) -> bool {
        HKHK_RULES.is_holiday(date)
    }
}
