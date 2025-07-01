use crate::dates::calendar::HolidayCalendar;
use crate::dates::rules::{FixedDate, NthWeekday, WeekdayShift, GoodFriday, HolidayRule};
use time::{Date, Month, Weekday};

/// Canadian banking CAD funds calendar (code: CATO).
#[derive(Debug, Clone, Copy, Default)]
pub struct Cato;

impl HolidayCalendar for Cato {
    fn is_holiday(&self, date: Date) -> bool {
        // Fixed-date holidays with Monday substitution
        FixedDate::new(Month::January, 1).observed_next_monday().applies(date) // New Year
            || FixedDate::new(Month::July, 1).observed_next_monday().applies(date) // Canada Day
            || FixedDate::new(Month::September, 30).observed_next_monday().applies(date) // Truth & Reconciliation
            || FixedDate::new(Month::November, 11).observed_next_monday().applies(date) // Remembrance
            || FixedDate::new(Month::December, 25).observed_next_monday().applies(date) // Christmas
            || FixedDate::new(Month::December, 26).observed_next_monday().applies(date) // Boxing
            // Family Day – 3rd Monday Feb
            || NthWeekday::new(3, Weekday::Monday, Month::February).applies(date)
            // Good Friday
            || GoodFriday.applies(date)
            // Victoria Day – Monday on or before 24 May (Mon preceding 25 May)
            || WeekdayShift::on_or_before(Weekday::Monday, Month::May, 25).applies(date)
            // Civic Holiday – 1st Monday Aug
            || NthWeekday::first(Weekday::Monday, Month::August).applies(date)
            // Labour Day – 1st Monday Sep
            || NthWeekday::first(Weekday::Monday, Month::September).applies(date)
            // Thanksgiving – 2nd Monday Oct
            || NthWeekday::new(2, Weekday::Monday, Month::October).applies(date)
    }
} 