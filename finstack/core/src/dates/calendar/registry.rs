//! Calendar registry for resolving calendars by identifier.
//!
//! Provides a global registry for looking up built-in holiday calendars by
//! their standard codes (e.g., "target2", "nyse", "gblo"). Supports both
//! string-based and typed ID resolution.

use crate::dates::calendar::business_days::HolidayCalendar;
use crate::dates::calendar::calendar_by_id;
use crate::dates::calendar::composite::{CompositeCalendar, CompositeMode};
use core::marker::PhantomData;
use std::sync::OnceLock;

/// Strongly-typed calendar identifier to avoid stringly-typed lookups.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CalendarId(pub &'static str);

/// Global, immutable registry for resolving calendars by typed ID.
pub struct CalendarRegistry<'a> {
    _marker: PhantomData<&'a ()>,
}

impl CalendarRegistry<'_> {
    /// Obtain the global registry instance.
    #[inline]
    pub fn global() -> &'static CalendarRegistry<'static> {
        static INSTANCE: OnceLock<CalendarRegistry> = OnceLock::new();
        INSTANCE.get_or_init(|| CalendarRegistry {
            _marker: PhantomData,
        })
    }

    /// Resolve a calendar by its lowercase code string (e.g., "gblo").
    #[inline]
    pub fn resolve_str(&self, code: &str) -> Option<&'static dyn HolidayCalendar> {
        calendar_by_id(code)
    }

    /// Resolve a calendar by `CalendarId`.
    #[inline]
    pub fn resolve(&self, id: CalendarId) -> Option<&'static dyn HolidayCalendar> {
        self.resolve_str(id.0)
    }

    /// Resolve many calendars into `storage` and return a composite using the specified mode.
    ///
    /// This avoids allocation leaks by letting the caller own the backing buffer used by the
    /// returned `CompositeCalendar` view.
    ///
    /// Usage:
    /// ```
    /// # use finstack_core::dates::calendar::registry::{CalendarId, CalendarRegistry};
    /// # use finstack_core::dates::calendar::composite::{CompositeCalendar, CompositeMode};
    /// # use finstack_core::dates::calendar::{GBLO, TARGET2};
    /// # use finstack_core::dates::HolidayCalendar;
    /// let ids = [CalendarId(TARGET2.id()), CalendarId(GBLO.id())];
    /// let regs = CalendarRegistry::global();
    /// let mut buf = Vec::new();
    /// let composite = regs.resolve_many(&ids, CompositeMode::Union, &mut buf);
    /// # let _ = composite.is_holiday(time::Date::from_calendar_date(2025, time::Month::January, 1).expect("Valid date"));
    /// ```
    #[inline]
    pub fn resolve_many<'s>(
        &self,
        ids: &[CalendarId],
        mode: CompositeMode,
        storage: &'s mut Vec<&'s dyn HolidayCalendar>,
    ) -> CompositeCalendar<'s> {
        storage.clear();
        storage.reserve(ids.len());
        for id in ids {
            if let Some(c) = self.resolve(*id) {
                storage.push(c);
            }
        }
        CompositeCalendar::with_mode(&storage[..], mode)
    }

    /// Resolve many calendars and return them as a `Vec<&dyn HolidayCalendar>`.
    ///
    /// This helper avoids leaking memory and lets callers build a
    /// `CompositeCalendar` by borrowing the returned `Vec` as a slice:
    ///
    /// ```
    /// # use finstack_core::dates::calendar::registry::{CalendarId, CalendarRegistry};
    /// # use finstack_core::dates::calendar::composite::{CompositeCalendar, CompositeMode};
    /// # use finstack_core::dates::calendar::{TARGET2, GBLO};
    /// # use finstack_core::dates::HolidayCalendar;
    /// let ids = [
    ///     CalendarId(TARGET2.id()),
    ///     CalendarId(GBLO.id()),
    /// ];
    /// let regs = CalendarRegistry::global();
    /// let v = regs.resolve_many_vec(&ids);
    /// let composite = CompositeCalendar::with_mode(&v[..], CompositeMode::Union);
    /// # let _ = composite.is_holiday(time::Date::from_calendar_date(2025, time::Month::January, 1).expect("Valid date"));
    /// ```
    #[inline]
    pub fn resolve_many_vec(&self, ids: &[CalendarId]) -> Vec<&'static dyn HolidayCalendar> {
        let mut out: Vec<&'static dyn HolidayCalendar> = Vec::with_capacity(ids.len());
        for id in ids {
            if let Some(c) = self.resolve(*id) {
                out.push(c);
            }
        }
        out
    }

    /// Return the list of available calendar identifiers.
    #[inline]
    pub fn available_ids(&self) -> &'static [&'static str] {
        crate::dates::available_calendars()
    }
}

// ----------------------------------------------------------------------------------------------
// Tests
// ----------------------------------------------------------------------------------------------
#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use crate::dates::calendar::{GBLO, TARGET2};
    use time::{Date, Month};

    #[test]
    fn resolve_many_vec_builds_composite_without_leak() {
        let ids = [CalendarId(TARGET2.id()), CalendarId(GBLO.id())];
        let regs = CalendarRegistry::global();
        let v = regs.resolve_many_vec(&ids);
        assert_eq!(v.len(), 2);

        let composite = CompositeCalendar::with_mode(&v[..], CompositeMode::Union);

        // Jan 1 is a holiday for both; union should be holiday.
        let d1 = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        assert!(composite.is_holiday(d1));

        // Date that is holiday in GBLO but not necessarily in Target2 (e.g., 26-May-2025)
        let d2 = Date::from_calendar_date(2025, Month::May, 26).expect("Valid test date");
        assert!(GBLO.is_holiday(d2));
        assert!(composite.is_holiday(d2));
    }
}
