//! Composable cashflow builder for instruments.
//!
//! What: Ergonomic, deterministic builder to produce a `CashFlowSchedule` from
//! principal, amortization and coupon/fee programs. Fixed coupons are supported
//! today; floating coupons and fees are scaffolded for parity and determinism.
//!
//! Why: Centralize schedule logic and ordering invariants so downstream pricing
//! and risk consumers operate on a single, canonical schedule shape.
//!
//! How: Start with `cf()` or `CashflowBuilder::new()`, set principal and (optionally)
//! amortization, add fixed or programmatic coupon windows, then `build()`.
//!
//! # Quick Start
//!
//! Build a simple fixed-rate bond cashflow:
//!
//! ```rust
//! use finstack_core::currency::Currency;
//! use finstack_core::dates::{Date, Frequency, DayCount, BusinessDayConvention, StubKind};
//! use finstack_core::money::Money;
//! use finstack_valuations::cashflow::builder::{cf, FixedCouponSpec, CouponType};
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
//! let schedule = cf()
//!     .principal(Money::new(1_000_000.0, Currency::USD), issue, maturity)
//!     .fixed_cf(fixed_spec)
//!     .build()
//!     .unwrap();
//!
//! assert!(schedule.flows.len() > 0);
//! ```

pub mod schedule;
pub mod types;
mod compile;
pub mod schedule_utils;
mod state;

pub use state::{CashflowBuilder};
#[inline]
pub fn cf() -> CashflowBuilder { CashflowBuilder::default() }
pub use schedule::{CashFlowSchedule, CashflowMeta};
pub use types::{
    CouponType, FixedCouponSpec, FloatingCouponSpec, FloatCouponParams,
    ScheduleParams, FixedWindow, FloatWindow, FeeSpec, FeeBase,
};
pub use schedule_utils::{build_dates, PeriodSchedule};


