use crate::dates::holiday::rule::Rule;
use time::{Date, Month};

/// German XETRA stock-exchange calendar (code: DEFR).
const DEFR_RULES: &[Rule] = &[
    Rule::fixed(Month::January, 1),   // New Year
    Rule::EasterOffset(-3),           // Good Friday
    Rule::EasterOffset(0),            // Easter Monday
    Rule::fixed(Month::May, 1),       // Labour Day
    Rule::fixed(Month::December, 25), // Christmas
    Rule::fixed(Month::December, 26), // St. Stephen
];

#[derive(Debug, Clone, Copy, Default)]
pub struct Defr;

impl Defr {
    #[inline]
    pub const fn id(self) -> &'static str {
        "defr"
    }
}

impl crate::dates::calendar::HolidayCalendar for Defr {
    fn is_holiday(&self, date: Date) -> bool {
        DEFR_RULES.is_holiday(date)
    }
}
