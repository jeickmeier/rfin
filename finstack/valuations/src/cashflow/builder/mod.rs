//! Composable cashflow builder for instruments.
//!
//! Provides a simplified interface for common cashflow patterns and a full-featured
//! builder for complex scenarios with programmatic control.
//!
//! # Simple Interface (Recommended)
//!
//! For most instruments, use the simplified builder:
//!
//! ```rust
//! use finstack_core::currency::Currency;
//! use finstack_core::dates::{Date, Frequency, DayCount, BusinessDayConvention, StubKind};
//! use finstack_core::money::Money;
//! use finstack_valuations::cashflow::builder::{cf, ScheduleParams, FixedCouponSpec, CouponType};
//! use time::Month;
//!
//! let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
//! let maturity = Date::from_calendar_date(2026, Month::January, 15).unwrap();
//!
//! let schedule_params = ScheduleParams {
//!     freq: Frequency::semi_annual(),
//!     dc: DayCount::Act365F,
//!     bdc: BusinessDayConvention::Following,
//!     calendar_id: None,
//!     stub: StubKind::None,
//! };
//!
//! let fixed_spec = FixedCouponSpec {
//!     coupon_type: CouponType::Cash,
//!     rate: 0.05,
//!     freq: schedule_params.freq,
//!     dc: schedule_params.dc,
//!     bdc: schedule_params.bdc,
//!     calendar_id: schedule_params.calendar_id,
//!     stub: schedule_params.stub,
//! };
//!
//! let schedule = cf()
//!     .principal(Money::new(1_000_000.0, Currency::USD), issue, maturity)
//!     .fixed_cf(fixed_spec)
//!     .build()
//!     .unwrap();
//!
//! assert!(schedule.flows.len() > 0);
//! ```
//!
//! # Full-featured Interface
//!
//! For complex scenarios (windows, programs, PIK toggles), use `CashflowBuilder`
//! directly with the types re-exported in this module (see `schedule` and
//! `schedule_utils`).

mod compile;
pub mod schedule;
pub mod schedule_utils;
mod state;
#[cfg(test)]
mod tests;
pub mod types;

// Export the full-featured builder as CashflowBuilder
pub use state::CashflowBuilder;

/// Convenience function to create a new cashflow builder.
pub fn cf() -> CashflowBuilder {
    CashflowBuilder::default()
}

// Re-export common types
pub use schedule::{CashFlowSchedule, CashflowMeta};
pub use schedule_utils::{build_dates, PeriodSchedule};
pub use types::{
    CouponType, FeeBase, FeeSpec, FixedCouponSpec, FixedWindow, FloatCouponParams, FloatWindow,
    FloatingCouponSpec, ScheduleParams,
};
