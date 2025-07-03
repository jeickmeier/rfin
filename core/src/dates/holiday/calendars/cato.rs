use crate::dates::holiday::rule::{Direction, Rule};
use time::{Date, Month, Weekday};

/// Canadian banking CAD funds calendar (code: CATO).
const CATO_RULES: &[Rule] = &[
    // Fixed-date holidays with Monday substitution
    Rule::fixed_next_monday(Month::January, 1), // New Year
    Rule::fixed_next_monday(Month::July, 1),    // Canada Day
    Rule::fixed_next_monday(Month::September, 30), // Truth & Reconciliation
    Rule::fixed_next_monday(Month::November, 11), // Remembrance
    Rule::fixed_next_monday(Month::December, 25), // Christmas
    Rule::fixed_next_monday(Month::December, 26), // Boxing
    // Family Day – 3rd Monday Feb
    Rule::NthWeekday {
        n: 3,
        weekday: Weekday::Monday,
        month: Month::February,
    },
    // Good Friday
    Rule::EasterOffset(-3),
    // Victoria Day – Monday on or before 24 May (Mon preceding 25 May)
    Rule::WeekdayShift {
        weekday: Weekday::Monday,
        month: Month::May,
        day: 25,
        dir: Direction::Before,
    },
    // Civic Holiday – 1st Monday Aug
    Rule::NthWeekday {
        n: 1,
        weekday: Weekday::Monday,
        month: Month::August,
    },
    // Labour Day – 1st Monday Sep
    Rule::NthWeekday {
        n: 1,
        weekday: Weekday::Monday,
        month: Month::September,
    },
    // Thanksgiving – 2nd Monday Oct
    Rule::NthWeekday {
        n: 2,
        weekday: Weekday::Monday,
        month: Month::October,
    },
];

#[derive(Debug, Clone, Copy, Default)]
pub struct Cato;

impl Cato {
    #[inline]
    pub const fn id(self) -> &'static str {
        "cato"
    }
}

impl crate::dates::calendar::HolidayCalendar for Cato {
    fn is_holiday(&self, date: Date) -> bool {
        CATO_RULES.is_holiday(date)
    }
}
