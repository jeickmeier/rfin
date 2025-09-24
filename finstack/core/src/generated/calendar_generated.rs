// Auto-generated placeholder for calendars registry (no-op)
// This minimal file is used in environments where build-time generation is not
// available. It defines empty sets and expected symbols to keep the build working.

// Stable list of all calendar ids compiled into the crate
pub const ALL_IDS: &[&str] = &["target2", "gblo", "nyse"];

use time::{Date, Month, Weekday};

#[inline]
fn is_weekend(d: Date) -> bool {
    matches!(d.weekday(), Weekday::Saturday | Weekday::Sunday)
}

// TARGET2 (ECB) – minimal placeholder: mark Jan 1 as holiday and weekends
pub struct Target2;
impl Target2 {
    #[inline]
    pub const fn id(self) -> &'static str { "target2" }
}
impl crate::dates::calendar::HolidayCalendar for Target2 {
    fn is_holiday(&self, date: Date) -> bool {
        is_weekend(date) || (date.month() == Month::January && date.day() == 1)
    }
}

// GBLO – minimal placeholder: mark Jan 1 and Spring Bank Holiday 2025-05-26
pub struct Gblo;
impl Gblo {
    #[inline]
    pub const fn id(self) -> &'static str { "gblo" }
}
impl crate::dates::calendar::HolidayCalendar for Gblo {
    fn is_holiday(&self, date: Date) -> bool {
        is_weekend(date)
            || (date.month() == Month::January && date.day() == 1)
            || (date.year() == 2025 && date.month() == Month::May && date.day() == 26)
    }
}

// NYSE – minimal placeholder: mark Jan 1 as holiday and weekends
pub struct Nyse;
impl Nyse {
    #[inline]
    pub const fn id(self) -> &'static str { "nyse" }
}
impl crate::dates::calendar::HolidayCalendar for Nyse {
    fn is_holiday(&self, date: Date) -> bool {
        is_weekend(date) || (date.month() == Month::January && date.day() == 1)
    }
}

// Simple runtime registry resolver
#[inline]
pub fn calendar_by_id(code: &str) -> Option<&'static dyn crate::dates::calendar::HolidayCalendar> {
    match code {
        "target2" => Some(&Target2),
        "gblo" => Some(&Gblo),
        "nyse" => Some(&Nyse),
        _ => None,
    }
}