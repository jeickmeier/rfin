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

mod date_extensions;

// Publicly re-export the extension traits so downstream crates can `use finstack_core::dates::DateExt`.
pub use date_extensions::{DateExt, OffsetDateTimeExt};

mod daycount;

pub use daycount::{DayCount, DayCountCtx, Thirty360Convention};

// Re-export new holiday calendars at the top level for convenience
pub use calendar::core::{adjust, BusinessDayConvention, HolidayCalendar};

// The canonical public discovery helper
pub use calendar::core::available_calendars;

mod schedule_iter;

pub use schedule_iter::{Frequency, ScheduleBuilder, StubKind};

pub use calendar::composite::CompositeCalendar;

mod imm;

pub use imm::{
    imm_option_expiry, next_cds_date, next_equity_option_expiry, next_imm, next_imm_option_expiry,
    third_friday, third_wednesday,
};

pub mod calendar;
pub use calendar::registry::CalendarRegistry;

mod periods;
pub use periods::{build_fiscal_periods, build_periods, FiscalConfig, Period, PeriodId, PeriodKey};

pub mod utils;
pub use utils::{add_months, date_to_days_since_epoch, days_since_epoch_to_date, is_leap_year};
