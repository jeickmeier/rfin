#![allow(clippy::assign_op_pattern)]

use super::HolidayRule;
use time::{Date, Duration, Month, Weekday};

/// Holiday falling on the *n*-th occurrence of a weekday in a given month.
///
/// * `n > 0` – nth occurrence from the **start** of the month (1 = first)
/// * `n < 0` – nth occurrence from the **end** of the month (-1 = last)
///
/// Example: US Labor Day ⇒ first Monday in September ⇒ `NthWeekday::new(1, Weekday::Monday, Month::September)`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NthWeekday {
    n: i8,
    weekday: Weekday,
    month: Month,
}

impl NthWeekday {
    /// Create rule for the *n*-th `weekday` in `month` (see struct docs).
    pub const fn new(n: i8, weekday: Weekday, month: Month) -> Self {
        Self { n, weekday, month }
    }

    /// First `weekday` in `month`.
    pub const fn first(weekday: Weekday, month: Month) -> Self {
        Self::new(1, weekday, month)
    }

    /// Last `weekday` in `month`.
    pub const fn last(weekday: Weekday, month: Month) -> Self {
        Self::new(-1, weekday, month)
    }

    /// Compute the holiday date for a given `year`.
    fn date_in_year(&self, year: i32) -> Date {
        if self.n > 0 {
            // Forward search from 1st of `month`.
            let mut date = Date::from_calendar_date(year, self.month, 1).unwrap();
            // find first occurrence of weekday
            while date.weekday() != self.weekday {
                date = date + Duration::DAY;
            }
            // advance (n-1) weeks
            date + Duration::weeks((self.n as i64 - 1).max(0))
        } else {
            // Backward search from last day of month.
            let next_month = if self.month == Month::December {
                (year + 1, Month::January)
            } else {
                (year, Month::try_from(self.month as u8 + 1).unwrap())
            };
            // first day of next month then -1 day gives last day of current month
            let mut date =
                Date::from_calendar_date(next_month.0, next_month.1, 1).unwrap() - Duration::DAY;
            while date.weekday() != self.weekday {
                date = date - Duration::DAY;
            }
            let pos = (-self.n) as i64; // 1 = last, 2 = second-last...
            date - Duration::weeks(pos - 1)
        }
    }
}

impl HolidayRule for NthWeekday {
    fn applies(&self, date: Date) -> bool {
        self.date_in_year(date.year()) == date
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_monday_feb() {
        let rule = NthWeekday::first(Weekday::Monday, Month::February);
        let date = Date::from_calendar_date(2025, Month::February, 3).unwrap();
        assert!(rule.applies(date));
    }

    #[test]
    fn last_friday_november() {
        let rule = NthWeekday::last(Weekday::Friday, Month::November);
        let date = Date::from_calendar_date(2024, Month::November, 29).unwrap(); // 2024-11-29 is last Friday
        assert!(rule.applies(date));
    }
}
