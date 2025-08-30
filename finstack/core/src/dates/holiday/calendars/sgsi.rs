use crate::dates::holiday::rule::Rule;
use time::{Month};

const CNY: Rule = Rule::ChineseNewYear;

/// Singapore inter-bank calendar (code: SGSI).
const SGSI_RULES: &[Rule] = &[
    // Fixed Gregorian holidays with Monday substitution
    Rule::fixed_next_monday(Month::January, 1), // New Year
    Rule::fixed_next_monday(Month::May, 1),     // Labour Day
    Rule::fixed_next_monday(Month::August, 9),  // National Day
    Rule::fixed_next_monday(Month::December, 25), // Christmas
    // Chinese New Year – first 2 days
    Rule::Span {
        start: &CNY,
        len: 2,
    },
    // Good Friday
    Rule::EasterOffset(-3),
];

#[derive(Debug, Clone, Copy, Default)]
pub struct Sgsi;

impl Sgsi {
    #[inline]
    pub const fn id(self) -> &'static str {
        "sgsi"
    }
}

crate::impl_calendar_generated!(Sgsi, "sgsi", SGSI_RULES);
