use crate::dates::holiday::rule::{Direction, Rule};
use time::{Month, Weekday};

/// Australian Securities Exchange holiday calendar (code: ASX).
///
/// **Source**: Australian Securities Exchange (ASX) official holiday schedule.
///
/// **Observance Policy**:
/// - Fixed holidays use Monday substitution: New Year, Christmas, Boxing Day
/// - Australia Day: Observed on the Monday on or after January 26th
/// - Anzac Day: Observed on April 25th with no substitution (even if weekend)
/// - Easter holidays: Good Friday and Easter Monday (no substitution needed)
/// - Weekend substitution: Holidays falling on weekends are moved to the following Monday
///
/// **Coverage**: Full year range supported (1970-2150).
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

crate::impl_calendar_generated!(Asx, "asx", ASX_RULES);
