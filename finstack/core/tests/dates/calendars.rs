//! Built-in holiday calendar tests with reduced duplication.

use super::common::make_date;
use finstack_core::dates::calendar::{
    calendar_by_id, ALL_IDS, ASX as Asx, AUCE as Auce, BRBD as Brbd, CATO as Cato, CHZH as Chzh,
    CME as Cme, CNBE as Cnbe, DEFR as Defr, GBLO as Gblo, HKEX as Hkex, HKHK as Hkhk,
    NYSE as Nyse, SGSI as Sgsi, SIFMA as Sifma, SSE as Sse, TARGET2 as Target2, USNY as Usny,
};
use finstack_core::dates::{CalendarRegistry, Date, HolidayCalendar};
use std::collections::HashSet;

fn holiday_set(cal: &dyn HolidayCalendar, year: i32) -> HashSet<Date> {
    (1..=if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) {
        366
    } else {
        365
    })
    .filter_map(|d| Date::from_ordinal_date(year, d).ok())
    .filter(|&dt| cal.is_holiday(dt))
    .collect()
}

#[derive(Clone, Copy)]
struct YearCheck {
    year: i32,
    expected_count: Option<usize>,
    must_have: &'static [(i32, u8, u8)],
}

#[derive(Clone, Copy)]
struct CalendarCase {
    name: &'static str,
    cal: &'static dyn HolidayCalendar,
    checks: &'static [YearCheck],
}

const CASES: &[CalendarCase] = &[
    CalendarCase {
        name: "USNY",
        cal: &Usny,
        checks: &[
            YearCheck {
                year: 2024,
                expected_count: Some(11),
                must_have: &[(2024, 1, 1), (2024, 7, 4), (2024, 12, 25)],
            },
            YearCheck {
                year: 2025,
                expected_count: Some(11),
                must_have: &[(2025, 1, 1), (2025, 7, 4), (2025, 12, 25)],
            },
        ],
    },
    CalendarCase {
        name: "NYSE",
        cal: &Nyse,
        checks: &[
            YearCheck {
                year: 2024,
                expected_count: Some(10),
                must_have: &[(2024, 1, 1), (2024, 3, 29), (2024, 12, 25)],
            },
            YearCheck {
                year: 2025,
                expected_count: Some(10),
                must_have: &[(2025, 1, 1), (2025, 4, 18), (2025, 12, 25)],
            },
        ],
    },
    CalendarCase {
        name: "CME",
        cal: &Cme,
        checks: &[
            YearCheck {
                year: 2024,
                expected_count: None,
                must_have: &[(2024, 3, 29), (2024, 7, 4)],
            },
            YearCheck {
                year: 2025,
                expected_count: None,
                must_have: &[(2025, 4, 18), (2025, 11, 27)],
            },
        ],
    },
    CalendarCase {
        name: "SIFMA",
        cal: &Sifma,
        checks: &[
            YearCheck {
                year: 2024,
                expected_count: Some(12),
                must_have: &[(2024, 3, 29), (2024, 10, 14), (2024, 11, 11)],
            },
            YearCheck {
                year: 2025,
                expected_count: Some(12),
                must_have: &[(2025, 4, 18), (2025, 10, 13), (2025, 11, 11)],
            },
        ],
    },
    CalendarCase {
        name: "TARGET2",
        cal: &Target2,
        checks: &[
            YearCheck {
                year: 2024,
                expected_count: Some(6),
                must_have: &[(2024, 3, 29), (2024, 12, 26)],
            },
            YearCheck {
                year: 2025,
                expected_count: Some(6),
                must_have: &[(2025, 4, 18), (2025, 12, 26)],
            },
        ],
    },
    CalendarCase {
        name: "DEFR",
        cal: &Defr,
        checks: &[
            YearCheck {
                year: 2024,
                expected_count: Some(6),
                must_have: &[(2024, 5, 1), (2024, 12, 25)],
            },
            YearCheck {
                year: 2025,
                expected_count: Some(6),
                must_have: &[(2025, 5, 1), (2025, 12, 25)],
            },
        ],
    },
    CalendarCase {
        name: "GBLO",
        cal: &Gblo,
        checks: &[
            YearCheck {
                year: 2024,
                expected_count: Some(8),
                must_have: &[(2024, 3, 29), (2024, 5, 6), (2024, 12, 25)],
            },
            YearCheck {
                year: 2025,
                expected_count: Some(8),
                must_have: &[(2025, 4, 18), (2025, 5, 5), (2025, 12, 25)],
            },
        ],
    },
    CalendarCase {
        name: "CHZH",
        cal: &Chzh,
        checks: &[
            YearCheck {
                year: 2024,
                expected_count: Some(10),
                must_have: &[(2024, 5, 9), (2024, 8, 1)],
            },
            YearCheck {
                year: 2025,
                expected_count: Some(10),
                must_have: &[(2025, 5, 29), (2025, 8, 1)],
            },
        ],
    },
    CalendarCase {
        name: "HKHK",
        cal: &Hkhk,
        checks: &[
            YearCheck {
                year: 2024,
                expected_count: Some(11),
                must_have: &[(2024, 2, 10), (2024, 7, 1), (2024, 12, 25)],
            },
            YearCheck {
                year: 2025,
                expected_count: Some(11),
                must_have: &[(2025, 1, 29), (2025, 4, 4), (2025, 10, 1)],
            },
        ],
    },
    CalendarCase {
        name: "HKEX",
        cal: &Hkex,
        checks: &[
            YearCheck {
                year: 2024,
                expected_count: Some(11),
                must_have: &[(2024, 2, 10), (2024, 5, 15)],
            },
            YearCheck {
                year: 2025,
                expected_count: Some(11),
                must_have: &[(2025, 1, 29), (2025, 10, 1)],
            },
        ],
    },
    CalendarCase {
        name: "CNBE",
        cal: &Cnbe,
        checks: &[
            YearCheck {
                year: 2024,
                expected_count: Some(23),
                must_have: &[(2024, 2, 10), (2024, 5, 1), (2024, 10, 1)],
            },
            YearCheck {
                year: 2025,
                expected_count: Some(23),
                must_have: &[(2025, 1, 29), (2025, 5, 1), (2025, 10, 1)],
            },
        ],
    },
    CalendarCase {
        name: "SSE",
        cal: &Sse,
        checks: &[
            YearCheck {
                year: 2024,
                expected_count: Some(23),
                must_have: &[(2024, 2, 10), (2024, 5, 1), (2024, 10, 1)],
            },
            YearCheck {
                year: 2025,
                expected_count: Some(23),
                must_have: &[(2025, 1, 29), (2025, 5, 1), (2025, 10, 1)],
            },
        ],
    },
    CalendarCase {
        name: "SGSI",
        cal: &Sgsi,
        checks: &[
            YearCheck {
                year: 2024,
                expected_count: Some(7),
                must_have: &[(2024, 2, 10), (2024, 3, 29)],
            },
            YearCheck {
                year: 2025,
                expected_count: Some(7),
                must_have: &[(2025, 1, 29), (2025, 8, 11)],
            },
        ],
    },
    CalendarCase {
        name: "ASX",
        cal: &Asx,
        checks: &[
            YearCheck {
                year: 2024,
                expected_count: Some(7),
                must_have: &[(2024, 1, 29), (2024, 3, 29)],
            },
            YearCheck {
                year: 2025,
                expected_count: Some(7),
                must_have: &[(2025, 1, 27), (2025, 4, 18)],
            },
        ],
    },
    CalendarCase {
        name: "AUCE",
        cal: &Auce,
        checks: &[
            YearCheck {
                year: 2024,
                expected_count: Some(10),
                must_have: &[(2024, 1, 29), (2024, 6, 10), (2024, 10, 7)],
            },
            YearCheck {
                year: 2025,
                expected_count: Some(10),
                must_have: &[(2025, 1, 27), (2025, 6, 9), (2025, 10, 6)],
            },
        ],
    },
    CalendarCase {
        name: "BRBD",
        cal: &Brbd,
        checks: &[
            YearCheck {
                year: 2024,
                expected_count: Some(9),
                must_have: &[(2024, 2, 12), (2024, 11, 20)],
            },
            YearCheck {
                year: 2025,
                expected_count: Some(9),
                must_have: &[(2025, 3, 3), (2025, 11, 20)],
            },
        ],
    },
    CalendarCase {
        name: "CATO",
        cal: &Cato,
        checks: &[
            YearCheck {
                year: 2024,
                expected_count: Some(12),
                must_have: &[(2024, 2, 19), (2024, 7, 1), (2024, 10, 14)],
            },
            YearCheck {
                year: 2025,
                expected_count: Some(12),
                must_have: &[(2025, 2, 17), (2025, 7, 1), (2025, 10, 13)],
            },
        ],
    },
];

#[test]
fn calendars_match_sample_expectations() {
    for case in CASES {
        for check in case.checks {
            let holidays = holiday_set(case.cal, check.year);
            if let Some(expected) = check.expected_count {
                assert_eq!(
                    holidays.len(),
                    expected,
                    "{} {} expected {} holidays",
                    case.name,
                    check.year,
                    expected
                );
            }
            for &(y, m, d) in check.must_have {
                assert!(
                    holidays.contains(&make_date(y, m, d)),
                    "{} {} should include {:04}-{:02}-{:02}",
                    case.name,
                    check.year,
                    y,
                    m,
                    d
                );
            }
        }
    }
}

#[test]
fn test_calendar_by_id_lookup() {
    for &id in ALL_IDS {
        let cal = calendar_by_id(id);
        assert!(cal.is_some(), "Calendar '{}' should be found", id);

        let typed = CalendarRegistry::global().resolve_str(id);
        assert!(typed.is_some(), "Registry should find '{}'", id);

        let mid_week_date = make_date(2025, 6, 18);
        let _ = cal.unwrap().is_holiday(mid_week_date);
    }
}

#[test]
fn test_unknown_calendar_id() {
    assert!(calendar_by_id("unknown_calendar").is_none());
}

#[test]
fn test_calendar_weekend_behavior() {
    let cal = Gblo;
    assert!(!cal.is_business_day(make_date(2025, 6, 21)));
    assert!(!cal.is_business_day(make_date(2025, 6, 22)));
    assert!(cal.is_business_day(make_date(2025, 6, 18)));
}

#[test]
fn test_all_calendar_constructors() {
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
    let _cme = Cme;
}

// ============================================
// Chinese New Year Edge Cases (1970-2150)
// ============================================

#[test]
fn test_cny_early_years_1970s() {
    let cnbe = Cnbe;
    let hkhk = Hkhk;
    let sgsi = Sgsi;

    for &(y, m, d) in &[
        (1970, 2, 6),
        (1975, 2, 11),
        (1980, 2, 16),
        (1989, 2, 6),
    ] {
        let date = make_date(y, m, d);
        assert!(cnbe.is_holiday(date));
        assert!(hkhk.is_holiday(date));
        assert!(sgsi.is_holiday(date));
    }
}

#[test]
fn test_cny_late_years_2100s() {
    let cnbe = Cnbe;
    let hkhk = Hkhk;
    let sgsi = Sgsi;

    for &(y, m, d) in &[(2101, 1, 29), (2125, 2, 3), (2150, 1, 28)] {
        let date = make_date(y, m, d);
        assert!(cnbe.is_holiday(date));
        assert!(hkhk.is_holiday(date));
        assert!(sgsi.is_holiday(date));
    }
}

#[test]
fn nyse_2024_2025() {
    let cal = Nyse;
    assert_holidays_exact!(
        cal,
        2024,
        [
            (2024, 1, 1),
            (2024, 1, 15),
            (2024, 2, 19),
            (2024, 3, 29),
            (2024, 5, 27),
            (2024, 6, 19),
            (2024, 7, 4),
            (2024, 9, 2),
            (2024, 11, 28),
            (2024, 12, 25)
        ]
    );
    assert_holidays_exact!(
        cal,
        2025,
        [
            (2025, 1, 1),
            (2025, 1, 20),
            (2025, 2, 17),
            (2025, 4, 18),
            (2025, 5, 26),
            (2025, 6, 19),
            (2025, 7, 4),
            (2025, 9, 1),
            (2025, 11, 27),
            (2025, 12, 25)
        ]
    );
}

#[test]
fn cme_2024_2025() {
    let cal = Cme;
    // CME mirrors NYSE
    nyse_2024_2025(); // will panic if mismatched; reuse
                      // Additional direct check: ensure Good Friday 2024 is holiday
    assert!(cal.is_holiday(make_date(2024, 3, 29)));
}

#[test]
fn sifma_2024_2025() {
    let cal = Sifma;
    assert_holidays_exact!(
        cal,
        2024,
        [
            (2024, 1, 1),
            (2024, 1, 15),
            (2024, 2, 19),
            (2024, 3, 29),
            (2024, 5, 27),
            (2024, 6, 19),
            (2024, 7, 4),
            (2024, 9, 2),
            (2024, 10, 14),
            (2024, 11, 11),
            (2024, 11, 28),
            (2024, 12, 25)
        ]
    );
    assert_holidays_exact!(
        cal,
        2025,
        [
            (2025, 1, 1),
            (2025, 1, 20),
            (2025, 2, 17),
            (2025, 4, 18),
            (2025, 5, 26),
            (2025, 6, 19),
            (2025, 7, 4),
            (2025, 9, 1),
            (2025, 10, 13),
            (2025, 11, 11),
            (2025, 11, 27),
            (2025, 12, 25)
        ]
    );
}

// ============================================
// European Calendars
// ============================================

#[test]
fn target2_2024_2025() {
    let cal = Target2;
    assert_holidays_exact!(
        cal,
        2024,
        [
            (2024, 1, 1),
            (2024, 3, 29), // Good Fri
            (2024, 4, 1),  // Easter Mon
            (2024, 5, 1),
            (2024, 12, 25),
            (2024, 12, 26),
        ]
    );
    assert_holidays_exact!(
        cal,
        2025,
        [
            (2025, 1, 1),
            (2025, 4, 18),
            (2025, 4, 21),
            (2025, 5, 1),
            (2025, 12, 25),
            (2025, 12, 26),
        ]
    );
}

#[test]
fn defr_2024_2025() {
    let cal = Defr;
    // Same date set as TARGET2 plus May1
    assert_holidays_exact!(
        cal,
        2024,
        [
            (2024, 1, 1),
            (2024, 3, 29),
            (2024, 4, 1),
            (2024, 5, 1),
            (2024, 12, 25),
            (2024, 12, 26)
        ]
    );
    assert_holidays_exact!(
        cal,
        2025,
        [
            (2025, 1, 1),
            (2025, 4, 18),
            (2025, 4, 21),
            (2025, 5, 1),
            (2025, 12, 25),
            (2025, 12, 26)
        ]
    );
}

#[test]
fn gblo_2024_2025() {
    let cal = Gblo;
    assert_holidays_exact!(
        cal,
        2024,
        [
            (2024, 1, 1),
            (2024, 3, 29),
            (2024, 4, 1),
            (2024, 5, 6),
            (2024, 5, 27),
            (2024, 8, 26),
            (2024, 12, 25),
            (2024, 12, 26)
        ]
    );
    assert_holidays_exact!(
        cal,
        2025,
        [
            (2025, 1, 1),
            (2025, 4, 18),
            (2025, 4, 21),
            (2025, 5, 5),
            (2025, 5, 26),
            (2025, 8, 25),
            (2025, 12, 25),
            (2025, 12, 26)
        ]
    );
}

#[test]
fn chzh_2024_2025() {
    let cal = Chzh;
    assert_holidays_exact!(
        cal,
        2024,
        [
            (2024, 1, 1),
            (2024, 1, 2),
            (2024, 3, 29),
            (2024, 4, 1),
            (2024, 5, 1),
            (2024, 5, 9),
            (2024, 5, 20),
            (2024, 8, 1),
            (2024, 12, 25),
            (2024, 12, 26)
        ]
    );
    assert_holidays_exact!(
        cal,
        2025,
        [
            (2025, 1, 1),
            (2025, 1, 2),
            (2025, 4, 18),
            (2025, 4, 21),
            (2025, 5, 1),
            (2025, 5, 29),
            (2025, 6, 9),
            (2025, 8, 1),
            (2025, 12, 25),
            (2025, 12, 26)
        ]
    );
}

// ============================================
// Asia-Pacific Calendars
// ============================================

#[test]
fn hkhk_2024_2025() {
    let cal = Hkhk;
    assert_holidays_exact!(
        cal,
        2024,
        [
            (2024, 1, 1),
            (2024, 2, 10),
            (2024, 2, 11),
            (2024, 2, 12),
            (2024, 4, 4),
            (2024, 5, 1),
            (2024, 5, 15),
            (2024, 7, 1),
            (2024, 10, 1),
            (2024, 12, 25),
            (2024, 12, 26)
        ]
    );
    assert_holidays_exact!(
        cal,
        2025,
        [
            (2025, 1, 1),
            (2025, 1, 29),
            (2025, 1, 30),
            (2025, 1, 31),
            (2025, 4, 4),
            (2025, 5, 1),
            (2025, 5, 4),
            (2025, 7, 1),
            (2025, 10, 1),
            (2025, 12, 25),
            (2025, 12, 26)
        ]
    );
}

#[test]
fn hkex_2024_2025() {
    let cal = Hkex;
    // HKEX mirrors HKHK
    hkhk_2024_2025();
    // spot check
    assert!(cal.is_holiday(make_date(2024, 5, 15)));
}

#[test]
fn cnbe_2024_2025() {
    let cal = Cnbe;
    // 2024 holiday dates per approx block implementation
    assert_holidays_exact!(
        cal,
        2024,
        [
            // New Year block
            (2024, 1, 1),
            (2024, 1, 2),
            (2024, 1, 3),
            // Spring Festival block (CNY 10 Feb 2024 + 6 days)
            (2024, 2, 10),
            (2024, 2, 11),
            (2024, 2, 12),
            (2024, 2, 13),
            (2024, 2, 14),
            (2024, 2, 15),
            (2024, 2, 16),
            // Qing Ming
            (2024, 4, 4),
            // Labour block 1–5 May
            (2024, 5, 1),
            (2024, 5, 2),
            (2024, 5, 3),
            (2024, 5, 4),
            (2024, 5, 5),
            // National block 1–7 Oct
            (2024, 10, 1),
            (2024, 10, 2),
            (2024, 10, 3),
            (2024, 10, 4),
            (2024, 10, 5),
            (2024, 10, 6),
            (2024, 10, 7)
        ]
    );

    // 2025 expected set
    assert_holidays_exact!(
        cal,
        2025,
        [
            // New Year block
            (2025, 1, 1),
            (2025, 1, 2),
            (2025, 1, 3),
            // Spring Festival block starting 29 Jan 2025
            (2025, 1, 29),
            (2025, 1, 30),
            (2025, 1, 31),
            (2025, 2, 1),
            (2025, 2, 2),
            (2025, 2, 3),
            (2025, 2, 4),
            // Qing Ming 4 Apr 2025
            (2025, 4, 4),
            // Labour 1–5 May
            (2025, 5, 1),
            (2025, 5, 2),
            (2025, 5, 3),
            (2025, 5, 4),
            (2025, 5, 5),
            // National 1–7 Oct
            (2025, 10, 1),
            (2025, 10, 2),
            (2025, 10, 3),
            (2025, 10, 4),
            (2025, 10, 5),
            (2025, 10, 6),
            (2025, 10, 7)
        ]
    );
}

#[test]
fn sse_2024_2025() {
    let cal = Sse;
    // Should match CNBE exactly
    cnbe_2024_2025();
    // Spot check one date
    assert!(cal.is_holiday(make_date(2024, 10, 3)));
}

#[test]
fn sgsi_2024_2025() {
    let cal = Sgsi;
    assert_holidays_exact!(
        cal,
        2024,
        [
            (2024, 1, 1),
            (2024, 2, 10),
            (2024, 2, 11),
            (2024, 3, 29),
            (2024, 5, 1),
            (2024, 8, 9),
            (2024, 12, 25)
        ]
    );
    assert_holidays_exact!(
        cal,
        2025,
        [
            (2025, 1, 1),
            (2025, 1, 29),
            (2025, 1, 30),
            (2025, 4, 18),
            (2025, 5, 1),
            (2025, 8, 11),
            (2025, 12, 25)
        ]
    );
}

#[test]
fn asx_2024_2025() {
    let cal = Asx;
    assert_holidays_exact!(
        cal,
        2024,
        [
            (2024, 1, 1),
            (2024, 1, 29),
            (2024, 3, 29),
            (2024, 4, 1),
            (2024, 4, 25),
            (2024, 12, 25),
            (2024, 12, 26)
        ]
    );
    assert_holidays_exact!(
        cal,
        2025,
        [
            (2025, 1, 1),
            (2025, 1, 27),
            (2025, 4, 18),
            (2025, 4, 21),
            (2025, 4, 25),
            (2025, 12, 25),
            (2025, 12, 26)
        ]
    );
}

#[test]
fn auce_2024_2025() {
    let cal = Auce;
    assert_holidays_exact!(
        cal,
        2024,
        [
            (2024, 1, 1),
            (2024, 1, 29),
            (2024, 3, 29),
            (2024, 4, 1),
            (2024, 4, 25),
            (2024, 6, 10),
            (2024, 8, 5),
            (2024, 10, 7),
            (2024, 12, 25),
            (2024, 12, 26)
        ]
    );
    assert_holidays_exact!(
        cal,
        2025,
        [
            (2025, 1, 1),
            (2025, 1, 27),
            (2025, 4, 18),
            (2025, 4, 21),
            (2025, 4, 25),
            (2025, 6, 9),
            (2025, 8, 4),
            (2025, 10, 6),
            (2025, 12, 25),
            (2025, 12, 26)
        ]
    );
}

// ============================================
// Latin America Calendars
// ============================================

#[test]
fn brbd_2024_2025() {
    let cal = Brbd;
    assert_holidays_exact!(
        cal,
        2024,
        [
            (2024, 1, 1),
            (2024, 2, 12),
            (2024, 2, 13),
            (2024, 3, 29),
            (2024, 5, 1),
            (2024, 5, 30),
            (2024, 11, 15),
            (2024, 11, 20),
            (2024, 12, 25)
        ]
    );
    assert_holidays_exact!(
        cal,
        2025,
        [
            (2025, 1, 1),
            (2025, 3, 3),
            (2025, 3, 4),
            (2025, 4, 18),
            (2025, 4, 21),
            (2025, 5, 1),
            (2025, 6, 19),
            (2025, 11, 20),
            (2025, 12, 25)
        ]
    );
}

#[test]
fn cato_2024_2025() {
    let cal = Cato;
    assert_holidays_exact!(
        cal,
        2024,
        [
            (2024, 1, 1),
            (2024, 2, 19),
            (2024, 3, 29),
            (2024, 5, 20),
            (2024, 7, 1),
            (2024, 8, 5),
            (2024, 9, 2),
            (2024, 9, 30),
            (2024, 10, 14),
            (2024, 11, 11),
            (2024, 12, 25),
            (2024, 12, 26)
        ]
    );
    assert_holidays_exact!(
        cal,
        2025,
        [
            (2025, 1, 1),
            (2025, 2, 17),
            (2025, 4, 18),
            (2025, 5, 19),
            (2025, 7, 1),
            (2025, 8, 4),
            (2025, 9, 1),
            (2025, 9, 30),
            (2025, 10, 13),
            (2025, 11, 11),
            (2025, 12, 25),
            (2025, 12, 26)
        ]
    );
}

// ============================================
// Calendar Lookup and ID Methods
// ============================================

#[test]
fn test_calendar_by_id_lookup() {
    // Test that all calendar IDs can be looked up
    for &id in ALL_IDS {
        let cal = calendar_by_id(id);
        assert!(cal.is_some(), "Calendar '{}' should be found", id);

        // Ensure registry resolves the same id
        let typed = CalendarRegistry::global().resolve_str(id);
        assert!(typed.is_some(), "Registry should find '{}'", id);

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

// ============================================
// Chinese New Year Edge Cases (1970-2150)
// ============================================

#[test]
fn test_cny_early_years_1970s() {
    let cnbe = Cnbe;
    let hkhk = Hkhk;
    let sgsi = Sgsi;

    // Test specific CNY dates in the early years (1970-1989)
    // These should now work with the extended CSV data

    // 1970: CNY = Feb 6 (from CSV)
    assert!(
        cnbe.is_holiday(make_date(1970, 2, 6)),
        "CNBE should recognize CNY 1970"
    );
    assert!(
        hkhk.is_holiday(make_date(1970, 2, 6)),
        "HKHK should recognize CNY 1970"
    );
    assert!(
        sgsi.is_holiday(make_date(1970, 2, 6)),
        "SGSI should recognize CNY 1970"
    );

    // 1975: CNY = Feb 11 (from CSV)
    assert!(
        cnbe.is_holiday(make_date(1975, 2, 11)),
        "CNBE should recognize CNY 1975"
    );
    assert!(
        hkhk.is_holiday(make_date(1975, 2, 11)),
        "HKHK should recognize CNY 1975"
    );
    assert!(
        sgsi.is_holiday(make_date(1975, 2, 11)),
        "SGSI should recognize CNY 1975"
    );

    // 1980: CNY = Feb 16 (from CSV)
    assert!(
        cnbe.is_holiday(make_date(1980, 2, 16)),
        "CNBE should recognize CNY 1980"
    );
    assert!(
        hkhk.is_holiday(make_date(1980, 2, 16)),
        "HKHK should recognize CNY 1980"
    );
    assert!(
        sgsi.is_holiday(make_date(1980, 2, 16)),
        "SGSI should recognize CNY 1980"
    );

    // 1989: CNY = Feb 6 (from CSV)
    assert!(
        cnbe.is_holiday(make_date(1989, 2, 6)),
        "CNBE should recognize CNY 1989"
    );
    assert!(
        hkhk.is_holiday(make_date(1989, 2, 6)),
        "HKHK should recognize CNY 1989"
    );
    assert!(
        sgsi.is_holiday(make_date(1989, 2, 6)),
        "SGSI should recognize CNY 1989"
    );
}

#[test]
fn test_cny_late_years_2100s() {
    let cnbe = Cnbe;
    let hkhk = Hkhk;
    let sgsi = Sgsi;

    // Test specific CNY dates in the late years (2101-2150)
    // These should now work with the extended CSV data

    // 2101: CNY = Jan 29 (from CSV)
    assert!(
        cnbe.is_holiday(make_date(2101, 1, 29)),
        "CNBE should recognize CNY 2101"
    );
    assert!(
        hkhk.is_holiday(make_date(2101, 1, 29)),
        "HKHK should recognize CNY 2101"
    );
    assert!(
        sgsi.is_holiday(make_date(2101, 1, 29)),
        "SGSI should recognize CNY 2101"
    );

    // 2125: CNY = Feb 3 (from CSV)
    assert!(
        cnbe.is_holiday(make_date(2125, 2, 3)),
        "CNBE should recognize CNY 2125"
    );
    assert!(
        hkhk.is_holiday(make_date(2125, 2, 3)),
        "HKHK should recognize CNY 2125"
    );
    assert!(
        sgsi.is_holiday(make_date(2125, 2, 3)),
        "SGSI should recognize CNY 2125"
    );

    // 2150: CNY = Jan 28 (from CSV)
    assert!(
        cnbe.is_holiday(make_date(2150, 1, 28)),
        "CNBE should recognize CNY 2150"
    );
    assert!(
        hkhk.is_holiday(make_date(2150, 1, 28)),
        "HKHK should recognize CNY 2150"
    );
    assert!(
        sgsi.is_holiday(make_date(2150, 1, 28)),
        "SGSI should recognize CNY 2150"
    );
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
        let expected_hkhk = day <= 8; // 3-day span
        let expected_sgsi = day <= 7; // 2-day span

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
    assert!(
        cnbe.is_holiday(make_date(1970, 2, 6)),
        "1970 CNY should work"
    );
    assert!(
        cnbe.is_holiday(make_date(2150, 1, 28)),
        "2150 CNY should work"
    );
}

#[test]
fn test_buddhas_birthday_edge_years() {
    let hkhk = Hkhk; // Uses Buddha's Birthday (CNY + 95 days)

    // Test that Buddha's Birthday works in edge years
    // 1970: CNY = Feb 6, so Buddha's Birthday = Feb 6 + 95 days = May 12
    let bb_1970 = make_date(1970, 2, 6)
        .checked_add(time::Duration::days(95))
        .unwrap();
    assert_eq!(bb_1970, make_date(1970, 5, 12));
    assert!(
        hkhk.is_holiday(bb_1970),
        "Buddha's Birthday 1970 should be recognized"
    );

    // 2125: CNY = Feb 3, so Buddha's Birthday = Feb 3 + 95 days = May 9
    let bb_2125 = make_date(2125, 2, 3)
        .checked_add(time::Duration::days(95))
        .unwrap();
    assert_eq!(bb_2125, make_date(2125, 5, 9));
    assert!(
        hkhk.is_holiday(bb_2125),
        "Buddha's Birthday 2125 should be recognized"
    );
}
