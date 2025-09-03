use crate::dates::holiday::rule::Rule;
use time::Month;

/// Swiss SIX exchange calendar (code: CHZH).
///
/// **Source**: Swiss SIX (Swiss Exchange) official holiday schedule.
///
/// **Observance Policy**:
/// - Fixed holidays: New Year (2 days), Labour Day, Swiss National Day, Christmas (2 days)
/// - Easter holidays: Good Friday, Easter Monday, Ascension Thursday, Pentecost Monday
/// - No weekend substitution: Holidays are observed on their actual dates regardless of day of week
/// - Multi-day blocks: New Year and Christmas periods include consecutive days
///
/// **Coverage**: Full year range supported (1970-2150).
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
