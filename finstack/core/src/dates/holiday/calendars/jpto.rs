use crate::dates::holiday::rule::Rule;
use time::{Month, Weekday};

/// Japanese banking calendar (code: JPTO) – simplified modern-era rules.
const JPTO_RULES: &[Rule] = &[
    // Year-end bank holidays
    Rule::fixed(Month::January, 1),
    Rule::fixed(Month::January, 2),
    Rule::fixed(Month::January, 3),
    // Substitute Monday for Jan-1 when weekend
    Rule::fixed_next_monday(Month::January, 1),
    // Happy Monday holidays
    Rule::NthWeekday {
        n: 2,
        weekday: Weekday::Monday,
        month: Month::January,
    }, // Coming of Age
    Rule::NthWeekday {
        n: 3,
        weekday: Weekday::Monday,
        month: Month::July,
    }, // Marine Day
    Rule::NthWeekday {
        n: 3,
        weekday: Weekday::Monday,
        month: Month::September,
    }, // Respect Aged
    Rule::NthWeekday {
        n: 2,
        weekday: Weekday::Monday,
        month: Month::October,
    }, // Sports Day
    // Fixed-date with In-lieu Monday (approx using weekend rule)
    Rule::fixed_weekend(Month::February, 11),
    Rule::fixed_weekend(Month::February, 23),
    Rule::fixed_weekend(Month::April, 29),
    Rule::fixed_weekend(Month::May, 3),
    Rule::fixed_weekend(Month::May, 4),
    Rule::fixed_weekend(Month::May, 5),
    Rule::fixed_weekend(Month::August, 11),
    Rule::fixed_weekend(Month::November, 3),
    Rule::fixed_weekend(Month::November, 23),
    // Equinoxes
    Rule::VernalEquinoxJP,
    Rule::AutumnalEquinoxJP,
];

#[derive(Debug, Clone, Copy, Default)]
pub struct Jpto;

impl Jpto {
    #[inline]
    pub const fn id(self) -> &'static str {
        "jpto"
    }
}

crate::impl_calendar_generated!(Jpto, "jpto", JPTO_RULES);
