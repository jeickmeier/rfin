//! Tests for calendar rule implementation

use finstack_core::dates::calendar::rule::{Direction, Observed, Rule};
use finstack_core::dates::{Date, HolidayCalendar};
use time::{Month, Weekday};

fn make_date(y: i32, m: u8, d: u8) -> Date {
    Date::from_calendar_date(y, Month::try_from(m).unwrap(), d).unwrap()
}

#[test]
fn rule_fixed_no_observation() {
    let rule = Rule::fixed(Month::July, 4);

    // Should match exact date in any year
    assert!(rule.applies(make_date(2025, 7, 4)));
    assert!(rule.applies(make_date(2024, 7, 4)));
    assert!(rule.applies(make_date(2026, 7, 4)));

    // Should not match other dates
    assert!(!rule.applies(make_date(2025, 7, 5)));
    assert!(!rule.applies(make_date(2025, 7, 3)));
    assert!(!rule.applies(make_date(2025, 6, 4)));
}

#[test]
fn rule_fixed_next_monday() {
    let rule = Rule::Fixed {
        month: Month::January,
        day: 1,
        observed: Observed::NextMonday,
    };

    // 2023: Jan 1 is Sunday, observed on Monday Jan 2
    assert!(!rule.applies(make_date(2023, 1, 1)));
    assert!(rule.applies(make_date(2023, 1, 2)));

    // 2024: Jan 1 is Monday, observed same day
    assert!(rule.applies(make_date(2024, 1, 1)));

    // 2022: Jan 1 is Saturday, observed on Monday Jan 3
    assert!(!rule.applies(make_date(2022, 1, 1)));
    assert!(rule.applies(make_date(2022, 1, 3)));
}

#[test]
fn rule_fixed_fri_if_sat_mon_if_sun() {
    let rule = Rule::Fixed {
        month: Month::July,
        day: 4,
        observed: Observed::FriIfSatMonIfSun,
    };

    // 2026: July 4 is Saturday, observed on Friday July 3
    assert!(rule.applies(make_date(2026, 7, 3)));
    assert!(!rule.applies(make_date(2026, 7, 4)));

    // 2027: July 4 is Sunday, observed on Monday July 5
    assert!(!rule.applies(make_date(2027, 7, 4)));
    assert!(rule.applies(make_date(2027, 7, 5)));

    // 2025: July 4 is Friday, observed same day
    assert!(rule.applies(make_date(2025, 7, 4)));
}

#[test]
fn rule_nth_weekday_positive() {
    // Third Monday of January
    let rule = Rule::NthWeekday {
        n: 3,
        weekday: Weekday::Monday,
        month: Month::January,
    };

    // 2025: Third Monday is Jan 20
    assert!(rule.applies(make_date(2025, 1, 20)));
    assert!(!rule.applies(make_date(2025, 1, 13))); // Second Monday
    assert!(!rule.applies(make_date(2025, 1, 27))); // Fourth Monday
}

#[test]
fn rule_nth_weekday_negative() {
    // Last Monday of May
    let rule = Rule::NthWeekday {
        n: -1,
        weekday: Weekday::Monday,
        month: Month::May,
    };

    // 2025: Last Monday of May is May 26
    assert!(rule.applies(make_date(2025, 5, 26)));
    assert!(!rule.applies(make_date(2025, 5, 19))); // Second-to-last Monday
}

#[test]
fn rule_weekday_shift_after() {
    // First Monday on or after May 25
    let rule = Rule::WeekdayShift {
        weekday: Weekday::Monday,
        month: Month::May,
        day: 25,
        dir: Direction::After,
    };

    // 2025: May 25 is Sunday, first Monday after is May 26
    assert!(rule.applies(make_date(2025, 5, 26)));
    assert!(!rule.applies(make_date(2025, 5, 25)));
}

#[test]
fn rule_weekday_shift_before() {
    // First Monday on or before May 25
    let rule = Rule::WeekdayShift {
        weekday: Weekday::Monday,
        month: Month::May,
        day: 25,
        dir: Direction::Before,
    };

    // 2025: May 25 is Sunday, first Monday before is May 19
    assert!(rule.applies(make_date(2025, 5, 19)));
    assert!(!rule.applies(make_date(2025, 5, 25)));
    assert!(!rule.applies(make_date(2025, 5, 26)));
}

#[test]
fn rule_easter_offset() {
    // Good Friday is Easter Monday - 3 days
    let rule = Rule::EasterOffset(-3);

    // 2025: Easter Monday is April 21, Good Friday is April 18
    assert!(rule.applies(make_date(2025, 4, 18)));
    assert!(!rule.applies(make_date(2025, 4, 21)));

    // 2024: Easter Monday is April 1, Good Friday is March 29
    assert!(rule.applies(make_date(2024, 3, 29)));
}

#[test]
fn rule_chinese_new_year() {
    let rule = Rule::ChineseNewYear;

    // Known CNY dates
    assert!(rule.applies(make_date(2024, 2, 10)));
    assert!(rule.applies(make_date(2025, 1, 29)));
    assert!(!rule.applies(make_date(2025, 1, 28)));
}

#[test]
fn rule_qing_ming() {
    let rule = Rule::QingMing;

    // Qing Ming is typically around April 4-5
    // 2024: April 4
    assert!(rule.applies(make_date(2024, 4, 4)));

    // 2025: April 4
    assert!(rule.applies(make_date(2025, 4, 4)));
}

#[test]
fn rule_buddhas_birthday() {
    let rule = Rule::BuddhasBirthday;

    // Buddha's Birthday is approximately CNY + 95 days
    // 2024: CNY is Feb 10, so Buddha's Birthday ~ May 15
    let bb_2024 = make_date(2024, 2, 10)
        .checked_add(time::Duration::days(95))
        .unwrap();
    assert!(rule.applies(bb_2024));
}

#[test]
fn rule_slice_as_holiday_calendar() {
    let rules: &[Rule] = &[
        Rule::fixed(Month::January, 1),
        Rule::fixed(Month::December, 25),
    ];

    // Should implement HolidayCalendar trait
    assert!(rules.is_holiday(make_date(2025, 1, 1)));
    assert!(rules.is_holiday(make_date(2025, 12, 25)));
    assert!(!rules.is_holiday(make_date(2025, 7, 4)));
}

#[test]
fn rule_materialize_year_fixed() {
    use smallvec::SmallVec;

    let rule = Rule::fixed(Month::July, 4);
    let mut out = SmallVec::<[Date; 16]>::new();

    rule.materialize_year(2025, &mut out);

    assert_eq!(out.len(), 1);
    assert_eq!(out[0], make_date(2025, 7, 4));
}

#[test]
fn rule_materialize_year_nth_weekday() {
    use smallvec::SmallVec;

    let rule = Rule::NthWeekday {
        n: 4,
        weekday: Weekday::Thursday,
        month: Month::November,
    };
    let mut out = SmallVec::<[Date; 16]>::new();

    rule.materialize_year(2025, &mut out);

    assert_eq!(out.len(), 1);
    // 2025: Fourth Thursday of November is Nov 27
    assert_eq!(out[0], make_date(2025, 11, 27));
}

#[test]
fn rule_materialize_year_easter_offset() {
    use smallvec::SmallVec;

    let rule = Rule::EasterOffset(-3); // Good Friday
    let mut out = SmallVec::<[Date; 16]>::new();

    rule.materialize_year(2025, &mut out);

    assert_eq!(out.len(), 1);
    assert_eq!(out[0], make_date(2025, 4, 18));
}

#[test]
fn rule_convenience_constructors() {
    // Test convenience constructors
    let fixed = Rule::fixed(Month::December, 25);
    assert!(matches!(
        fixed,
        Rule::Fixed {
            observed: Observed::None,
            ..
        }
    ));

    let next_mon = Rule::fixed_next_monday(Month::January, 1);
    assert!(matches!(
        next_mon,
        Rule::Fixed {
            observed: Observed::NextMonday,
            ..
        }
    ));

    let weekend = Rule::fixed_weekend(Month::July, 4);
    assert!(matches!(
        weekend,
        Rule::Fixed {
            observed: Observed::FriIfSatMonIfSun,
            ..
        }
    ));
}

#[test]
fn rule_weekday_shift_on_weekday() {
    // If base date is already the target weekday
    let rule = Rule::WeekdayShift {
        weekday: Weekday::Monday,
        month: Month::January,
        day: 6, // Jan 6, 2025 is already Monday
        dir: Direction::After,
    };

    assert!(rule.applies(make_date(2025, 1, 6)));
}

#[test]
fn rule_multiple_rules_combine() {
    let rules: &[Rule] = &[
        Rule::fixed(Month::January, 1),
        Rule::fixed(Month::July, 4),
        Rule::fixed(Month::December, 25),
        Rule::EasterOffset(-3),
    ];

    // New Year
    assert!(rules.is_holiday(make_date(2025, 1, 1)));

    // Independence Day
    assert!(rules.is_holiday(make_date(2025, 7, 4)));

    // Christmas
    assert!(rules.is_holiday(make_date(2025, 12, 25)));

    // Good Friday 2025
    assert!(rules.is_holiday(make_date(2025, 4, 18)));

    // Regular day
    assert!(!rules.is_holiday(make_date(2025, 6, 15)));
}

#[test]
fn rule_vernal_equinox_jp() {
    let rule = Rule::VernalEquinoxJP;

    // Vernal equinox in Japan is typically around March 20-21
    // Test that it produces a valid date in that range
    for year in 2020..2030 {
        use smallvec::SmallVec;
        let mut out = SmallVec::<[Date; 1]>::new();
        rule.materialize_year(year, &mut out);

        assert_eq!(out.len(), 1);
        let date = out[0];
        assert_eq!(date.month(), Month::March);
        assert!(date.day() >= 19 && date.day() <= 21);
    }
}

#[test]
fn rule_autumnal_equinox_jp() {
    let rule = Rule::AutumnalEquinoxJP;

    // Autumnal equinox in Japan is typically around September 22-23
    for year in 2020..2030 {
        use smallvec::SmallVec;
        let mut out = SmallVec::<[Date; 1]>::new();
        rule.materialize_year(year, &mut out);

        assert_eq!(out.len(), 1);
        let date = out[0];
        assert_eq!(date.month(), Month::September);
        assert!(date.day() >= 22 && date.day() <= 24);
    }
}

#[test]
fn rule_easter_known_dates_2020_2030() {
    // Known Easter Monday dates (authoritative reference from astronomical calculations)
    let known_easter_mondays = [
        (2020, 4, 13),
        (2021, 4, 5),
        (2022, 4, 18),
        (2023, 4, 10),
        (2024, 4, 1),
        (2025, 4, 21),
        (2026, 4, 6),
        (2027, 3, 29),
        (2028, 4, 17),
        (2029, 4, 2),
        (2030, 4, 22),
    ];

    let rule = Rule::EasterOffset(0); // Easter Monday

    for (year, month, day) in known_easter_mondays {
        let expected = make_date(year, month, day);
        assert!(
            rule.applies(expected),
            "Easter Monday {} should be {:?}",
            year,
            expected
        );
    }
}

#[test]
fn rule_good_friday_is_3_days_before_easter_monday() {
    let good_friday = Rule::EasterOffset(-3);
    let easter_monday = Rule::EasterOffset(0);

    for year in 2020..=2030 {
        use smallvec::SmallVec;
        let mut gf_out = SmallVec::<[Date; 1]>::new();
        let mut em_out = SmallVec::<[Date; 1]>::new();

        good_friday.materialize_year(year, &mut gf_out);
        easter_monday.materialize_year(year, &mut em_out);

        assert_eq!(gf_out.len(), 1);
        assert_eq!(em_out.len(), 1);

        let diff = (em_out[0] - gf_out[0]).whole_days();
        assert_eq!(
            diff, 3,
            "Good Friday should be 3 days before Easter Monday in {}",
            year
        );
    }
}

#[test]
fn rule_easter_sunday_is_1_day_before_easter_monday() {
    let easter_sunday = Rule::EasterOffset(-1);
    let easter_monday = Rule::EasterOffset(0);

    for year in 2020..=2030 {
        use smallvec::SmallVec;
        let mut sun_out = SmallVec::<[Date; 1]>::new();
        let mut mon_out = SmallVec::<[Date; 1]>::new();

        easter_sunday.materialize_year(year, &mut sun_out);
        easter_monday.materialize_year(year, &mut mon_out);

        assert_eq!(sun_out.len(), 1);
        assert_eq!(mon_out.len(), 1);

        let diff = (mon_out[0] - sun_out[0]).whole_days();
        assert_eq!(
            diff, 1,
            "Easter Sunday should be 1 day before Easter Monday in {}",
            year
        );

        // Easter Sunday should always be a Sunday
        assert_eq!(
            sun_out[0].weekday(),
            time::Weekday::Sunday,
            "Easter {} should fall on Sunday",
            year
        );
    }
}
