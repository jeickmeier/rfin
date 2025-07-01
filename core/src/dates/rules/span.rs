use super::HolidayRule;
use time::{Date, Duration};

/// A block of consecutive holidays starting from `start_rule` and spanning `days` days.
/// Example: if a multi-day festival begins on a fixed date rule, you can wrap it in
/// `HolidaySpan::new(start_rule, 3)` to mark start date plus the next 2 days.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HolidaySpan<R: HolidayRule> {
    start: R,
    days: u8,
}

impl<R: HolidayRule> HolidaySpan<R> {
    /// Construct a new `HolidaySpan` starting at `start` and covering `days`
    /// consecutive calendar days (including the start day).
    #[must_use]
    pub const fn new(start: R, days: u8) -> Self {
        Self { start, days }
    }
}

impl<R: HolidayRule> HolidayRule for HolidaySpan<R> {
    fn applies(&self, date: Date) -> bool {
        for offset in 0..self.days as i64 {
            let candidate = date - Duration::days(offset);
            if self.start.applies(candidate) {
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dates::rules::FixedDate;
    use time::Month;

    #[test]
    fn span_applies() {
        let start = FixedDate::new(Month::January, 1);
        let span = HolidaySpan::new(start, 3); // 1 Jan + 2 following days
        let d2 = Date::from_calendar_date(2025, Month::January, 2).unwrap();
        assert!(span.applies(d2));
        let d4 = Date::from_calendar_date(2025, Month::January, 4).unwrap();
        assert!(!span.applies(d4));
    }
}
