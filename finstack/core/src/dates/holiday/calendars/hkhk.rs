use crate::dates::calendar::HolidayCalendar;
use crate::dates::holiday::generated::{BASE_YEAR, HKHK_ORDS, HKHK_ORDS_OFFSETS};
use crate::dates::holiday::rule::Rule;
use std::collections::{HashSet};
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
    // Prefer CSV ordinals if available for this year
    if (BASE_YEAR..=2150).contains(&year) {
        let idx = (year - BASE_YEAR) as usize;
        let start = HKHK_ORDS_OFFSETS[idx] as usize;
        let end = HKHK_ORDS_OFFSETS[idx + 1] as usize;
        if start < end {
            let jan1 = Date::from_calendar_date(year, Month::January, 1).unwrap();
            for &doy in &HKHK_ORDS[start..end] {
                let d = jan1 + Duration::days(doy as i64);
                set.insert(d);
            }
            return set;
        }
    }
    // Fallback: generate from rules
    let mut date = Date::from_calendar_date(year, Month::January, 1).unwrap();
    while date.year() == year {
        if HKHK_RULES.is_holiday(date) { set.insert(date); }
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

crate::impl_calendar_generated_from_ords!(Hkhk, "hkhk", HKHK_ORDS, HKHK_ORDS_OFFSETS, HKHK_RULES);
