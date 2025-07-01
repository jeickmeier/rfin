#[cfg(test)]
use super::FixedDate;
use super::HolidayRule;
use time::Date;

/// A fixed-date holiday that only applies between `start_year` and `end_year`
/// (inclusive).
///
/// Example: UK Early-May bank holiday introduced in 1978 ⇒
/// `FixedDateRange::new(1978, i32::MAX, FixedDate::new(Month::May, 1))`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixedDateRange<R: HolidayRule> {
    start_year: i32,
    end_year: i32,
    rule: R,
}

impl<R: HolidayRule> FixedDateRange<R> {
    /// Construct a new ranged rule that is active between `start_year` and
    /// `end_year` (inclusive).
    #[must_use]
    pub const fn new(start_year: i32, end_year: i32, rule: R) -> Self {
        Self {
            start_year,
            end_year,
            rule,
        }
    }
}

impl<R: HolidayRule> HolidayRule for FixedDateRange<R> {
    fn applies(&self, date: Date) -> bool {
        let y = date.year();
        y >= self.start_year && y <= self.end_year && self.rule.applies(date)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::Month;

    #[test]
    fn within_range() {
        let rule = FixedDateRange::new(2000, 2005, FixedDate::new(Month::January, 1));
        let d = Date::from_calendar_date(2003, Month::January, 1).unwrap();
        assert!(rule.applies(d));
    }

    #[test]
    fn outside_range() {
        let rule = FixedDateRange::new(2000, 2005, FixedDate::new(Month::January, 1));
        let d = Date::from_calendar_date(2006, Month::January, 1).unwrap();
        assert!(!rule.applies(d));
    }
}
