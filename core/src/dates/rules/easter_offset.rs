use super::HolidayRule;
use time::{Date, Duration, Month};

/// Internal helper: compute Easter Monday date for a given year (Gregorian calendar).
/// Copied from `easter.rs` but kept private so we don\'t make the algorithm public twice.
fn easter_monday(year: i32) -> Date {
    let a = year % 19;
    let b = year / 100;
    let c = year % 100;
    let d = b / 4;
    let e = b % 4;
    let f = (b + 8) / 25;
    let g = (b - f + 1) / 3;
    let h = (19 * a + b - d - g + 15) % 30;
    let i = c / 4;
    let k = c % 4;
    let l = (32 + 2 * e + 2 * i - h - k) % 7;
    let m = (a + 11 * h + 22 * l) / 451;
    let month_num = (h + l - 7 * m + 114) / 31; // 3=March 4=April
    let day = ((h + l - 7 * m + 114) % 31) + 1; // Easter Sunday
    let month = if month_num == 3 {
        Month::March
    } else {
        Month::April
    };
    let easter_sunday = Date::from_calendar_date(year, month, day as u8).unwrap();
    easter_sunday + Duration::DAY
}

/// Generic rule representing a day that is an offset (in days) from Easter Monday.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EasterOffset {
    /// Days relative to Easter Monday (e.g. -3 = Good Friday, +39 = Ascension Thursday).
    offset_days: i16,
}

impl EasterOffset {
    /// Create a new rule `offset` days away from Easter Monday.
    pub const fn new(offset_days: i16) -> Self {
        Self { offset_days }
    }
}

impl HolidayRule for EasterOffset {
    fn applies(&self, date: Date) -> bool {
        let easter_mon = easter_monday(date.year());
        let target = easter_mon + Duration::days(self.offset_days as i64);
        date == target
    }
}

/// Easter Monday (offset 0 days from itself) – kept as a dedicated type for ergonomic imports.
#[derive(Debug, Clone, Copy, Default)]
pub struct EasterMonday;

impl HolidayRule for EasterMonday {
    fn applies(&self, date: Date) -> bool {
        EasterOffset::new(0).applies(date)
    }
}

// -----------------------------------------------------------------------------
// Convenience wrappers for common holidays
// -----------------------------------------------------------------------------

/// Good Friday (Easter Monday - 3 days).
#[derive(Debug, Clone, Copy, Default)]
pub struct GoodFriday;

impl HolidayRule for GoodFriday {
    fn applies(&self, date: Date) -> bool {
        EasterOffset::new(-3).applies(date)
    }
}

/// Ascension Thursday (Easter Monday + 39 days).
#[derive(Debug, Clone, Copy, Default)]
pub struct AscensionThursday;

impl HolidayRule for AscensionThursday {
    fn applies(&self, date: Date) -> bool {
        EasterOffset::new(38).applies(date)
    }
}

/// Pentecost Monday (Easter Monday + 49 days, or Easter Monday + 49?) Wait Pentecost is Sunday after 49, Monday +50?
/// Actually Easter Sunday + 49 = Pentecost Sunday, so Monday is +50 from Easter Sunday, +49 from Easter Monday.
#[derive(Debug, Clone, Copy, Default)]
pub struct PentecostMonday;

impl HolidayRule for PentecostMonday {
    fn applies(&self, date: Date) -> bool {
        EasterOffset::new(49).applies(date)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn good_friday_2025() {
        // Easter 2025 Monday is 2025-04-21, so Good Friday 2025-04-18
        let d = Date::from_calendar_date(2025, Month::April, 18).unwrap();
        assert!(GoodFriday.applies(d));
    }

    #[test]
    fn ascension_2026() {
        // Easter Monday 2026 is 2026-04-06; +39 = 2026-05-15 (but Thursday). Wait calc.
        let d = Date::from_calendar_date(2026, Month::May, 14).unwrap();
        assert!(AscensionThursday.applies(d));
    }

    #[test]
    fn pentecost_2024() {
        // Easter Monday 2024 is 2024-04-01; +49 = 2024-05-20
        let d = Date::from_calendar_date(2024, Month::May, 20).unwrap();
        assert!(PentecostMonday.applies(d));
    }
}
