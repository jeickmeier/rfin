//! Finance date helpers – facade over the `time` crate
//!
//! The `finstack_core::dates` module deliberately keeps its public surface **very small**:
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

mod date_extensions;

// Publicly re-export the extension traits so downstream crates can `use finstack_core::dates::DateExt`.
pub use date_extensions::{DateExt, OffsetDateTimeExt};

mod daycount;

pub use daycount::DayCount;

mod calendar;

// Re-export new holiday calendars at the top level for convenience
pub use calendar::{adjust, BusinessDayConvention, HolidayCalendar};

// The canonical public discovery helper
pub use calendar::available_calendars;

mod schedule_iter;

pub use schedule_iter::{schedule, Frequency, ScheduleBuilder, StubKind};

mod composite;

pub use composite::{CompositeCalendar, MergeMode};

mod imm;

pub use imm::{next_cds_date, next_imm, third_wednesday};

pub mod holiday;
// Re-export primary types for ergonomic use
pub use holiday::{Calendar as HolidayCalendarNew, Rule as HolidayRule};

// Re-export calendars directly for `finstack_core::dates::Target2` etc.
pub use holiday::calendars::*;

mod periods;
pub use periods::{build_periods, Period, PeriodId, PeriodKey};

pub mod calendars {
    #![allow(missing_docs)]
    pub use crate::dates::holiday::calendars::*;
}
