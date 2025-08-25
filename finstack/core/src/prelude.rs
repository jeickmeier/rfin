//! finstack-core prelude: commonly used items + Polars re-exports
//!
//! Import this module to get a convenient set of core types and Polars
//! helpers in scope without pulling `polars` directly at call sites.
//! Downstream crates should prefer `use finstack_core::prelude::*;`.

// Core re-exports (keep list focused on ergonomic entry points)
pub use crate::currency::Currency;
pub use crate::dates::{
    adjust, available_calendars, next_cds_date, next_imm, third_wednesday, BusinessDayConvention,
    Date, DateExt, DayCount, HolidayCalendar, MergeMode, OffsetDateTime, OffsetDateTimeExt, Period,
    PeriodId, PeriodKey, ScheduleBuilder, StubKind,
};
pub use crate::error::{Error, InputError};
pub use crate::expr::{CompiledExpr, Expr, ExpressionContext};
pub use crate::market_data::{
    id::CurveId,
    interp::{InterpFn, InterpStyle},
    traits::{Discount, Forward, Inflation as InflationTs, Surface, Survival, TermStructure},
};
pub use crate::money::{fx::*, Money};
pub use crate::Result;

// Re-export Polars with an alias to avoid `Expr` name collision and to keep a
// consistent surface across the workspace.
pub use polars::prelude::{DataFrame, Series};
pub use polars::lazy::prelude::{col, lit, Expr as PolarsExpr, LazyFrame};
