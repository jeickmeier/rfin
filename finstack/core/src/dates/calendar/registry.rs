//! Calendar registry exposing typed resolution.
//!
//! Calendar registry that resolves calendars by identifier (lowercase string)
//! to the corresponding built-in calendar implementations. It uses the
//! generated registry from `dates::calendar` for the authoritative list.

use crate::dates::calendar::calendar_by_id;
use crate::dates::calendar::composite::{CompositeCalendar, CompositeMode};
use crate::dates::calendar::core::HolidayCalendar;
use core::marker::PhantomData;
use once_cell::sync::OnceCell;

/// Strongly-typed calendar identifier to avoid stringly-typed lookups.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CalendarId(pub &'static str);

/// Global, immutable registry for resolving calendars by typed ID.
pub struct CalendarRegistry<'a> {
    _marker: PhantomData<&'a ()>,
}

impl CalendarRegistry<'_> {
    /// Obtain the global registry instance.
    #[inline]
    pub fn global() -> &'static CalendarRegistry<'static> {
        static INSTANCE: OnceCell<CalendarRegistry> = OnceCell::new();
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

    /// Resolve many calendars and return a composite using the specified mode.
    ///
    /// Note: The caller must own the backing slice for the lifetime of the composite view.
    #[inline]
    pub fn resolve_many<'s>(
        &self,
        ids: &'s [CalendarId],
        mode: CompositeMode,
    ) -> CompositeCalendar<'s> {
        // Materialize resolved calendars into a temporary Vec<&dyn HolidayCalendar> that
        // shares the same lifetime as the provided `ids` slice. The Vec is then borrowed
        // as a slice for the CompositeCalendar. Caller is responsible for keeping `ids`
        // (and thus this Vec) alive for as long as the composite is used.
        //
        // To preserve lifetimes, we construct the vector here and then leak it, which is
        // acceptable for long-lived registries and tiny slices. For short-lived use, prefer
        // building a local `Vec` and constructing `CompositeCalendar` directly at the call site.
        let mut tmp: Vec<&'static dyn HolidayCalendar> = Vec::with_capacity(ids.len());
        for id in ids {
            if let Some(c) = self.resolve(*id) {
                tmp.push(c);
            }
        }
        let leaked: &'s [&'s dyn HolidayCalendar] = Box::leak(tmp.into_boxed_slice());
        CompositeCalendar::with_mode(leaked, mode)
    }

    /// Return the list of available calendar identifiers.
    #[inline]
    pub fn available_ids(&self) -> &'static [&'static str] {
        crate::dates::calendar::available_calendars()
    }
}
