//! Common test helpers and utilities.

use std::collections::HashSet;
use time::Date;
use finstack_core::dates::HolidayCalendar;

/// Approximate equality for floating point values.
#[allow(dead_code)]
pub fn approx_eq(a: f64, b: f64, eps: f64) -> bool {
    (a - b).abs() < eps
}

/// Test holiday calendar that treats specific dates as holidays.
#[derive(Debug, Clone)]
pub struct TestCal {
    pub holidays: HashSet<Date>,
}

impl TestCal {
    pub fn new() -> Self {
        Self {
            holidays: HashSet::new(),
        }
    }

    #[allow(dead_code)]
    pub fn with_holiday(mut self, date: Date) -> Self {
        self.holidays.insert(date);
        self
    }

    #[allow(dead_code)]
    pub fn with_holidays(mut self, dates: &[Date]) -> Self {
        for &date in dates {
            self.holidays.insert(date);
        }
        self
    }
}

impl Default for TestCal {
    fn default() -> Self {
        Self::new()
    }
}

impl HolidayCalendar for TestCal {
    fn is_holiday(&self, date: Date) -> bool {
        // Weekend check is handled by the trait's default is_business_day implementation
        self.holidays.contains(&date)
    }
}

/// Helper to create dates more concisely in tests.
#[allow(dead_code)]
pub fn make_date(year: i32, month: u8, day: u8) -> Date {
    Date::from_calendar_date(year, time::Month::try_from(month).unwrap(), day).unwrap()
}

/// Simple test expression context for mapping column names to indices.
#[derive(Debug, Clone)]
pub struct TestExprCtx {
    pub name_to_index: std::collections::HashMap<String, usize>,
}

impl TestExprCtx {
    pub fn new() -> Self {
        Self {
            name_to_index: std::collections::HashMap::new(),
        }
    }

    #[allow(dead_code)]
    pub fn with_column(mut self, name: &str, index: usize) -> Self {
        self.name_to_index.insert(name.to_string(), index);
        self
    }
}

impl Default for TestExprCtx {
    fn default() -> Self {
        Self::new()
    }
}

impl finstack_core::expr::ExpressionContext for TestExprCtx {
    fn resolve_index(&self, name: &str) -> Option<usize> {
        self.name_to_index.get(name).copied()
    }
}
