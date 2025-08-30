use crate::dates::holiday::rule::{Direction, Rule};
use time::{Month, Weekday};

/// Australia NSW inter-bank calendar (code: AUCE).
const AUCE_RULES: &[Rule] = &[
    // New Year's Day (Mon substitute)
    Rule::fixed_next_monday(Month::January, 1),
    // Australia Day – Monday on/after 26 Jan
    Rule::WeekdayShift {
        weekday: Weekday::Monday,
        month: Month::January,
        day: 26,
        dir: Direction::After,
    },
    // Good Friday / Easter Monday
    Rule::EasterOffset(-3),
    Rule::EasterOffset(0),
    // Anzac Day – 25 Apr (no substitute)
    Rule::fixed(Month::April, 25),
    // King/Queen's Birthday – 2nd Monday June
    Rule::NthWeekday {
        n: 2,
        weekday: Weekday::Monday,
        month: Month::June,
    },
    // NSW Bank Holiday – 1st Monday August
    Rule::NthWeekday {
        n: 1,
        weekday: Weekday::Monday,
        month: Month::August,
    },
    // Labour Day – 1st Monday October
    Rule::NthWeekday {
        n: 1,
        weekday: Weekday::Monday,
        month: Month::October,
    },
    // Christmas / Boxing (Mon substitute)
    Rule::fixed_next_monday(Month::December, 25),
    Rule::fixed_next_monday(Month::December, 26),
];

#[derive(Debug, Clone, Copy, Default)]
pub struct Auce;

impl Auce {
    #[inline]
    pub const fn id(self) -> &'static str {
        "auce"
    }
}

crate::impl_calendar_generated!(Auce, "auce", AUCE_RULES);
