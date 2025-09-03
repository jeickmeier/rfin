use crate::dates::holiday::rule::Rule;
use time::{Month, Weekday};

/// U.S. Fedwire / Government Securities settlement calendar (code: USNY).
///
/// **Source**: U.S. Federal Reserve Fedwire and Government Securities settlement holiday schedule.
///
/// **Observance Policy**:
/// - Fixed holidays with weekend substitution: New Year, Juneteenth, Independence Day, Veterans Day, Christmas
/// - Floating holidays: MLK Day (3rd Monday January), Presidents Day (3rd Monday February), Memorial Day (last Monday May), Labor Day (1st Monday September), Columbus Day (2nd Monday October), Thanksgiving (4th Thursday November)
/// - Weekend substitution: Holidays falling on weekends are moved to the following Monday
/// - Government operations: Affects Fedwire payments and government securities settlement
///
/// **Coverage**: Full year range supported (1970-2150).
const USNY_RULES: &[Rule] = &[
    // Fixed-date holidays with weekend observation
    Rule::fixed_weekend(Month::January, 1),   // New Year
    Rule::fixed_weekend(Month::June, 19),     // Juneteenth
    Rule::fixed_weekend(Month::July, 4),      // Independence Day
    Rule::fixed_weekend(Month::November, 11), // Veterans Day
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
        n: 2,
        weekday: Weekday::Monday,
        month: Month::October,
    }, // Columbus Day
    Rule::NthWeekday {
        n: 4,
        weekday: Weekday::Thursday,
        month: Month::November,
    }, // Thanksgiving
];

#[derive(Debug, Clone, Copy, Default)]
pub struct Usny;

impl Usny {
    #[inline]
    pub const fn id(self) -> &'static str {
        "usny"
    }
}

crate::impl_calendar_generated!(Usny, "usny", USNY_RULES);
