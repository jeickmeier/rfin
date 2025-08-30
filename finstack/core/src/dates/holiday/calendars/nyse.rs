use crate::dates::holiday::rule::Rule;
use time::{Month, Weekday};

/// New York Stock Exchange full-day holiday calendar (code: NYSE).
const NYSE_RULES: &[Rule] = &[
    // Fixed-date holidays with Fri/Sat-Mon observation
    Rule::fixed_weekend(Month::January, 1),   // New Year
    Rule::fixed_weekend(Month::June, 19),     // Juneteenth
    Rule::fixed_weekend(Month::July, 4),      // Independence Day
    Rule::fixed_weekend(Month::December, 25), // Christmas
    // Floating weekday holidays
    Rule::NthWeekday {
        n: 3,
        weekday: Weekday::Monday,
        month: Month::January,
    }, // MLK
    Rule::NthWeekday {
        n: 3,
        weekday: Weekday::Monday,
        month: Month::February,
    }, // Presidents
    Rule::NthWeekday {
        n: -1,
        weekday: Weekday::Monday,
        month: Month::May,
    }, // Memorial
    Rule::NthWeekday {
        n: 1,
        weekday: Weekday::Monday,
        month: Month::September,
    }, // Labor
    Rule::NthWeekday {
        n: 4,
        weekday: Weekday::Thursday,
        month: Month::November,
    }, // Thanksgiving
    // Good Friday
    Rule::EasterOffset(-3),
];

#[derive(Debug, Clone, Copy, Default)]
pub struct Nyse;

impl Nyse {
    #[inline]
    pub const fn id(self) -> &'static str {
        "nyse"
    }
}

crate::impl_calendar_generated!(Nyse, "nyse", NYSE_RULES);
