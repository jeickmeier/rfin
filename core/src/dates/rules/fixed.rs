use super::HolidayRule;
use time::{Date, Duration, Month, Weekday};

/// How a fixed-date holiday is observed when it falls on a weekend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Observed {
    /// No observation adjustment – holiday is **only** the given calendar date.
    None,
    /// If Saturday → following Monday, if Sunday → following Monday.
    NextMonday,
    /// If Saturday → previous Friday, if Sunday → following Monday.
    FriIfSatMonIfSun,
}

/// A holiday occurring on a fixed calendar date every year (e.g. 1 January).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixedDate {
    month: Month,
    day: u8,
    observed: Observed,
}

impl FixedDate {
    /// Creates a new fixed-date holiday without observation rules.
    pub const fn new(month: Month, day: u8) -> Self {
        Self {
            month,
            day,
            observed: Observed::None,
        }
    }

    /// Observed on the following Monday when falling on a weekend.
    #[must_use]
    pub const fn observed_next_monday(self) -> Self {
        Self {
            observed: Observed::NextMonday,
            ..self
        }
    }

    /// Observed Friday if Saturday / Monday if Sunday.
    #[must_use]
    pub const fn observed_weekend(self) -> Self {
        Self {
            observed: Observed::FriIfSatMonIfSun,
            ..self
        }
    }

    /// Helper: returns the observed date in the given year.
    fn observed_date(&self, year: i32) -> Date {
        // Safe unwrap: constructor guarantees valid month/day.
        let base = Date::from_calendar_date(year, self.month, self.day).unwrap();
        match self.observed {
            Observed::None => base,
            Observed::NextMonday => match base.weekday() {
                Weekday::Saturday => base + Duration::DAY * 2,
                Weekday::Sunday => base + Duration::DAY,
                _ => base,
            },
            Observed::FriIfSatMonIfSun => match base.weekday() {
                Weekday::Saturday => base - Duration::DAY,
                Weekday::Sunday => base + Duration::DAY,
                _ => base,
            },
        }
    }
}

impl HolidayRule for FixedDate {
    fn applies(&self, date: Date) -> bool {
        self.observed_date(date.year()) == date
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_fixed_date() {
        let rule = FixedDate::new(Month::January, 1);
        let d = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        assert!(rule.applies(d));
        let not = Date::from_calendar_date(2025, Month::January, 2).unwrap();
        assert!(!rule.applies(not));
    }

    #[test]
    fn next_monday_observance() {
        let rule = FixedDate::new(Month::January, 1).observed_next_monday();
        // 2027-01-01 is Friday – observed on the day
        assert!(rule.applies(Date::from_calendar_date(2027, Month::January, 1).unwrap()));
        // 2028-01-01 is Saturday – observed Monday 3 Jan
        assert!(rule.applies(Date::from_calendar_date(2028, Month::January, 3).unwrap()));
    }

    #[test]
    fn fri_mon_observance() {
        let rule = FixedDate::new(Month::December, 25).observed_weekend();
        // 2027-12-25 is Saturday → observed Friday 24
        assert!(rule.applies(Date::from_calendar_date(2027, Month::December, 24).unwrap()));
        // 2026-12-25 is Friday → observed on the day
        assert!(rule.applies(Date::from_calendar_date(2026, Month::December, 25).unwrap()));
        // 2028-12-25 is Monday → observed Monday
        assert!(rule.applies(Date::from_calendar_date(2028, Month::December, 25).unwrap()));
    }
}
