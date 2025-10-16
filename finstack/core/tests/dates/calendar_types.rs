use finstack_core::dates::calendar::business_days::HolidayCalendar;
use finstack_core::dates::calendar::generated::{
    day_of_year_0_based, YearBits, BASE_YEAR, BITSET_WORDS,
};
use finstack_core::dates::calendar::rule::Rule;
use finstack_core::dates::calendar::types::Calendar;
use finstack_core::dates::Date;
use time::{Duration, Month, Weekday};

fn date(year: i32, month: Month, day: u8) -> Date {
    Date::from_calendar_date(year, month, day).unwrap()
}

#[test]
fn calendar_rule_based_holiday_detection() {
    let rules = Box::leak(Box::new([Rule::fixed_next_monday(Month::January, 1)]));
    let cal = Calendar::new("test", "Test Calendar", false, rules);

    let jan1 = date(2024, Month::January, 1); // Monday => observed same day
    assert!(cal.is_holiday(jan1));

    let jan1_2023 = date(2023, Month::January, 1); // Sunday => observed next Monday
    let monday_observed = jan1_2023 + Duration::days(1);
    assert!(cal.is_holiday(monday_observed));
}

#[test]
fn calendar_bitset_lookup_honors_ignore_weekends() {
    let mut bits: YearBits = [0; BITSET_WORDS];
    let mut weekend_day = date(BASE_YEAR, Month::January, 1);
    while !matches!(weekend_day.weekday(), Weekday::Saturday | Weekday::Sunday) {
        weekend_day += Duration::days(1);
    }
    let day_idx = day_of_year_0_based(weekend_day);
    let word = (day_idx as usize) >> 6;
    let bit = day_idx as usize & 63;
    bits[word] |= 1u64 << bit;

    let cal = Calendar::new("bit", "Bitset", true, &[]).with_bitsets(Box::leak(Box::new([bits])));
    assert!(!cal.is_holiday(weekend_day));

    let weekday = weekend_day + Duration::days(2);
    let idx = day_of_year_0_based(weekday);
    let mut bits_weekday = [0u64; BITSET_WORDS];
    let word_w = (idx as usize) >> 6;
    let bit_w = idx as usize & 63;
    bits_weekday[word_w] |= 1u64 << bit_w;
    let cal_weekday = Calendar::new("bit", "Bitset", false, &[])
        .with_bitsets(Box::leak(Box::new([bits_weekday])));
    assert!(cal_weekday.is_holiday(weekday));
}
