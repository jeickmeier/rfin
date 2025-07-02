//! Composite holiday calendars combining multiple underlying calendars.
//!
//! This helper allows treating *multiple* [`HolidayCalendar`] implementations
//! as a single calendar using either
//! * *union* semantics (a day is a holiday if **any** sub-calendar is a
//!   holiday) – this is the default and matches the strictest view, and
//! * *intersection* semantics (a day is a holiday only if **all**
//!   sub-calendars are holidays).
//!
//! It is entirely allocation-free: a [`CompositeCalendar`] simply holds a
//! borrowed slice of `&dyn HolidayCalendar` trait objects.  This makes it
//! zero-sized for the common case where the slice lives on the stack.
//!
//! The type is deliberately lightweight and `no_std`-friendly so it can be
//! used inside `const` contexts once trait-object support for const fn lands.
//!
//! # Examples
//! ```
//! use rfin_core::dates::{CompositeCalendar, MergeMode, Target2, Gblo, HolidayCalendar};
//! use time::Date;
//!
//! let t2 = Target2::new();
//! let gb = Gblo::new();
//! let calendars = [&t2 as &dyn HolidayCalendar, &gb as &dyn HolidayCalendar];
//!
//! // Union (default) – treat the day as a holiday if *either* market is closed.
//! let cal_union = CompositeCalendar::merge(&calendars);
//! let jan1_2025 = Date::from_calendar_date(2025, time::Month::January, 1).unwrap();
//! assert!(cal_union.is_holiday(jan1_2025));
//!
//! // Intersection – holiday only if *both* markets are closed.
//! let cal_inter = CompositeCalendar::merge_with_mode(&calendars, MergeMode::Intersection);
//! let may26_2025 = Date::from_calendar_date(2025, time::Month::May, 26).unwrap();
//! assert!(cal_union.is_holiday(may26_2025)); // U.K. spring bank holiday
//! assert!(!cal_inter.is_holiday(may26_2025));
//! ```

#![allow(clippy::many_single_char_names)]

use super::HolidayCalendar;
use time::Date;

/// Merge logic for [`CompositeCalendar`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeMode {
    /// Union of holidays – *any* sub-calendar marks the date a holiday.
    Union,
    /// Intersection of holidays – date is a holiday only when *all*
    /// sub-calendars mark it a holiday.
    Intersection,
}

/// A lightweight view combining several holiday calendars.
#[derive(Clone, Copy)]
pub struct CompositeCalendar<'a> {
    calendars: &'a [&'a dyn HolidayCalendar],
    mode: MergeMode,
}

impl core::fmt::Debug for CompositeCalendar<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CompositeCalendar")
            .field("mode", &self.mode)
            .field("calendars_len", &self.calendars.len())
            .finish()
    }
}

impl<'a> CompositeCalendar<'a> {
    /// Create a new composite calendar using `mode` for merge semantics.
    #[must_use]
    pub const fn new(calendars: &'a [&'a dyn HolidayCalendar], mode: MergeMode) -> Self {
        Self { calendars, mode }
    }

    /// Convenience wrapper constructing a *union* composite (strict by default).
    #[must_use]
    pub const fn merge(calendars: &'a [&'a dyn HolidayCalendar]) -> Self {
        Self::new(calendars, MergeMode::Union)
    }

    /// Convenience wrapper constructing a composite with explicit `mode`.
    #[must_use]
    pub const fn merge_with_mode(
        calendars: &'a [&'a dyn HolidayCalendar],
        mode: MergeMode,
    ) -> Self {
        Self::new(calendars, mode)
    }
}

impl HolidayCalendar for CompositeCalendar<'_> {
    fn is_holiday(&self, date: Date) -> bool {
        match self.mode {
            MergeMode::Union => {
                // Empty slice ⇒ no holidays, so return false.
                self.calendars.iter().any(|c| c.is_holiday(date))
            }
            MergeMode::Intersection => {
                if self.calendars.is_empty() {
                    return false;
                }
                self.calendars.iter().all(|c| c.is_holiday(date))
            }
        }
    }
}

// ----------------------------------------------------------------------------------------------
// Tests
// ----------------------------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use crate::dates::calendars::{Gblo, Target2};
    use time::{Date, Month};

    #[test]
    fn union_vs_intersection() {
        let t2 = Target2::new();
        let gb = Gblo::new();
        let calendars = [&t2 as &dyn HolidayCalendar, &gb as &dyn HolidayCalendar];

        let cal_union = CompositeCalendar::merge(&calendars);
        let cal_inter = CompositeCalendar::merge_with_mode(&calendars, MergeMode::Intersection);

        // Date that is holiday in both calendars (New Year's Day)
        let d1 = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        assert!(cal_union.is_holiday(d1));
        assert!(cal_inter.is_holiday(d1));

        // Date that is holiday only in GBLO (Spring bank holiday 26-May-2025)
        let d2 = Date::from_calendar_date(2025, Month::May, 26).unwrap();
        assert!(Gblo::new().is_holiday(d2));
        assert!(!Target2::new().is_holiday(d2));

        assert!(cal_union.is_holiday(d2));
        assert!(!cal_inter.is_holiday(d2));
    }
}
