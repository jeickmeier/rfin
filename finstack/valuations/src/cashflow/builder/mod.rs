//! Composable cashflow builder for instruments.
//!
//! Provides a builder interface for common cashflow patterns with programmatic control.
//! Use `CashFlowSchedule::builder()` as the standard entry point.
//!
//! # Usage
//!
//! ```rust
//! use finstack_core::currency::Currency;
//! use finstack_core::dates::{Date, Frequency, DayCount, BusinessDayConvention, StubKind};
//! use finstack_core::money::Money;
//! use finstack_valuations::cashflow::builder::{CashFlowSchedule, ScheduleParams, FixedCouponSpec, CouponType};
//! use time::Month;
//!
//! let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
//! let maturity = Date::from_calendar_date(2026, Month::January, 15).unwrap();
//!
//! let fixed_spec = FixedCouponSpec {
//!     coupon_type: CouponType::Cash,
//!     rate: 0.05,
//!     freq: Frequency::semi_annual(),
//!     dc: DayCount::Act365F,
//!     bdc: BusinessDayConvention::Following,
//!     calendar_id: None,
//!     stub: StubKind::None,
//! };
//!
//! let schedule = CashFlowSchedule::builder()
//!     .principal(Money::new(1_000_000.0, Currency::USD), issue, maturity)
//!     .fixed_cf(fixed_spec)
//!     .build()
//!     .unwrap();
//!
//! assert!(schedule.flows.len() > 0);
//! ```

mod compile;
pub mod frame;
pub mod schedule;
pub mod schedule_utils;
mod state;
#[cfg(test)]
mod tests;
pub mod types;

// Export the builder as CashflowBuilder
pub use state::CashflowBuilder;

// Re-export common types
pub use frame::FlowFrame;
pub use schedule::{CashFlowSchedule, CashflowMeta};
pub use schedule_utils::{build_dates, PeriodSchedule};
pub use types::{
    CouponType, FeeBase, FeeSpec, FixedCouponSpec, FixedWindow, FloatCouponParams, FloatWindow,
    FloatingCouponSpec, ScheduleParams,
};
