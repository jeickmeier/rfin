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
    assert_eq!(Gblo.id(), "gblo");
    assert_eq!(Target2.id(), "target2");
    assert_eq!(Asx.id(), "asx");
    assert_eq!(Auce.id(), "auce");
    assert_eq!(Cato.id(), "cato");
    assert_eq!(Defr.id(), "defr");
    assert_eq!(Nyse.id(), "nyse");
    assert_eq!(Usny.id(), "usny");
    assert_eq!(Sifma.id(), "sifma");
    assert_eq!(Brbd.id(), "brbd");
    assert_eq!(Chzh.id(), "chzh");
    assert_eq!(Cnbe.id(), "cnbe");
    assert_eq!(Sgsi.id(), "sgsi");
    assert_eq!(Sse.id(), "sse");
    assert_eq!(Hkhk.id(), "hkhk");
    assert_eq!(Hkex.id(), "hkex");
    assert_eq!(Jpto.id(), "jpto");
    assert_eq!(Jpx.id(), "jpx");
    assert_eq!(Cme.id(), "cme");
}

#[test]
fn test_gblo_known_holidays() {
    let cal = Gblo;
    
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
    let cal = Target2;
    
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
    let cal = Nyse;
    
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
    let cal = Gblo;
    
    // Saturday and Sunday should not be business days
    assert!(!cal.is_business_day(make_date(2025, 6, 21))); // Saturday
    assert!(!cal.is_business_day(make_date(2025, 6, 22))); // Sunday
    
    // Regular weekday should be a business day (assuming no holiday)
    assert!(cal.is_business_day(make_date(2025, 6, 18))); // Wednesday
}

#[test]
fn test_all_calendar_constructors() {
    // Test that all calendar types can be constructed without panicking
    let _gblo = Gblo;
    let _target2 = Target2;
    let _asx = Asx;
    let _auce = Auce;
    let _cato = Cato;
    let _defr = Defr;
    let _nyse = Nyse;
    let _usny = Usny;
    let _sifma = Sifma;
    let _brbd = Brbd;
    let _chzh = Chzh;
    let _cnbe = Cnbe;
    let _sgsi = Sgsi;
    let _sse = Sse;
    let _hkhk = Hkhk;
    let _hkex = Hkex;
    let _jpto = Jpto;
    let _jpx = Jpx;
    let _cme = Cme;
}
