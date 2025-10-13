//! Tests for Chinese New Year edge cases and extended date range coverage.

use finstack_core::dates::{Date, HolidayCalendar};
use finstack_core::dates::calendar::{CNBE as Cnbe, HKHK as Hkhk, SGSI as Sgsi};
use time::Month;

fn make_date(y: i32, m: u8, d: u8) -> Date {
    Date::from_calendar_date(y, Month::try_from(m).unwrap(), d).unwrap()
}

#[test]
fn test_cny_early_years_1970s() {
    let cnbe = Cnbe;
    let hkhk = Hkhk;
    let sgsi = Sgsi;
    
    // Test specific CNY dates in the early years (1970-1989)
    // These should now work with the extended CSV data
    
    // 1970: CNY = Feb 6 (from CSV)
    assert!(cnbe.is_holiday(make_date(1970, 2, 6)), "CNBE should recognize CNY 1970");
    assert!(hkhk.is_holiday(make_date(1970, 2, 6)), "HKHK should recognize CNY 1970");
    assert!(sgsi.is_holiday(make_date(1970, 2, 6)), "SGSI should recognize CNY 1970");
    
    // 1975: CNY = Feb 11 (from CSV)
    assert!(cnbe.is_holiday(make_date(1975, 2, 11)), "CNBE should recognize CNY 1975");
    assert!(hkhk.is_holiday(make_date(1975, 2, 11)), "HKHK should recognize CNY 1975");
    assert!(sgsi.is_holiday(make_date(1975, 2, 11)), "SGSI should recognize CNY 1975");
    
    // 1980: CNY = Feb 16 (from CSV)
    assert!(cnbe.is_holiday(make_date(1980, 2, 16)), "CNBE should recognize CNY 1980");
    assert!(hkhk.is_holiday(make_date(1980, 2, 16)), "HKHK should recognize CNY 1980");
    assert!(sgsi.is_holiday(make_date(1980, 2, 16)), "SGSI should recognize CNY 1980");
    
    // 1989: CNY = Feb 6 (from CSV)
    assert!(cnbe.is_holiday(make_date(1989, 2, 6)), "CNBE should recognize CNY 1989");
    assert!(hkhk.is_holiday(make_date(1989, 2, 6)), "HKHK should recognize CNY 1989");
    assert!(sgsi.is_holiday(make_date(1989, 2, 6)), "SGSI should recognize CNY 1989");
}

#[test]
fn test_cny_late_years_2100s() {
    let cnbe = Cnbe;
    let hkhk = Hkhk;
    let sgsi = Sgsi;
    
    // Test specific CNY dates in the late years (2101-2150)
    // These should now work with the extended CSV data
    
    // 2101: CNY = Jan 29 (from CSV)
    assert!(cnbe.is_holiday(make_date(2101, 1, 29)), "CNBE should recognize CNY 2101");
    assert!(hkhk.is_holiday(make_date(2101, 1, 29)), "HKHK should recognize CNY 2101");
    assert!(sgsi.is_holiday(make_date(2101, 1, 29)), "SGSI should recognize CNY 2101");
    
    // 2125: CNY = Feb 3 (from CSV)
    assert!(cnbe.is_holiday(make_date(2125, 2, 3)), "CNBE should recognize CNY 2125");
    assert!(hkhk.is_holiday(make_date(2125, 2, 3)), "HKHK should recognize CNY 2125");
    assert!(sgsi.is_holiday(make_date(2125, 2, 3)), "SGSI should recognize CNY 2125");
    
    // 2150: CNY = Jan 28 (from CSV)
    assert!(cnbe.is_holiday(make_date(2150, 1, 28)), "CNBE should recognize CNY 2150");
    assert!(hkhk.is_holiday(make_date(2150, 1, 28)), "HKHK should recognize CNY 2150");
    assert!(sgsi.is_holiday(make_date(2150, 1, 28)), "SGSI should recognize CNY 2150");
}

#[test]
fn test_cny_span_rules_edge_years() {
    let cnbe = Cnbe; // Has 7-day CNY span
    let hkhk = Hkhk; // Has 3-day CNY span
    let sgsi = Sgsi; // Has 2-day CNY span
    
    // Test that CNY spans work correctly in edge years
    
    // 1970: CNY = Feb 6, so spans should be Feb 6-12 (CNBE), Feb 6-8 (HKHK), Feb 6-7 (SGSI)
    for day in 6..=12 {
        let expected_cnbe = day <= 12; // 7-day span
        let expected_hkhk = day <= 8;  // 3-day span  
        let expected_sgsi = day <= 7;  // 2-day span
        
        assert_eq!(
            cnbe.is_holiday(make_date(1970, 2, day)),
            expected_cnbe,
            "CNBE CNY span day {} in 1970",
            day
        );
        assert_eq!(
            hkhk.is_holiday(make_date(1970, 2, day)),
            expected_hkhk,
            "HKHK CNY span day {} in 1970", 
            day
        );
        assert_eq!(
            sgsi.is_holiday(make_date(1970, 2, day)),
            expected_sgsi,
            "SGSI CNY span day {} in 1970",
            day
        );
    }
}

#[test]
fn test_cny_boundary_validation() {
    let cnbe = Cnbe;
    
    // Test years right at the boundaries of our CSV coverage. Older implementations
    // could panic or return inconsistent values outside the supported range, so we
    // simply ensure the calls succeed and then assert known boundary-year holidays.
    let _ = cnbe.is_holiday(make_date(1969, 1, 1));
    let _ = cnbe.is_holiday(make_date(2151, 1, 1));

    // At the documented boundaries – CNY span should be honoured.
    assert!(cnbe.is_holiday(make_date(1970, 2, 6)), "1970 CNY should work");
    assert!(cnbe.is_holiday(make_date(2150, 1, 28)), "2150 CNY should work");
}

#[test] 
fn test_buddhas_birthday_edge_years() {
    let hkhk = Hkhk; // Uses Buddha's Birthday (CNY + 95 days)
    
    // Test that Buddha's Birthday works in edge years
    // 1970: CNY = Feb 6, so Buddha's Birthday = Feb 6 + 95 days = May 12
    let bb_1970 = make_date(1970, 2, 6).checked_add(time::Duration::days(95)).unwrap();
    assert_eq!(bb_1970, make_date(1970, 5, 12));
    assert!(hkhk.is_holiday(bb_1970), "Buddha's Birthday 1970 should be recognized");
    
    // 2125: CNY = Feb 3, so Buddha's Birthday = Feb 3 + 95 days = May 9  
    let bb_2125 = make_date(2125, 2, 3).checked_add(time::Duration::days(95)).unwrap();
    assert_eq!(bb_2125, make_date(2125, 5, 9));
    assert!(hkhk.is_holiday(bb_2125), "Buddha's Birthday 2125 should be recognized");
}
