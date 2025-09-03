use crate::dates::holiday::rule::Rule;
use time::{Month, Weekday};

/// New York Stock Exchange holiday calendar (code: NYSE).
///
/// **Source**: New York Stock Exchange (NYSE) official holiday schedule.
///
/// **Observance Policy**:
/// - Fixed holidays with weekend substitution: New Year, Juneteenth, Independence Day, Christmas
/// - Floating holidays: MLK Day (3rd Monday January), Presidents Day (3rd Monday February), Memorial Day (last Monday May), Labor Day (1st Monday September), Thanksgiving (4th Thursday November)
/// - Easter holidays: Good Friday
/// - Weekend substitution: Holidays falling on weekends are moved to the following Monday
/// - Full-day closures: All listed holidays result in complete market closure
///
/// **Coverage**: Full year range supported (1970-2150).
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
