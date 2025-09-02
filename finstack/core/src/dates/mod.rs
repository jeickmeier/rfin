//! Finance date helpers – facade over the `time` crate
//!
//! The `finstack_core::dates` module deliberately keeps its public surface **very small**:
//! it only re-exports the most commonly used value types from the [`time`](https://docs.rs/time)
//! crate so that downstream code does not have to depend on it directly.  This allows
//! the RustFin project to absorb upstream `time` version bumps behind a stable façade.
//!
//! We compile the `time` crate with `default-features = false` to keep dependencies
//! lean while still providing the core value types we need.
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

pub use daycount::{DayCount, Thirty360Convention};

mod calendar;

// Re-export new holiday calendars at the top level for convenience
pub use calendar::{adjust, BusinessDayConvention, HolidayCalendar};

// The canonical public discovery helper
pub use calendar::available_calendars;

mod schedule_iter;

pub use schedule_iter::{schedule, try_schedule, Frequency, ScheduleBuilder, StubKind};

mod composite;

pub use composite::CompositeCalendar;

mod imm;

pub use imm::{
    imm_option_expiry, next_cds_date, next_equity_option_expiry, next_imm, 
    next_imm_option_expiry, third_friday, third_wednesday,
};

pub mod holiday;
// Keep holiday DSL under `dates::holiday`; avoid redundant aliases at root.
// Re-export calendars directly for `finstack_core::dates::Target2` etc.
pub use holiday::calendars::*;

mod periods;
pub use periods::{build_fiscal_periods, build_periods, FiscalConfig, Period, PeriodId, PeriodKey};

pub mod utils;
pub use utils::{add_months, date_to_days_since_epoch, days_since_epoch_to_date, is_leap_year};

pub mod calendars {
    #![allow(missing_docs)]
    pub use crate::dates::holiday::calendars::*;
}
