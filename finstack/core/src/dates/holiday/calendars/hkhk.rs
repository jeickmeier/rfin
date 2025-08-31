use crate::dates::calendar::HolidayCalendar;
use crate::dates::holiday::rule::Rule;
use std::collections::HashSet;
use time::{Date, Duration, Month};

const CNY: Rule = Rule::ChineseNewYear;

/// Hong Kong banking calendar (code: HKHK).
const HKHK_RULES: &[Rule] = &[
    // Fixed-date Gregorian holidays
    Rule::fixed(Month::January, 1),
    Rule::fixed(Month::May, 1),
    Rule::fixed(Month::July, 1),
    Rule::fixed(Month::October, 1),
    Rule::fixed(Month::December, 25),
    Rule::fixed(Month::December, 26),
    // Lunar/solar term holidays
    Rule::Span {
        start: &CNY,
        len: 3,
    },
    Rule::QingMing,
    Rule::BuddhasBirthday,
];

#[allow(dead_code)]
fn build_year(year: i32) -> HashSet<Date> {
    let mut set: HashSet<Date> = HashSet::new();
    // Generate from rules
    let mut date = Date::from_calendar_date(year, Month::January, 1).unwrap();
    while date.year() == year {
        if HKHK_RULES.is_holiday(date) {
            set.insert(date);
        }
        date += Duration::days(1);
    }
    set
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Hkhk;

impl Hkhk {
    #[inline]
    pub const fn id(self) -> &'static str {
        "hkhk"
    }
}

crate::impl_calendar_generated!(Hkhk, "hkhk", HKHK_RULES);
