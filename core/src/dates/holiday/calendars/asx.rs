use crate::dates::holiday::rule::{Direction, Rule};
use time::{Date, Month, Weekday};

/// Australian Securities Exchange holiday calendar (code: ASX).
const ASX_RULES: &[Rule] = &[
    // New Year's Day (Mon substitution)
    Rule::fixed_next_monday(Month::January, 1),
    // Australia Day – Monday on/after 26 Jan
    Rule::WeekdayShift {
        weekday: Weekday::Monday,
        month: Month::January,
        day: 26,
        dir: Direction::After,
    },
    // Good Friday
    Rule::EasterOffset(-3),
    // Easter Monday
    Rule::EasterOffset(0),
    // Anzac Day – 25 Apr (no substitution)
    Rule::fixed(Month::April, 25),
    // Christmas / Boxing (Mon substitution)
    Rule::fixed_next_monday(Month::December, 25),
    Rule::fixed_next_monday(Month::December, 26),
];

#[derive(Debug, Clone, Copy, Default)]
pub struct Asx;

impl Asx {
    #[inline]
    pub const fn id(self) -> &'static str {
        "asx"
    }
}

impl crate::dates::calendar::HolidayCalendar for Asx {
    fn is_holiday(&self, date: Date) -> bool {
        ASX_RULES.is_holiday(date)
    }
}
