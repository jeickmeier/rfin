use crate::dates::holiday::rule::Rule;
use time::{Month, Weekday};

/// Japanese banking calendar (code: JPTO).
///
/// **Source**: Japanese banking system holiday schedule for inter-bank operations.
///
/// **Observance Policy**:
/// - Year-end holidays: New Year period (January 1-3) with Monday substitution for January 1st
/// - Happy Monday holidays: Coming of Age Day (2nd Monday January), Marine Day (3rd Monday July), Respect for the Aged Day (3rd Monday September), Sports Day (2nd Monday October)
/// - Fixed holidays with weekend substitution: National Foundation Day, Emperor's Birthday, Showa Day, Constitution Day, Greenery Day, Children's Day, Mountain Day, Culture Day, Labor Thanksgiving Day
/// - Equinox holidays: Vernal Equinox Day, Autumnal Equinox Day (calculated annually)
/// - Weekend substitution: Holidays falling on weekends are moved to the following Monday
///
/// **Coverage**: Full year range supported (1970-2150).
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
