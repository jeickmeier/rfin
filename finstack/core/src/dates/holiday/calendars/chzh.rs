use crate::dates::holiday::rule::Rule;
use time::{Month};

/// Swiss SIX exchange calendar (code: CHZH).
const CHZH_RULES: &[Rule] = &[
    Rule::fixed(Month::January, 1),
    Rule::fixed(Month::January, 2),
    Rule::EasterOffset(-3), // Good Friday
    Rule::EasterOffset(0),  // Easter Monday
    Rule::fixed(Month::May, 1),
    Rule::EasterOffset(38), // Ascension Thursday (EasterMon+38)
    Rule::EasterOffset(49), // Pentecost Monday
    Rule::fixed(Month::August, 1),
    Rule::fixed(Month::December, 25),
    Rule::fixed(Month::December, 26),
];

#[derive(Debug, Clone, Copy, Default)]
pub struct Chzh;

impl Chzh {
    #[inline]
    pub const fn id(self) -> &'static str {
        "chzh"
    }
}

crate::impl_calendar_generated!(Chzh, "chzh", CHZH_RULES);
