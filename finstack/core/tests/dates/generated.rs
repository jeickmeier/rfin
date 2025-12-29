//! Generated calendars wiring tests.
//!
//! These tests validate that build-time generated calendars are exposed through the
//! public `finstack_core::dates` API (without relying on internal bitset helpers).

use finstack_core::dates::calendar::{TARGET2, USNY};
use finstack_core::dates::{CalendarRegistry, Date, HolidayCalendar};
use time::Month;

fn make_date(y: i32, m: u8, d: u8) -> Date {
    Date::from_calendar_date(y, Month::try_from(m).unwrap(), d).unwrap()
}

#[test]
fn generated_calendar_constants_exist_and_work() {
    // Jan 1 is a holiday for TARGET2
    let jan1 = make_date(2025, 1, 1);
    assert!(TARGET2.is_holiday(jan1));

    // Weekends are never business days (trait default)
    let sat = make_date(2025, 1, 4);
    assert!(!USNY.is_business_day(sat));
}

#[test]
fn calendar_registry_resolves_generated_calendars() {
    let registry = CalendarRegistry::global();

    let target2 = registry
        .resolve_str("target2")
        .expect("TARGET2 should be resolvable");

    let jan1 = make_date(2025, 1, 1);
    assert!(target2.is_holiday(jan1));
}

