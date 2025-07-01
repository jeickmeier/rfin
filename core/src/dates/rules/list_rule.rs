use super::HolidayRule;
use time::Date;

/// Matches against a compile-time slice of specific dates (ad-hoc closures).
///
/// This rule is extremely lightweight: it just performs a linear search over
/// the slice.  For typical ad-hoc lists (\<= a few dozen entries) this is fine.
#[derive(Debug)]
pub struct ListRule<'a> {
    dates: &'a [Date],
}

impl<'a> ListRule<'a> {
    /// Build a new `ListRule` backed by the provided slice of dates.
    #[must_use]
    pub const fn new(dates: &'a [Date]) -> Self {
        Self { dates }
    }
}

impl HolidayRule for ListRule<'_> {
    fn applies(&self, date: Date) -> bool {
        self.dates.iter().copied().any(|d| d == date)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::Month;

    #[test]
    fn list_rule_matches() {
        let dates = [
            Date::from_calendar_date(2001, Month::September, 11).unwrap(),
            Date::from_calendar_date(2018, Month::December, 5).unwrap(),
        ];
        let rule = ListRule::new(&dates);
        let d = dates[0];
        assert!(rule.applies(d));
        let not = Date::from_calendar_date(2020, Month::January, 1).unwrap();
        assert!(!rule.applies(not));
    }
}
