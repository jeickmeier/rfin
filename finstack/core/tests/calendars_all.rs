//! Tests for all built-in holiday calendars.

mod common;

use common::make_date;
use finstack_core::dates::calendars::*;
use finstack_core::dates::HolidayCalendar;

#[test]
fn test_calendar_by_id_lookup() {
    // Test that all calendar IDs can be looked up
    for &id in ALL_IDS {
        let cal = calendar_by_id(id);
        assert!(cal.is_some(), "Calendar '{}' should be found", id);
        
        // Test that each calendar can be used (call is_holiday on a mid-week date)
        let mid_week_date = make_date(2025, 6, 18); // Wednesday
        let _is_holiday = cal.unwrap().is_holiday(mid_week_date);
        // We don't assert the result since we don't know if this specific date is a holiday
        // The important thing is that the call doesn't panic
    }
}

#[test]
fn test_unknown_calendar_id() {
    let cal = calendar_by_id("unknown_calendar");
    assert!(cal.is_none());
}

#[test]
fn test_calendar_id_methods() {
    // Test that each calendar type's id() method returns the correct string
    assert_eq!(Gblo::new().id(), "gblo");
    assert_eq!(Target2::new().id(), "target2");
    assert_eq!(Asx::new().id(), "asx");
    assert_eq!(Auce::new().id(), "auce");
    assert_eq!(Cato::new().id(), "cato");
    assert_eq!(Defr::new().id(), "defr");
    assert_eq!(Nyse::new().id(), "nyse");
    assert_eq!(Usny::new().id(), "usny");
    assert_eq!(Sifma::new().id(), "sifma");
    assert_eq!(Brbd::new().id(), "brbd");
    assert_eq!(Chzh::new().id(), "chzh");
    assert_eq!(Cnbe::new().id(), "cnbe");
    assert_eq!(Sgsi::new().id(), "sgsi");
    assert_eq!(Sse::new().id(), "sse");
    assert_eq!(Hkhk::new().id(), "hkhk");
    assert_eq!(Hkex::new().id(), "hkex");
    assert_eq!(Jpto::new().id(), "jpto");
    assert_eq!(Jpx::new().id(), "jpx");
    assert_eq!(Cme::new().id(), "cme");
}

#[test]
fn test_gblo_known_holidays() {
    let cal = Gblo::new();
    
    // Test some known UK holidays in 2025
    // New Year's Day 2025 (January 1) - Wednesday
    assert!(cal.is_holiday(make_date(2025, 1, 1)));
    
    // Christmas Day 2025 (December 25) - Thursday
    assert!(cal.is_holiday(make_date(2025, 12, 25)));
    
    // Boxing Day 2025 (December 26) - Friday
    assert!(cal.is_holiday(make_date(2025, 12, 26)));
    
    // A regular business day should not be a holiday
    assert!(!cal.is_holiday(make_date(2025, 6, 18))); // Wednesday
}

#[test]
fn test_target2_known_holidays() {
    let cal = Target2::new();
    
    // Test some known TARGET2 holidays in 2025
    // New Year's Day 2025 (January 1) - Wednesday
    assert!(cal.is_holiday(make_date(2025, 1, 1)));
    
    // Christmas Day 2025 (December 25) - Thursday
    assert!(cal.is_holiday(make_date(2025, 12, 25)));
    
    // A regular business day should not be a holiday
    assert!(!cal.is_holiday(make_date(2025, 6, 18))); // Wednesday
}

#[test]
fn test_nyse_known_holidays() {
    let cal = Nyse::new();
    
    // Test some known NYSE holidays in 2025
    // New Year's Day 2025 (January 1) - Wednesday
    assert!(cal.is_holiday(make_date(2025, 1, 1)));
    
    // Christmas Day 2025 (December 25) - Thursday
    assert!(cal.is_holiday(make_date(2025, 12, 25)));
    
    // A regular business day should not be a holiday
    assert!(!cal.is_holiday(make_date(2025, 6, 18))); // Wednesday
}

#[test]
fn test_calendar_weekend_behavior() {
    // Test that all calendars properly handle weekends via the trait default
    let cal = Gblo::new();
    
    // Saturday and Sunday should not be business days
    assert!(!cal.is_business_day(make_date(2025, 6, 21))); // Saturday
    assert!(!cal.is_business_day(make_date(2025, 6, 22))); // Sunday
    
    // Regular weekday should be a business day (assuming no holiday)
    assert!(cal.is_business_day(make_date(2025, 6, 18))); // Wednesday
}

#[test]
fn test_all_calendar_constructors() {
    // Test that all calendar types can be constructed without panicking
    let _gblo = Gblo::new();
    let _target2 = Target2::new();
    let _asx = Asx::new();
    let _auce = Auce::new();
    let _cato = Cato::new();
    let _defr = Defr::new();
    let _nyse = Nyse::new();
    let _usny = Usny::new();
    let _sifma = Sifma::new();
    let _brbd = Brbd::new();
    let _chzh = Chzh::new();
    let _cnbe = Cnbe::new();
    let _sgsi = Sgsi::new();
    let _sse = Sse::new();
    let _hkhk = Hkhk::new();
    let _hkex = Hkex::new();
    let _jpto = Jpto::new();
    let _jpx = Jpx::new();
    let _cme = Cme::new();
}
