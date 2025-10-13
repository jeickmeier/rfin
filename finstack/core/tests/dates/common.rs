use std::collections::HashSet;

use finstack_core::dates::calendar::business_days::HolidayCalendar;
use finstack_core::dates::Date;
use time::Month;

/// Helper to construct `Date` instances.
pub(crate) fn make_date(year: i32, month: u8, day: u8) -> Date {
    Date::from_calendar_date(year, Month::try_from(month).unwrap(), day).unwrap()
}

/// Simple in-memory calendar used for testing adjustments.
#[derive(Default, Clone)]
pub(crate) struct TestCal {
    holidays: HashSet<Date>,
}

impl TestCal {
    pub(crate) fn new() -> Self {
        Self {
            holidays: HashSet::new(),
        }
    }

    pub(crate) fn with_holiday(mut self, date: Date) -> Self {
        self.holidays.insert(date);
        self
    }
}

impl HolidayCalendar for TestCal {
    fn is_holiday(&self, date: Date) -> bool {
        self.holidays.contains(&date)
    }
}
