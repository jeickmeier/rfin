//! Tests for generated calendar bitsets and helpers

use finstack_core::dates::calendar::generated::{
    bit_test, day_of_year_0_based, nth_weekday_of_month, YearBits, BASE_YEAR, BITSET_WORDS,
    END_YEAR, YEARS,
};
use finstack_core::dates::Date;
use time::{Duration, Month, Weekday};

fn make_date(y: i32, m: u8, d: u8) -> Date {
    Date::from_calendar_date(y, Month::try_from(m).unwrap(), d).unwrap()
}

#[test]
fn constants_are_sensible() {
    assert_eq!(BASE_YEAR, 1970);
    assert_eq!(END_YEAR, 2150);
    assert_eq!(YEARS, (END_YEAR - BASE_YEAR + 1) as usize);
    assert_eq!(BITSET_WORDS, 6); // (366 + 63) / 64 = 6
}

#[test]
fn day_of_year_0_based_jan_1() {
    let jan1 = make_date(2025, 1, 1);
    let day_idx = day_of_year_0_based(jan1);
    assert_eq!(day_idx, 0);
}

#[test]
fn day_of_year_0_based_dec_31() {
    // Non-leap year
    let dec31_2025 = make_date(2025, 12, 31);
    let day_idx = day_of_year_0_based(dec31_2025);
    assert_eq!(day_idx, 364); // 0-based, 365 days total

    // Leap year
    let dec31_2024 = make_date(2024, 12, 31);
    let day_idx = day_of_year_0_based(dec31_2024);
    assert_eq!(day_idx, 365); // 0-based, 366 days total
}

#[test]
fn day_of_year_0_based_feb_29_leap_year() {
    let feb29_2024 = make_date(2024, 2, 29);
    let day_idx = day_of_year_0_based(feb29_2024);

    // Jan has 31 days, so Feb 29 is day 31+28 = 59 (0-based)
    assert_eq!(day_idx, 59);
}

#[test]
fn day_of_year_0_based_sequential() {
    let jan1 = make_date(2025, 1, 1);

    for days in 0..365 {
        let date = jan1 + Duration::days(days);
        let idx = day_of_year_0_based(date);
        assert_eq!(idx as i64, days);
    }
}

#[test]
fn bit_test_all_zero() {
    let bits: YearBits = [0; BITSET_WORDS];

    for idx in 0..366 {
        assert!(!bit_test(&bits, idx));
    }
}

#[test]
fn bit_test_all_one() {
    let bits: YearBits = [u64::MAX; BITSET_WORDS];

    // First 366 bits should be set
    for idx in 0..366 {
        assert!(bit_test(&bits, idx));
    }
}

#[test]
fn bit_test_specific_bits() {
    let mut bits: YearBits = [0; BITSET_WORDS];

    // Set bit 0 (Jan 1)
    bits[0] |= 1;
    assert!(bit_test(&bits, 0));
    assert!(!bit_test(&bits, 1));

    // Set bit 59 (Feb 29 in leap year)
    bits[59 >> 6] |= 1 << (59 & 63);
    assert!(bit_test(&bits, 59));

    // Set bit 365 (Dec 31 in leap year)
    bits[365 >> 6] |= 1 << (365 & 63);
    assert!(bit_test(&bits, 365));
}

#[test]
fn nth_weekday_of_month_first_monday() {
    // First Monday of January 2025
    let date = nth_weekday_of_month(2025, Month::January, Weekday::Monday, 1);
    assert_eq!(date, make_date(2025, 1, 6));
    assert_eq!(date.weekday(), Weekday::Monday);
}

#[test]
fn nth_weekday_of_month_third_wednesday() {
    // Third Wednesday of March 2025 (IMM date)
    let date = nth_weekday_of_month(2025, Month::March, Weekday::Wednesday, 3);
    assert_eq!(date, make_date(2025, 3, 19));
    assert_eq!(date.weekday(), Weekday::Wednesday);
}

#[test]
fn nth_weekday_of_month_last_monday() {
    // Last Monday of May 2025
    let date = nth_weekday_of_month(2025, Month::May, Weekday::Monday, -1);
    assert_eq!(date, make_date(2025, 5, 26));
    assert_eq!(date.weekday(), Weekday::Monday);
}

#[test]
fn nth_weekday_of_month_second_last() {
    // Second-to-last Friday of December 2025
    let date = nth_weekday_of_month(2025, Month::December, Weekday::Friday, -2);
    assert_eq!(date.weekday(), Weekday::Friday);
    assert_eq!(date.month(), Month::December);

    // Should be earlier than last Friday
    let last_friday = nth_weekday_of_month(2025, Month::December, Weekday::Friday, -1);
    assert!(date < last_friday);
}

#[test]
fn nth_weekday_of_month_consistency() {
    // Verify all weekdays in a month
    for month in [
        Month::January,
        Month::February,
        Month::March,
        Month::April,
        Month::May,
        Month::June,
        Month::July,
        Month::August,
        Month::September,
        Month::October,
        Month::November,
        Month::December,
    ] {
        for weekday in [
            Weekday::Monday,
            Weekday::Tuesday,
            Weekday::Wednesday,
            Weekday::Thursday,
            Weekday::Friday,
            Weekday::Saturday,
            Weekday::Sunday,
        ] {
            // First occurrence
            let first = nth_weekday_of_month(2025, month, weekday, 1);
            assert_eq!(first.weekday(), weekday);
            assert_eq!(first.month(), month);
            assert!(first.day() <= 7); // First occurrence within first week

            // Last occurrence
            let last = nth_weekday_of_month(2025, month, weekday, -1);
            assert_eq!(last.weekday(), weekday);
            assert_eq!(last.month(), month);
        }
    }
}

#[test]
fn nth_weekday_of_month_fourth_vs_last() {
    // In some months, 4th and last might be the same
    // In others, they might differ

    // November 2025: Check if 4th Thursday == last Thursday
    let fourth = nth_weekday_of_month(2025, Month::November, Weekday::Thursday, 4);
    let last = nth_weekday_of_month(2025, Month::November, Weekday::Thursday, -1);

    // November 2025 has 4 Thursdays, so 4th == last
    assert_eq!(fourth, last);

    // Try a month with 5 occurrences: January 2025 has 5 Thursdays
    let fourth_jan = nth_weekday_of_month(2025, Month::January, Weekday::Thursday, 4);
    let last_jan = nth_weekday_of_month(2025, Month::January, Weekday::Thursday, -1);

    // Should have 5 Thursdays, so 4th != last
    assert_ne!(fourth_jan, last_jan);
    assert!(fourth_jan < last_jan);
}

#[test]
fn bitset_word_boundary_handling() {
    let mut bits: YearBits = [0; BITSET_WORDS];

    // Test bits around word boundaries (multiples of 64)
    for bit_idx in [63, 64, 127, 128, 191, 192, 255, 256, 319, 320] {
        if bit_idx < 366 {
            // Set the bit
            let word = (bit_idx as usize) >> 6;
            let offset = bit_idx as usize & 63;
            bits[word] |= 1 << offset;

            // Test it
            assert!(bit_test(&bits, bit_idx));
        }
    }
}

#[test]
fn day_of_year_consistency_across_leap_and_regular() {
    // Test that day indices progress correctly across leap and non-leap years

    // Regular year (2025)
    let jan1_2025 = make_date(2025, 1, 1);
    let feb1_2025 = make_date(2025, 2, 1);
    let mar1_2025 = make_date(2025, 3, 1);

    assert_eq!(day_of_year_0_based(jan1_2025), 0);
    assert_eq!(day_of_year_0_based(feb1_2025), 31);
    assert_eq!(day_of_year_0_based(mar1_2025), 59); // 31 + 28

    // Leap year (2024)
    let jan1_2024 = make_date(2024, 1, 1);
    let feb1_2024 = make_date(2024, 2, 1);
    let mar1_2024 = make_date(2024, 3, 1);

    assert_eq!(day_of_year_0_based(jan1_2024), 0);
    assert_eq!(day_of_year_0_based(feb1_2024), 31);
    assert_eq!(day_of_year_0_based(mar1_2024), 60); // 31 + 29
}
