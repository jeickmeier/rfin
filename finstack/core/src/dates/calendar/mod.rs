//! Holiday calendar DSL – unified design and semantics.
//!
//! ## Supported Date Range
//!
//! Holiday calendars are optimized for years **1970-2150** using generated bitsets.
//! Years outside this range fall back to runtime rule evaluation.
//!
//! **Chinese New Year (CNY) Coverage**: All CNY-dependent calendars (CNBE, HKHK, SGSI)
//! now support the full 1970-2150 range through externally-sourced data.
//! Previously, CNY was limited to 1990-2100, causing silent degradation outside that range.
//!
//! ## Semantics
//!
//! - "Holiday" refers to non-working dates as defined by a specific market
//!   calendar. Many calendars also label weekends as holidays for convenience,
//!   while some intentionally ignore weekends in `is_holiday`.
//! - Independent of the above, [`crate::dates::calendar::HolidayCalendar::is_business_day`]
//!   always treats Saturday/Sunday as non-business days and defers to
//!   `is_holiday` for market-specific closures.
//! - Prefer `is_business_day` for scheduling and adjustment logic. Use
//!   [`crate::dates::calendar::is_weekend`] if you need to only detect Saturday/Sunday.

pub mod composite;
pub mod core;
pub mod generated;
pub mod registry;
pub mod rule;

// Internal modules: macros and generated registry/types
mod generated_registry;
mod macros;

// Re-export commonly used items for ergonomic imports.
pub use core::{adjust, available_calendars, BusinessDayConvention, HolidayCalendar};
pub use rule::{Direction, Observed, Rule};

// Convenience alias so users can `use finstack_core::dates::calendar::Calendar`.
// We re-export the existing `HolidayCalendar` trait from the parent module.
// This keeps the public API surface small while allowing direct usage.
//
// Example:
//
// fn foo(cal: &impl Calendar) { /* ... */ }

pub use self::core::HolidayCalendar as Calendar;

// Re-export generated calendar types and registry helpers at this module root
// so existing paths like `crate::dates::calendar::calendar_by_id` remain valid.
pub use generated_registry::*;
