use super::HolidayRule;
use time::{Date, Duration, Weekday};

/// Holiday bridging: if a holiday falls on Thursday, declare Friday off as a bridge day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BridgeDay<R: HolidayRule> {
    preceding: R,
}

impl<R: HolidayRule> BridgeDay<R> {
    /// Create a new `BridgeDay` that observes the Friday immediately following
    /// a `preceding` holiday occurring on Thursday.
    #[must_use]
    pub const fn new(preceding: R) -> Self {
        Self { preceding }
    }
}

impl<R: HolidayRule> HolidayRule for BridgeDay<R> {
    fn applies(&self, date: Date) -> bool {
        if date.weekday() != Weekday::Friday {
            return false;
        }
        let thursday = date - Duration::DAY; // Thursday
        self.preceding.applies(thursday)
    }
}

/// In-lieu Monday: if a holiday falls on weekend, following Monday is holiday.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InLieuMonday<R: HolidayRule> {
    base: R,
}

impl<R: HolidayRule> InLieuMonday<R> {
    /// Create a new `InLieuMonday` wrapper for `base`. If `base` lands on
    /// Saturday or Sunday the following Monday becomes a holiday.
    #[must_use]
    pub const fn new(base: R) -> Self {
        Self { base }
    }
}

impl<R: HolidayRule> HolidayRule for InLieuMonday<R> {
    fn applies(&self, date: Date) -> bool {
        if date.weekday() != Weekday::Monday {
            return false;
        }
        let saturday = date - Duration::DAY * 2;
        let sunday = date - Duration::DAY;
        self.base.applies(saturday) || self.base.applies(sunday)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dates::rules::FixedDate;
    use time::Month;

    #[test]
    fn bridge_day_after_thursday_holiday() {
        let holiday = FixedDate::new(Month::May, 6); // pretend 2021-05-06 Thu
        let bridge = BridgeDay::new(holiday);
        let date = Date::from_calendar_date(2021, Month::May, 7).unwrap(); // Friday
        assert!(bridge.applies(date));
    }

    #[test]
    fn in_lieu_monday() {
        let holiday = FixedDate::new(Month::July, 4); // US Independence Day
        let in_lieu = InLieuMonday::new(holiday);
        // 2026-07-04 Saturday ⇒ Monday 6 July observed
        let date = Date::from_calendar_date(2026, Month::July, 6).unwrap();
        assert!(in_lieu.applies(date));
    }
}
