use super::HolidayRule;
use time::Date;

/// A holiday defined by an external year→Date calculator.
///
/// Supply a `fn(i32) -> Date` that returns the holiday date for the given year
/// (e.g. precalculated Chinese New Year algorithm).
#[derive(Clone, Copy)]
pub struct CustomFuncRule {
    calc: fn(i32) -> Date,
}

impl CustomFuncRule {
    /// Create a new `CustomFuncRule` wrapping a year→date calculator.
    ///
    /// The supplied function **must** return a valid holiday date for *every*
    /// Gregorian calendar year.
    #[must_use]
    pub const fn new(calc: fn(i32) -> Date) -> Self {
        Self { calc }
    }
}

impl core::fmt::Debug for CustomFuncRule {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("CustomFuncRule { .. }")
    }
}

impl HolidayRule for CustomFuncRule {
    fn applies(&self, date: Date) -> bool {
        let expected = (self.calc)(date.year());
        expected == date
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::Month;

    // simple dummy calc: always 29 Feb in leap years, else 1 Mar
    fn dummy(year: i32) -> Date {
        if (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0) {
            Date::from_calendar_date(year, Month::February, 29).unwrap()
        } else {
            Date::from_calendar_date(year, Month::March, 1).unwrap()
        }
    }

    #[test]
    fn custom_func_rule() {
        let rule = CustomFuncRule::new(dummy);
        let d = Date::from_calendar_date(2024, Month::February, 29).unwrap();
        assert!(rule.applies(d));
        let not = Date::from_calendar_date(2024, Month::February, 28).unwrap();
        assert!(!rule.applies(not));
    }
}
