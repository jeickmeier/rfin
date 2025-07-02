//! Finance date helpers – facade over the `time` crate
//!
//! The `rfin_core::dates` module deliberately keeps its public surface **very small**:
//! it only re-exports the most commonly used value types from the [`time`](https://docs.rs/time)
//! crate so that downstream code does not have to depend on it directly.  This allows
//! the RustFin project to absorb upstream `time` version bumps behind a stable façade.
//!
//! The re-export is `#![no_std]`-compatible.  We compile the `time` crate with
//! `default-features = false`, which disables heavy std / formatting functionality while
//! still giving access to the core value types we need.
//!
//! # Re-exported items
//! * [`Date`]
//! * [`PrimitiveDateTime`]
//! * [`OffsetDateTime`]
//!
//! More specialised helpers (day-count, business-day logic, schedule builder, …) will
//! be introduced in follow-up pull-requests.  For now this module is intentionally
//! lightweight and free of additional abstractions.

// ----------------------------------------------------------------------------------
// Re-exports – keep list short & focused
// ----------------------------------------------------------------------------------

pub use time::{Date, OffsetDateTime, PrimitiveDateTime};

// In the future we might expose the `time::macros` helpers behind a feature flag.  Until
// then consumers can `use time::macros::*` directly if needed.

mod ext;

// Publicly re-export the extension traits so downstream crates can `use rfin_core::dates::DateExt`.
pub use ext::{DateExt, OffsetDateTimeExt};

mod daycount;

pub use daycount::DayCount;

mod calendar;

pub mod calendars;
pub use calendars::*;

pub mod rules;
pub use rules::*;

pub use calendar::{adjust, BusDayConv, HolidayCalendar};

pub use calendar::available_calendars;

mod schedule;

pub use schedule::{ScheduleBuilder, Frequency, Schedule, StubRule};

mod composite;

pub use composite::{CompositeCalendar, MergeMode};

mod imm;

pub use imm::{third_wednesday, next_imm, next_cds_date};
