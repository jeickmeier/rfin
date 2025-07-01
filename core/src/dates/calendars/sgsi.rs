use crate::dates::calendar::HolidayCalendar;
use crate::dates::rules::*;
use time::{Date, Month};

/// Singapore inter-bank calendar (code: SGSI).
///
/// Implemented subset (core public holidays):
/// • New Year's Day (1 Jan, Monday substitution)
/// • Labour Day (1 May, Monday substitution)
/// • National Day (9 Aug, Monday substitution)
/// • Christmas Day (25 Dec, Monday substitution)
/// • Chinese New Year – 1st *two* days
/// • Good Friday
///
/// Remaining variable religious holidays (Hari Raya Puasa/Haji, Vesak, Deepavali)
/// and election days are left as TODO.
#[derive(Debug, Clone, Copy, Default)]
pub struct Sgsi;

impl HolidayCalendar for Sgsi {
    fn is_holiday(&self, date: Date) -> bool {
        // Fixed Gregorian holidays with Monday substitution
        FixedDate::new(Month::January, 1).observed_next_monday().applies(date) // New Year
            || FixedDate::new(Month::May, 1).observed_next_monday().applies(date) // Labour Day
            || FixedDate::new(Month::August, 9).observed_next_monday().applies(date) // National Day
            || FixedDate::new(Month::December, 25).observed_next_monday().applies(date) // Christmas
            // Chinese New Year – first 2 days
            || HolidaySpan::new(ChineseNewYear, 2).applies(date)
            // Good Friday
            || GoodFriday.applies(date)
            // TODO: Hari Raya Puasa, Hari Raya Haji, Vesak, Deepavali, election days.
    }
} 