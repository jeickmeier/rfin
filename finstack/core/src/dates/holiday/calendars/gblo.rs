use crate::dates::holiday::rule::Rule;
use time::{Date, Month, Weekday};

/// U.K. inter-bank business calendar (code: GBLO).
/// Observes England & Wales Bank Holidays with Monday substitution rule.
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

impl crate::dates::calendar::HolidayCalendar for Gblo {
    fn is_holiday(&self, date: Date) -> bool {
        GBLO_RULES.is_holiday(date)
    }
}
