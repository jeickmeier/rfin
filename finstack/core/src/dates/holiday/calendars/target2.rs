use crate::dates::holiday::rule::Rule;
use time::{Month};

/// European TARGET2 settlement calendar (ECB).
const TARGET2_RULES: &[Rule] = &[
    Rule::fixed(Month::January, 1),   // New Year's Day
    Rule::EasterOffset(-3),           // Good Friday
    Rule::EasterOffset(0),            // Easter Monday
    Rule::fixed(Month::May, 1),       // Labour Day
    Rule::fixed(Month::December, 25), // Christmas Day
    Rule::fixed(Month::December, 26), // Boxing Day
];

#[derive(Debug, Clone, Copy, Default)]
pub struct Target2;

impl Target2 {
    #[inline]
    pub const fn id(self) -> &'static str {
        "target2"
    }
}

crate::impl_calendar_generated!(Target2, "target2", TARGET2_RULES);
