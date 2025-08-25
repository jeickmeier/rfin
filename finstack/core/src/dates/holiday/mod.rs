//! Holiday calendar DSL – new unified design.

pub mod calendars;
pub mod rule;

// Re-export commonly used items for ergonomic imports.
pub use rule::{Direction, Observed, Rule};

// Convenience alias so users can `use rfin_core::dates::holiday::Calendar`.
// We re-export the existing `HolidayCalendar` trait from the parent module.
// This keeps the public API surface small while allowing direct usage.
//
// Example:
// use rfin_core::dates::holiday::{Rule, Calendar};
//
// fn foo(cal: &impl Calendar) { /* ... */ }

pub use crate::dates::calendar::HolidayCalendar as Calendar;

// Re-export most used calendars at holiday root level
pub use calendars::*;
