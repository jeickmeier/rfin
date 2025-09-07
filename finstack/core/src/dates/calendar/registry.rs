//! Calendar registry exposing typed resolution.
//!
//! Calendar registry that resolves calendars by identifier (lowercase string)
//! to the corresponding built-in calendar implementations. It uses the
//! generated registry from `dates::calendar` for the authoritative list.

use crate::dates::calendar::core::HolidayCalendar;
use crate::dates::calendar::calendar_by_id;
use core::marker::PhantomData;
use once_cell::sync::OnceCell;

/// Global, immutable registry for resolving calendars by typed ID.
pub struct CalendarRegistry<'a> {
    _marker: PhantomData<&'a ()>,
}

impl CalendarRegistry<'_> {
    /// Obtain the global registry instance.
    #[inline]
    pub fn global() -> &'static CalendarRegistry<'static> {
        static INSTANCE: OnceCell<CalendarRegistry> = OnceCell::new();
        INSTANCE.get_or_init(|| CalendarRegistry { _marker: PhantomData })
    }

    /// Resolve a calendar by its lowercase code string (e.g., "gblo").
    #[inline]
    pub fn resolve_str(&self, code: &str) -> Option<&'static dyn HolidayCalendar> {
        calendar_by_id(code)
    }

    /// Return the list of available calendar identifiers.
    #[inline]
    pub fn available_ids(&self) -> &'static [&'static str] {
        crate::dates::calendar::available_calendars()
    }
}


