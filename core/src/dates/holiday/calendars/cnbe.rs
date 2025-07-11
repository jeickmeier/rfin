use crate::dates::calendar::HolidayCalendar;
use crate::dates::holiday::rule::Rule;
use once_cell::sync::Lazy;
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use time::{Date, Duration, Month};

const JAN1: Rule = Rule::fixed(Month::January, 1);
const MAY1: Rule = Rule::fixed(Month::May, 1);
const OCT1: Rule = Rule::fixed(Month::October, 1);
const CNY: Rule = Rule::ChineseNewYear;

/// China inter-bank settlement calendar (code: CNBE).
const CNBE_RULES: &[Rule] = &[
    // New Year – 3 day block starting 1 Jan
    Rule::Span {
        start: &JAN1,
        len: 3,
    },
    // Spring Festival – 7-day block from Chinese New Year
    Rule::Span {
        start: &CNY,
        len: 7,
    },
    // Qing Ming
    Rule::QingMing,
    // Labour Day – 5-day block from 1-May
    Rule::Span {
        start: &MAY1,
        len: 5,
    },
    // National Day – 7-day block from 1-Oct
    Rule::Span {
        start: &OCT1,
        len: 7,
    },
];

static CNBE_CACHE: Lazy<Mutex<HashMap<i32, HashSet<Date>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

fn build_year(year: i32) -> HashSet<Date> {
    let mut set: HashSet<Date> = HashSet::new();
    // Iterate through all days of the year and collect holidays once.
    let mut date = Date::from_calendar_date(year, Month::January, 1).unwrap();
    while date.year() == year {
        if CNBE_RULES.is_holiday(date) {
            set.insert(date);
        }
        date += Duration::DAY;
    }
    set
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Cnbe;

impl Cnbe {
    #[inline]
    pub const fn id(self) -> &'static str {
        "cnbe"
    }
}

impl HolidayCalendar for Cnbe {
    fn is_holiday(&self, date: Date) -> bool {
        let year = date.year();
        let mut map = CNBE_CACHE.lock().unwrap();
        let set = map.entry(year).or_insert_with(|| build_year(year));
        set.contains(&date)
    }
}
