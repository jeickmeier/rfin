use crate::dates::holiday::rule::Rule;
use time::{Month, Weekday};

/// U.K. inter-bank business calendar (code: GBLO).
///
/// **Source**: U.K. inter-bank market holiday schedule for England & Wales.
///
/// **Observance Policy**:
/// - Fixed holidays use Monday substitution: New Year, Christmas, Boxing Day
/// - Easter holidays: Good Friday, Easter Monday (no substitution needed)
/// - Floating holidays: Early May Bank Holiday (1st Monday May), Spring Bank Holiday (last Monday May), Summer Bank Holiday (last Monday August)
/// - Weekend substitution: Holidays falling on weekends are moved to the following Monday
/// - England & Wales specific: Does not include Scotland or Northern Ireland specific holidays
///
/// **Coverage**: Full year range supported (1970-2150).
const GBLO_RULES: &[Rule] = &[
    // Fixed-date holidays with Monday substitution
    Rule::fixed_next_monday(Month::January, 1), // New Year
    Rule::fixed_next_monday(Month::December, 25), // Christmas
    Rule::fixed_next_monday(Month::December, 26), // Boxing
    // Good Friday / Easter Monday
    Rule::EasterOffset(-3), // Good Friday
    Rule::EasterOffset(0),  // Easter Monday (offset 0 from Easter Monday)
    // Early May Bank Holiday – first Monday May
    Rule::NthWeekday {
        n: 1,
        weekday: Weekday::Monday,
        month: Month::May,
    },
    // Spring Bank Holiday – last Monday May
    Rule::NthWeekday {
        n: -1,
        weekday: Weekday::Monday,
        month: Month::May,
    },
    // Summer Bank Holiday – last Monday August
    Rule::NthWeekday {
        n: -1,
        weekday: Weekday::Monday,
        month: Month::August,
    },
];

/// Marker struct for GBLO.
#[derive(Debug, Clone, Copy, Default)]
pub struct Gblo;

impl Gblo {
    #[inline]
    pub const fn id(self) -> &'static str {
        "gblo"
    }
}

crate::impl_calendar_generated!(Gblo, "gblo", GBLO_RULES);
