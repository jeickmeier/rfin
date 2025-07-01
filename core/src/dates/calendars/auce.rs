use crate::dates::calendar::HolidayCalendar;
use crate::dates::rules::{FixedDate, NthWeekday, WeekdayShift, GoodFriday, EasterMonday, HolidayRule};
use time::{Date, Month, Weekday};

/// Australia NSW inter-bank calendar (code: AUCE).
#[derive(Debug, Clone, Copy, Default)]
pub struct Auce;

impl HolidayCalendar for Auce {
    fn is_holiday(&self, date: Date) -> bool {
        // New Year's Day (Mon substitute)
        FixedDate::new(Month::January, 1).observed_next_monday().applies(date)
            // Australia Day – Monday on/after 26 Jan
            || WeekdayShift::on_or_after(Weekday::Monday, Month::January, 26).applies(date)
            // Good Friday / Easter Monday
            || GoodFriday.applies(date)
            || EasterMonday.applies(date)
            // Anzac Day – 25 Apr (no substitute)
            || FixedDate::new(Month::April, 25).applies(date)
            // King/Queen's Birthday – 2nd Monday June
            || NthWeekday::new(2, Weekday::Monday, Month::June).applies(date)
            // NSW Bank Holiday – 1st Monday August
            || NthWeekday::first(Weekday::Monday, Month::August).applies(date)
            // Labour Day – 1st Monday October
            || NthWeekday::first(Weekday::Monday, Month::October).applies(date)
            // Christmas / Boxing (Mon substitute)
            || FixedDate::new(Month::December, 25).observed_next_monday().applies(date)
            || FixedDate::new(Month::December, 26).observed_next_monday().applies(date)
    }
} 