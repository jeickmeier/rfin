use crate::dates::holiday::rule::Rule;
use time::{Date, Month};

const JAN1: Rule = Rule::fixed(Month::January, 1);
const MAY1: Rule = Rule::fixed(Month::May, 1);
const OCT1: Rule = Rule::fixed(Month::October, 1);
const CNY: Rule = Rule::ChineseNewYear;

/// China inter-bank settlement calendar (code: CNBE).
const CNBE_RULES: &[Rule] = &[
    // New Year – 3 day block starting 1 Jan
    Rule::Span {
        start: &JAN1,
        len: 3,
    },
    // Spring Festival – 7-day block from Chinese New Year
    Rule::Span {
        start: &CNY,
        len: 7,
    },
    // Qing Ming
    Rule::QingMing,
    // Labour Day – 5-day block from 1-May
    Rule::Span {
        start: &MAY1,
        len: 5,
    },
    // National Day – 7-day block from 1-Oct
    Rule::Span {
        start: &OCT1,
        len: 7,
    },
];

#[derive(Debug, Clone, Copy, Default)]
pub struct Cnbe;

impl Cnbe {
    #[inline]
    pub const fn id(self) -> &'static str {
        "cnbe"
    }
}

impl crate::dates::calendar::HolidayCalendar for Cnbe {
    fn is_holiday(&self, date: Date) -> bool {
        CNBE_RULES.is_holiday(date)
    }
}
