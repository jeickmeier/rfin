use crate::dates::calendar::HolidayCalendar;
use time::Date;

use super::*;

/// Unified access to all built-in holiday calendars.
///
/// This replaces the need to instantiate the individual zero-sized
/// calendar structs directly (`Nyse`, `Gblo`, …).  The enum is `Copy`
/// and zero-sized, so it's as lightweight as the previous approach while
/// providing a single type that can be matched on or passed around.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BuiltInCal {
    Asx,
    Auce,
    Brbd,
    Cato,
    Chzh,
    Cnbe,
    Defr,
    Gblo,
    Hkex,
    Hkhk,
    Jpto,
    Jpx,
    Nyse,
    Sgsi,
    Sifma,
    Sse,
    Target2,
    Usny,
}

impl HolidayCalendar for BuiltInCal {
    #[inline]
    fn is_holiday(&self, date: Date) -> bool {
        match self {
            BuiltInCal::Asx => (Asx).is_holiday(date),
            BuiltInCal::Auce => (Auce).is_holiday(date),
            BuiltInCal::Brbd => (Brbd).is_holiday(date),
            BuiltInCal::Cato => (Cato).is_holiday(date),
            BuiltInCal::Chzh => (Chzh).is_holiday(date),
            BuiltInCal::Cnbe => (Cnbe).is_holiday(date),
            BuiltInCal::Defr => (Defr).is_holiday(date),
            BuiltInCal::Gblo => (Gblo).is_holiday(date),
            BuiltInCal::Hkex => (Hkex).is_holiday(date),
            BuiltInCal::Hkhk => (Hkhk).is_holiday(date),
            BuiltInCal::Jpto => (Jpto).is_holiday(date),
            BuiltInCal::Jpx => (Jpx).is_holiday(date),
            BuiltInCal::Nyse => Nyse::new().is_holiday(date),
            BuiltInCal::Sgsi => (Sgsi).is_holiday(date),
            BuiltInCal::Sifma => (Sifma).is_holiday(date),
            BuiltInCal::Sse => (Sse).is_holiday(date),
            BuiltInCal::Target2 => (Target2).is_holiday(date),
            BuiltInCal::Usny => Usny::new().is_holiday(date),
        }
    }
}

impl BuiltInCal {
    /// Returns the short identifier string (e.g. "nyse", "gblo").
    #[inline]
    pub const fn id(self) -> &'static str {
        match self {
            BuiltInCal::Asx => "asx",
            BuiltInCal::Auce => "auce",
            BuiltInCal::Brbd => "brbd",
            BuiltInCal::Cato => "cato",
            BuiltInCal::Chzh => "chzh",
            BuiltInCal::Cnbe => "cnbe",
            BuiltInCal::Defr => "defr",
            BuiltInCal::Gblo => "gblo",
            BuiltInCal::Hkex => "hkex",
            BuiltInCal::Hkhk => "hkhk",
            BuiltInCal::Jpto => "jpto",
            BuiltInCal::Jpx => "jpx",
            BuiltInCal::Nyse => "nyse",
            BuiltInCal::Sgsi => "sgsi",
            BuiltInCal::Sifma => "sifma",
            BuiltInCal::Sse => "sse",
            BuiltInCal::Target2 => "target2",
            BuiltInCal::Usny => "usny",
        }
    }
}
