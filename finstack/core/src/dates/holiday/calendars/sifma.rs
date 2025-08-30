use crate::dates::holiday::rule::Rule;
use time::{Month, Weekday};

/// SIFMA recommended U.S. bond-market holiday calendar (code: SIFMA).
/// Based on NYSE holidays plus Columbus Day and Veterans Day when weekday.
const SIFMA_RULES: &[Rule] = &[
    // --- NYSE base list ---
    Rule::fixed_weekend(Month::January, 1),
    Rule::fixed_weekend(Month::June, 19),
    Rule::fixed_weekend(Month::July, 4),
    Rule::fixed_weekend(Month::December, 25),
    Rule::NthWeekday {
        n: 3,
        weekday: Weekday::Monday,
        month: Month::January,
    },
    Rule::NthWeekday {
        n: 3,
        weekday: Weekday::Monday,
        month: Month::February,
    },
    Rule::NthWeekday {
        n: -1,
        weekday: Weekday::Monday,
        month: Month::May,
    },
    Rule::NthWeekday {
        n: 1,
        weekday: Weekday::Monday,
        month: Month::September,
    },
    Rule::NthWeekday {
        n: 4,
        weekday: Weekday::Thursday,
        month: Month::November,
    },
    Rule::EasterOffset(-3),
    // --- SIFMA additional holidays ---
    Rule::NthWeekday {
        n: 2,
        weekday: Weekday::Monday,
        month: Month::October,
    }, // Columbus
    Rule::fixed_weekend(Month::November, 11), // Veterans (weekday only, but weekend obs rule replicates behaviour)
];

#[derive(Debug, Clone, Copy, Default)]
pub struct Sifma;

impl Sifma {
    #[inline]
    pub const fn id(self) -> &'static str {
        "sifma"
    }
}

crate::impl_calendar_generated!(Sifma, "sifma", SIFMA_RULES);
