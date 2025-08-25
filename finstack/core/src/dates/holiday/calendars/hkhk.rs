use crate::dates::calendar::HolidayCalendar;
use crate::dates::holiday::rule::Rule;
use once_cell::sync::Lazy;
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
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

static HKHK_CACHE: Lazy<Mutex<HashMap<i32, HashSet<Date>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

fn build_year(year: i32) -> HashSet<Date> {
    let mut set: HashSet<Date> = HashSet::new();
    let mut date = Date::from_calendar_date(year, Month::January, 1).unwrap();
    while date.year() == year {
        if HKHK_RULES.is_holiday(date) {
            set.insert(date);
        }
        date += Duration::DAY;
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

impl HolidayCalendar for Hkhk {
    fn is_holiday(&self, date: Date) -> bool {
        let year = date.year();
        let mut map = HKHK_CACHE.lock().unwrap();
        let set = map.entry(year).or_insert_with(|| build_year(year));
        set.contains(&date)
    }
}
