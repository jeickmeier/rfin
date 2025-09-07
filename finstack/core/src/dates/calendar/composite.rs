//! Composite holiday calendars combining multiple underlying calendars.
//!
//! This helper allows treating *multiple* [`HolidayCalendar`] implementations
//! as a single calendar. By default it uses a strict
//! **union** of holidays (a day is a holiday if any sub-calendar is a holiday).
//! Optionally, you can request **intersection** semantics (a day is a holiday
//! only if all sub-calendars are holidays) via a boolean flag.
//!
//! It is entirely allocation-free: a [`CompositeCalendar`] simply holds a
//! borrowed slice of `&dyn HolidayCalendar` trait objects.  This makes it
//! zero-sized for the common case where the slice lives on the stack.
//!
//! The type is deliberately lightweight so it can be used inside `const`
//! contexts once trait-object support for const fn lands.
//!
//! # Examples
//! ```
//! use finstack_core::dates::{CompositeCalendar, HolidayCalendar};
//! use finstack_core::dates::calendar::{Target2, Gblo};
//! use time::Date;
//!
//! let t2 = Target2;
//! let gb = Gblo;
//! let calendars = [&t2 as &dyn HolidayCalendar, &gb as &dyn HolidayCalendar];
//!
//! // Union (default) – treat the day as a holiday if *either* market is closed.
//! let cal_union = CompositeCalendar::new(&calendars);
//! let jan1_2025 = Date::from_calendar_date(2025, time::Month::January, 1).unwrap();
//! assert!(cal_union.is_holiday(jan1_2025));
//!
//! // Intersection – holiday only if *both* markets are closed.
//! let cal_inter = CompositeCalendar::merge_with_intersection(&calendars, true);
//! let may26_2025 = Date::from_calendar_date(2025, time::Month::May, 26).unwrap();
//! assert!(cal_union.is_holiday(may26_2025)); // U.K. spring bank holiday
//! assert!(!cal_inter.is_holiday(may26_2025));
//! ```

use crate::dates::calendar::core::HolidayCalendar;
use time::Date;

/// A lightweight view combining several holiday calendars.
#[derive(Clone, Copy)]
pub struct CompositeCalendar<'a> {
    calendars: &'a [&'a dyn HolidayCalendar],
    intersection: bool,
}

impl core::fmt::Debug for CompositeCalendar<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CompositeCalendar")
            .field("intersection", &self.intersection)
            .field("calendars_len", &self.calendars.len())
            .finish()
    }
}

impl<'a> CompositeCalendar<'a> {
    /// Create a new composite calendar using union semantics (strict by default).
    #[must_use]
    pub const fn new(calendars: &'a [&'a dyn HolidayCalendar]) -> Self {
        Self {
            calendars,
            intersection: false,
        }
    }

    // Single canonical constructor is `new`; former `merge` alias removed for simplicity.

    /// Construct a composite calendar with an explicit intersection flag.
    /// When `intersection` is `true`, a date is a holiday only if all sub-calendars
    /// mark it as a holiday. When `false`, union semantics are used.
    #[must_use]
    pub const fn merge_with_intersection(
        calendars: &'a [&'a dyn HolidayCalendar],
        intersection: bool,
    ) -> Self {
        Self {
            calendars,
            intersection,
        }
    }
}

impl HolidayCalendar for CompositeCalendar<'_> {
    fn is_holiday(&self, date: Date) -> bool {
        if self.intersection {
            if self.calendars.is_empty() {
                return false;
            }
            self.calendars.iter().all(|c| c.is_holiday(date))
        } else {
            // Empty slice ⇒ no holidays, so return false.
            self.calendars.iter().any(|c| c.is_holiday(date))
        }
    }
}

// ----------------------------------------------------------------------------------------------
// Tests
// ----------------------------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use crate::dates::calendar::{Gblo, Target2};
    use time::{Date, Month};

    #[test]
    fn union_vs_intersection() {
        let t2 = Target2;
        let gb = Gblo;
        let calendars = [&t2 as &dyn HolidayCalendar, &gb as &dyn HolidayCalendar];

        let cal_union = CompositeCalendar::new(&calendars);
        let cal_inter = CompositeCalendar::merge_with_intersection(&calendars, true);

        // Date that is holiday in both calendars (New Year's Day)
        let d1 = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        assert!(cal_union.is_holiday(d1));
        assert!(cal_inter.is_holiday(d1));

        // Date that is holiday only in GBLO (Spring bank holiday 26-May-2025)
        let d2 = Date::from_calendar_date(2025, Month::May, 26).unwrap();
        assert!(Gblo.is_holiday(d2));
        assert!(!Target2.is_holiday(d2));

        assert!(cal_union.is_holiday(d2));
        assert!(!cal_inter.is_holiday(d2));
    }
}
