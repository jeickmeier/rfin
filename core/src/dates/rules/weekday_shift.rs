#![allow(clippy::assign_op_pattern)]

use super::HolidayRule;
use time::{Date, Duration, Month, Weekday};

/// A base date (month+day) shifted to the nearest weekday **on/after** or
/// **on/before** that date.
///
/// Example: "Monday on or after 21 May" →
/// `WeekdayShift::on_or_after(Weekday::Monday, Month::May, 21)`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WeekdayShift {
    weekday: Weekday,
    month: Month,
    day: u8,
    direction: Direction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    After,
    Before,
}

impl WeekdayShift {
    /// Construct a rule for the given components.
    pub const fn new(weekday: Weekday, month: Month, day: u8, direction: Direction) -> Self {
        Self {
            weekday,
            month,
            day,
            direction,
        }
    }

    /// Rule: target weekday **on or after** the given date.
    pub const fn on_or_after(weekday: Weekday, month: Month, day: u8) -> Self {
        Self::new(weekday, month, day, Direction::After)
    }

    /// Rule: target weekday **on or before** the given date.
    pub const fn on_or_before(weekday: Weekday, month: Month, day: u8) -> Self {
        Self::new(weekday, month, day, Direction::Before)
    }

    fn date_in_year(&self, year: i32) -> Date {
        let mut date = Date::from_calendar_date(year, self.month, self.day).unwrap();
        match self.direction {
            Direction::After => {
                while date.weekday() != self.weekday {
                    date = date + Duration::DAY;
                }
            }
            Direction::Before => {
                while date.weekday() != self.weekday {
                    date = date - Duration::DAY;
                }
            }
        }
        date
    }
}

impl HolidayRule for WeekdayShift {
    fn applies(&self, date: Date) -> bool {
        self.date_in_year(date.year()) == date
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monday_on_or_after_may21() {
        let rule = WeekdayShift::on_or_after(Weekday::Monday, Month::May, 21);
        // 2025-05-21 is Wednesday → holiday Monday 26 May 2025
        let holiday = Date::from_calendar_date(2025, Month::May, 26).unwrap();
        assert!(rule.applies(holiday));
    }

    #[test]
    fn friday_on_or_before_july4() {
        let rule = WeekdayShift::on_or_before(Weekday::Friday, Month::July, 4);
        // 2026-07-04 is Saturday → Friday 3rd observed
        let holiday = Date::from_calendar_date(2026, Month::July, 3).unwrap();
        assert!(rule.applies(holiday));
    }
}
