//! Cross-year span rule tests

use finstack_core::dates::calendar::{HolidayCalendar, Rule};
use finstack_core::dates::Date;
use time::Month;

// A static start rule for Dec 31 each year
static DEC31: Rule = Rule::fixed(Month::December, 31);

#[test]
fn span_len2_cross_year() {
    let rules: &[Rule] = &[Rule::Span {
        start: &DEC31,
        len: 2,
    }];

    let dec31 = Date::from_calendar_date(2024, Month::December, 31).unwrap();
    let jan01 = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let jan02 = Date::from_calendar_date(2025, Month::January, 2).unwrap();

    assert!(rules.is_holiday(dec31));
    assert!(rules.is_holiday(jan01));
    assert!(!rules.is_holiday(jan02));
}

#[test]
fn span_len3_cross_year() {
    let rules: &[Rule] = &[Rule::Span {
        start: &DEC31,
        len: 3,
    }];

    let dec31 = Date::from_calendar_date(2024, Month::December, 31).unwrap();
    let jan01 = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let jan02 = Date::from_calendar_date(2025, Month::January, 2).unwrap();
    let jan03 = Date::from_calendar_date(2025, Month::January, 3).unwrap();

    assert!(rules.is_holiday(dec31));
    assert!(rules.is_holiday(jan01));
    assert!(rules.is_holiday(jan02));
    assert!(!rules.is_holiday(jan03));
}
