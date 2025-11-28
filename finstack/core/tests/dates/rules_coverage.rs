//! Additional coverage tests for calendar rules
//!
//! This module tests edge cases and less commonly used code paths

use finstack_core::dates::calendar::rule::{Direction, Observed, Rule};
use finstack_core::dates::Date;
use smallvec::SmallVec;
use time::{Month, Weekday};

fn make_date(y: i32, m: u8, d: u8) -> Date {
    Date::from_calendar_date(y, Month::try_from(m).unwrap(), d).unwrap()
}

// ============================================
// Span Rule Tests
// ============================================

#[test]
fn rule_span_basic() {
    static START_RULE: Rule = Rule::Fixed {
        month: Month::December,
        day: 24,
        observed: Observed::None,
    };

    let span = Rule::Span {
        start: &START_RULE,
        len: 3,
    };

    // Dec 24, 25, 26 should all be holidays
    assert!(span.applies(make_date(2025, 12, 24)));
    assert!(span.applies(make_date(2025, 12, 25)));
    assert!(span.applies(make_date(2025, 12, 26)));

    // Dec 23 and 27 should not be
    assert!(!span.applies(make_date(2025, 12, 23)));
    assert!(!span.applies(make_date(2025, 12, 27)));
}

#[test]
fn rule_span_crossing_year_boundary() {
    static START_RULE: Rule = Rule::Fixed {
        month: Month::December,
        day: 30,
        observed: Observed::None,
    };

    let span = Rule::Span {
        start: &START_RULE,
        len: 5,
    };

    // Dec 30, 31 of 2024, and Jan 1, 2, 3 of 2025
    assert!(span.applies(make_date(2024, 12, 30)));
    assert!(span.applies(make_date(2024, 12, 31)));
    assert!(span.applies(make_date(2025, 1, 1)));
    assert!(span.applies(make_date(2025, 1, 2)));
    assert!(span.applies(make_date(2025, 1, 3)));
    assert!(!span.applies(make_date(2025, 1, 4)));
}

#[test]
fn rule_span_zero_length() {
    static START_RULE: Rule = Rule::Fixed {
        month: Month::January,
        day: 1,
        observed: Observed::None,
    };

    let span = Rule::Span {
        start: &START_RULE,
        len: 0,
    };

    // Zero-length span should not match any date
    assert!(!span.applies(make_date(2025, 1, 1)));
    assert!(!span.applies(make_date(2025, 1, 2)));
}

#[test]
fn rule_span_single_day() {
    static START_RULE: Rule = Rule::Fixed {
        month: Month::January,
        day: 1,
        observed: Observed::None,
    };

    let span = Rule::Span {
        start: &START_RULE,
        len: 1,
    };

    // Single-day span should match only the start date
    assert!(span.applies(make_date(2025, 1, 1)));
    assert!(!span.applies(make_date(2025, 1, 2)));
}

#[test]
fn rule_span_materialize_year() {
    static START_RULE: Rule = Rule::Fixed {
        month: Month::April,
        day: 29,
        observed: Observed::None,
    };

    let span = Rule::Span {
        start: &START_RULE,
        len: 5,
    };

    let mut out = SmallVec::<[Date; 32]>::new();
    span.materialize_year(2025, &mut out);

    // Implementation also materializes previous year for spans > 1 day
    // So we get both 2024 and 2025 spans (10 dates total)
    // Just verify the 2025 dates are included
    assert!(out.contains(&make_date(2025, 4, 29)));
    assert!(out.contains(&make_date(2025, 4, 30)));
    assert!(out.contains(&make_date(2025, 5, 1)));
    assert!(out.contains(&make_date(2025, 5, 2)));
    assert!(out.contains(&make_date(2025, 5, 3)));
}

// ============================================
// Japanese Equinox Tests
// ============================================

#[test]
fn rule_vernal_equinox_jp_applies() {
    let rule = Rule::VernalEquinoxJP;

    // Test specific known dates for vernal equinox
    // 2024: March 20
    assert!(rule.applies(make_date(2024, 3, 20)));

    // Non-equinox dates should not match
    assert!(!rule.applies(make_date(2024, 3, 19)));
    assert!(!rule.applies(make_date(2024, 3, 22)));
}

#[test]
fn rule_autumnal_equinox_jp_applies() {
    let rule = Rule::AutumnalEquinoxJP;

    // Test that it applies for various years
    for year in 2020..2030 {
        let mut out = SmallVec::<[Date; 1]>::new();
        rule.materialize_year(year, &mut out);
        let eq_date = out[0];

        // The rule should apply to its computed date
        assert!(
            rule.applies(eq_date),
            "Autumnal equinox {} should apply to its computed date",
            year
        );

        // Days before/after should not match
        let prev_day = eq_date - time::Duration::days(1);
        let next_day = eq_date + time::Duration::days(1);
        assert!(!rule.applies(prev_day));
        assert!(!rule.applies(next_day));
    }
}

// ============================================
// Buddha's Birthday Tests
// ============================================

#[test]
fn rule_buddhas_birthday_materialize() {
    let rule = Rule::BuddhasBirthday;

    for year in 2020..2030 {
        let mut out = SmallVec::<[Date; 1]>::new();
        rule.materialize_year(year, &mut out);

        // Should produce exactly one date
        assert_eq!(out.len(), 1, "Buddha's Birthday should produce 1 date for year {}", year);

        // Buddha's Birthday is CNY + 95 days, so it falls in April-June range
        let date = out[0];
        assert!(
            date.month() == Month::April || date.month() == Month::May || date.month() == Month::June,
            "Buddha's Birthday should be in April, May or June, got {:?}",
            date
        );
    }
}

#[test]
fn rule_buddhas_birthday_applies() {
    let rule = Rule::BuddhasBirthday;

    // Get the computed date for 2025
    let mut out = SmallVec::<[Date; 1]>::new();
    rule.materialize_year(2025, &mut out);
    let bb_date = out[0];

    // Should apply to computed date
    assert!(rule.applies(bb_date));

    // Should not apply to adjacent dates
    let prev = bb_date - time::Duration::days(1);
    let next = bb_date + time::Duration::days(1);
    assert!(!rule.applies(prev));
    assert!(!rule.applies(next));
}

// ============================================
// Qing Ming Tests
// ============================================

#[test]
fn rule_qing_ming_materialize() {
    let rule = Rule::QingMing;

    for year in 2020..2030 {
        let mut out = SmallVec::<[Date; 1]>::new();
        rule.materialize_year(year, &mut out);

        // Should produce exactly one date
        assert_eq!(out.len(), 1);

        // Should be in April
        let date = out[0];
        assert_eq!(date.month(), Month::April);

        // Should be between 4th and 6th
        assert!(
            date.day() >= 4 && date.day() <= 6,
            "Qing Ming should be April 4-6, got {:?}",
            date
        );
    }
}

// ============================================
// Chinese New Year Tests
// ============================================

#[test]
fn rule_chinese_new_year_materialize() {
    let rule = Rule::ChineseNewYear;

    for year in 2020..2030 {
        let mut out = SmallVec::<[Date; 1]>::new();
        rule.materialize_year(year, &mut out);

        // Should produce exactly one date
        assert_eq!(out.len(), 1);

        // Should be January or February
        let date = out[0];
        assert!(
            date.month() == Month::January || date.month() == Month::February,
            "CNY should be in January or February, got {:?}",
            date
        );
    }
}

#[test]
fn rule_chinese_new_year_known_dates() {
    let rule = Rule::ChineseNewYear;

    // Known CNY dates
    let known = [
        (2020, 1, 25),
        (2021, 2, 12),
        (2022, 2, 1),
        (2023, 1, 22),
        (2024, 2, 10),
        (2025, 1, 29),
    ];

    for (year, month, day) in known {
        let date = make_date(year, month, day);
        assert!(
            rule.applies(date),
            "CNY {} should be {}-{}-{}",
            year,
            year,
            month,
            day
        );
    }
}

// ============================================
// Fixed Rule Edge Cases
// ============================================

#[test]
fn rule_fixed_feb_29_leap_year() {
    let rule = Rule::Fixed {
        month: Month::February,
        day: 29,
        observed: Observed::None,
    };

    // Feb 29 exists in 2024 (leap year)
    assert!(rule.applies(make_date(2024, 2, 29)));

    // Feb 29 doesn't exist in 2023 - shouldn't match any date
    assert!(!rule.applies(make_date(2023, 2, 28)));
    assert!(!rule.applies(make_date(2023, 3, 1)));
}

#[test]
fn rule_fixed_materialize_feb_29_non_leap() {
    let rule = Rule::Fixed {
        month: Month::February,
        day: 29,
        observed: Observed::None,
    };

    // Non-leap year 2023
    let mut out = SmallVec::<[Date; 1]>::new();
    rule.materialize_year(2023, &mut out);

    // Should not produce any dates (invalid date in non-leap year)
    // Based on implementation, it pushes 1900-01-01 then filters it out
    assert!(
        out.is_empty() || out.iter().all(|d| d.year() != 2023),
        "Feb 29 in non-leap year should not produce valid 2023 dates"
    );
}

#[test]
fn rule_fixed_materialize_feb_29_leap() {
    let rule = Rule::Fixed {
        month: Month::February,
        day: 29,
        observed: Observed::None,
    };

    // Leap year 2024
    let mut out = SmallVec::<[Date; 1]>::new();
    rule.materialize_year(2024, &mut out);

    // Should produce Feb 29, 2024
    assert_eq!(out.len(), 1);
    assert_eq!(out[0], make_date(2024, 2, 29));
}

// ============================================
// WeekdayShift Edge Cases
// ============================================

#[test]
fn rule_weekday_shift_materialize() {
    // First Tuesday on or after Nov 2 (US Election Day)
    let rule = Rule::WeekdayShift {
        weekday: Weekday::Tuesday,
        month: Month::November,
        day: 2,
        dir: Direction::After,
    };

    let mut out = SmallVec::<[Date; 1]>::new();
    rule.materialize_year(2024, &mut out);

    // 2024: Nov 2 is Saturday, first Tuesday after is Nov 5
    assert_eq!(out.len(), 1);
    assert_eq!(out[0], make_date(2024, 11, 5));
    assert_eq!(out[0].weekday(), Weekday::Tuesday);
}

#[test]
fn rule_weekday_shift_before_materialize() {
    let rule = Rule::WeekdayShift {
        weekday: Weekday::Friday,
        month: Month::June,
        day: 15,
        dir: Direction::Before,
    };

    let mut out = SmallVec::<[Date; 1]>::new();
    rule.materialize_year(2025, &mut out);

    assert_eq!(out.len(), 1);
    assert_eq!(out[0].weekday(), Weekday::Friday);
    // Should be on or before June 15
    assert!(out[0] <= make_date(2025, 6, 15));
}

// ============================================
// NthWeekday Edge Cases
// ============================================

#[test]
fn rule_nth_weekday_fifth_occurrence() {
    // 5th Monday if exists
    let rule = Rule::NthWeekday {
        n: 5,
        weekday: Weekday::Monday,
        month: Month::December,
    };

    let mut out = SmallVec::<[Date; 1]>::new();
    rule.materialize_year(2025, &mut out);

    // December 2025 has 5 Mondays: 1, 8, 15, 22, 29
    assert_eq!(out.len(), 1);
    assert_eq!(out[0], make_date(2025, 12, 29));
}

#[test]
fn rule_nth_weekday_second_to_last() {
    // Second-to-last Friday
    let rule = Rule::NthWeekday {
        n: -2,
        weekday: Weekday::Friday,
        month: Month::November,
    };

    let mut out = SmallVec::<[Date; 1]>::new();
    rule.materialize_year(2025, &mut out);

    assert_eq!(out.len(), 1);
    // November 2025: Fridays are 7, 14, 21, 28
    // Second-to-last = 21st
    assert_eq!(out[0], make_date(2025, 11, 21));
}

// ============================================
// Easter Offset Edge Cases
// ============================================

#[test]
fn rule_easter_positive_offsets() {
    // Ascension Day is Easter Monday + 38 days
    let ascension = Rule::EasterOffset(38);

    let mut out = SmallVec::<[Date; 1]>::new();
    ascension.materialize_year(2025, &mut out);

    assert_eq!(out.len(), 1);
    // Easter Monday 2025 is April 21
    // Ascension = April 21 + 38 = May 29
    assert_eq!(out[0], make_date(2025, 5, 29));
    assert_eq!(out[0].weekday(), Weekday::Thursday);
}

#[test]
fn rule_easter_whit_monday() {
    // Whit Monday is Easter Monday + 49 days
    let whit = Rule::EasterOffset(49);

    let mut out = SmallVec::<[Date; 1]>::new();
    whit.materialize_year(2025, &mut out);

    assert_eq!(out.len(), 1);
    // Easter Monday 2025 is April 21
    // Whit = April 21 + 49 = June 9
    assert_eq!(out[0], make_date(2025, 6, 9));
    assert_eq!(out[0].weekday(), Weekday::Monday);
}

// ============================================
// Observed Edge Cases
// ============================================

#[test]
fn rule_observed_friday_for_saturday() {
    let rule = Rule::Fixed {
        month: Month::July,
        day: 4,
        observed: Observed::FriIfSatMonIfSun,
    };

    // 2020: July 4 is Saturday
    // Observed on Friday July 3
    assert!(rule.applies(make_date(2020, 7, 3)));
    assert!(!rule.applies(make_date(2020, 7, 4)));
    assert!(!rule.applies(make_date(2020, 7, 6)));
}

#[test]
fn rule_observed_monday_for_sunday() {
    let rule = Rule::Fixed {
        month: Month::July,
        day: 4,
        observed: Observed::FriIfSatMonIfSun,
    };

    // 2021: July 4 is Sunday
    // Observed on Monday July 5
    assert!(rule.applies(make_date(2021, 7, 5)));
    assert!(!rule.applies(make_date(2021, 7, 4)));
    assert!(!rule.applies(make_date(2021, 7, 2)));
}

#[test]
fn rule_observed_next_monday_saturday() {
    let rule = Rule::Fixed {
        month: Month::December,
        day: 25,
        observed: Observed::NextMonday,
    };

    // 2021: Dec 25 is Saturday
    // UK style: observed on Monday Dec 27
    assert!(rule.applies(make_date(2021, 12, 27)));
    assert!(!rule.applies(make_date(2021, 12, 25)));
}

#[test]
fn rule_observed_next_monday_sunday() {
    let rule = Rule::Fixed {
        month: Month::December,
        day: 25,
        observed: Observed::NextMonday,
    };

    // 2022: Dec 25 is Sunday
    // UK style: observed on Monday Dec 26
    assert!(rule.applies(make_date(2022, 12, 26)));
    assert!(!rule.applies(make_date(2022, 12, 25)));
}

// ============================================
// Direction Enum Tests
// ============================================

#[test]
fn direction_after_finds_same_day() {
    // If reference is already the target weekday, After should find it
    let rule = Rule::WeekdayShift {
        weekday: Weekday::Wednesday,
        month: Month::January,
        day: 1, // 2025: Jan 1 is Wednesday
        dir: Direction::After,
    };

    assert!(rule.applies(make_date(2025, 1, 1)));
}

#[test]
fn direction_before_finds_same_day() {
    // If reference is already the target weekday, Before should find it
    let rule = Rule::WeekdayShift {
        weekday: Weekday::Wednesday,
        month: Month::January,
        day: 1, // 2025: Jan 1 is Wednesday
        dir: Direction::Before,
    };

    assert!(rule.applies(make_date(2025, 1, 1)));
}

