//! Date utilities: business-day adjustments, day-counts, schedules, IMM helpers.
//!
//! This module wraps the [`time`](https://docs.rs/time) crate and exposes
//! domain-specific helpers commonly needed by pricing engines. Everything is
//! available through [`finstack_core::dates`], keeping downstream dependencies
//! small and version-stable.
//!
//! # Highlights
//! - ergonomic re-exports of `time` primitives (`Date`, `OffsetDateTime`, …)
//! - holiday calendars and business-day conventions (`adjust`, `HolidayCalendar`)
//! - schedule generation utilities (`ScheduleBuilder`)
//! - IMM/third-Wednesday helpers for derivatives roll dates
//!
//! # Examples
//! ```rust
//! use finstack_core::dates::{
//!     adjust, build_periods, BusinessDayConvention, Date, Frequency, ScheduleBuilder,
//! };
//! use finstack_core::dates::calendar::Target2;
//! use time::{Duration, Month};
//!
//! let trade_date = Date::from_calendar_date(2024, Month::March, 29).unwrap();
//! let adjusted = adjust(trade_date, BusinessDayConvention::Following, &Target2).unwrap();
//! assert!(adjusted >= trade_date);
//!
//! let end = trade_date + Duration::days(365);
//! let schedule = ScheduleBuilder::new(trade_date, end)
//!     .frequency(Frequency::quarterly())
//!     .build()
//!     .unwrap();
//! assert!(schedule.dates.len() >= 4);
//!
//! let periods = build_periods("2024Q1..Q4", None).unwrap();
//! assert_eq!(periods.periods.len(), 4);
//! ```

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
