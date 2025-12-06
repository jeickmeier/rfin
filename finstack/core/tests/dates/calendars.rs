//! Calendar tests (sample-based to reduce duplication)

use super::common::make_date;
use finstack_core::dates::calendar::{
    calendar_by_id, ALL_IDS, ASX as Asx, AUCE as Auce, BRBD as Brbd, CATO as Cato, CHZH as Chzh,
    CME as Cme, CNBE as Cnbe, DEFR as Defr, GBLO as Gblo, HKEX as Hkex, HKHK as Hkhk, NYSE as Nyse,
    SGSI as Sgsi, SIFMA as Sifma, SSE as Sse, TARGET2 as Target2, USNY as Usny,
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
                expected_count: None,
                must_have: &[(2024, 2, 10), (2024, 5, 1), (2024, 10, 1)],
            },
            YearCheck {
                year: 2025,
                expected_count: None,
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

    for &(y, m, d) in &[(1970, 2, 6), (1975, 2, 11), (1980, 2, 16), (1989, 2, 6)] {
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
