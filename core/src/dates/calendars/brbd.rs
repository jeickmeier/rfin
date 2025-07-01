use crate::dates::calendar::HolidayCalendar;
use crate::dates::rules::{FixedDate, GoodFriday, EasterOffset, HolidayRule};
use time::{Date, Month, Weekday};

/// Brazil B3 exchange holiday calendar (code: BRBD).
///
/// Rules (full-day closures only):
/// • New Year's Day (1 Jan)
/// • Carnival Monday & Tuesday (-49 & -48 days from Easter Monday)
/// • Good Friday (Easter Monday -3)
/// • Tiradentes (21 Apr)
/// • Labour Day (1 May)
/// • Corpus Christi (+59 days from Easter Monday)
/// • Independence Day (7 Sep)
/// • Our Lady Aparecida (12 Oct)
/// • All Souls (2 Nov) - only if weekday
/// • Republic Proclamation (15 Nov)
/// • Black Awareness Day (20 Nov - São Paulo)
/// • Christmas Day (25 Dec)
///
/// No weekend substitution: if the date falls on Saturday/Sunday the market
/// remains **open**.
#[derive(Debug, Clone, Copy, Default)]
pub struct Brbd;

impl HolidayCalendar for Brbd {
    fn is_holiday(&self, date: Date) -> bool {
        // Market ignores weekends entirely for holiday purposes.
        if matches!(date.weekday(), Weekday::Saturday | Weekday::Sunday) {
            return false;
        }

        // Fixed-date holidays (no substitution)
        if FixedDate::new(Month::January, 1).applies(date) // New Year
            || FixedDate::new(Month::April, 21).applies(date) // Tiradentes
            || FixedDate::new(Month::May, 1).applies(date) // Labour Day
            || FixedDate::new(Month::September, 7).applies(date) // Independence
            || FixedDate::new(Month::October, 12).applies(date) // Our Lady Aparecida
            || (FixedDate::new(Month::November, 2).applies(date)) // All Souls (already weekday-checked)
            || FixedDate::new(Month::November, 15).applies(date) // Republic
            || FixedDate::new(Month::November, 20).applies(date) // Black Awareness (São Paulo)
            || FixedDate::new(Month::December, 25).applies(date) // Christmas
        {
            return true;
        }

        // Moveable holidays relative to Easter
        if GoodFriday.applies(date) {
            return true;
        }

        // Carnival Monday (-49) and Tuesday (-48) from Easter Monday
        if EasterOffset::new(-49).applies(date) || EasterOffset::new(-48).applies(date) {
            return true;
        }

        // Corpus Christi (+59 days from Easter Monday)
        if EasterOffset::new(59).applies(date) {
            return true;
        }

        false
    }
} 