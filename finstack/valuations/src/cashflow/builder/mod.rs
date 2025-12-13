//! Composable cashflow builder for instruments.
//!
//! Provides a builder interface for common cashflow patterns with programmatic control.
//! Use `CashFlowSchedule::builder()` as the standard entry point.
//!
//! # Usage
//!
//! ```rust
//! use finstack_core::currency::Currency;
//! use finstack_core::dates::{Date, Tenor, DayCount, BusinessDayConvention, StubKind};
//! use finstack_core::money::Money;
//! use finstack_valuations::cashflow::builder::{CashFlowSchedule, ScheduleParams, FixedCouponSpec, CouponType};
//! use time::Month;
//!
//! let issue = Date::from_calendar_date(2025, Month::January, 15).expect("valid date");
//! let maturity = Date::from_calendar_date(2026, Month::January, 15).expect("valid date");
//!
//! let fixed_spec = FixedCouponSpec {
//!     coupon_type: CouponType::Cash,
//!     rate: 0.05,
//!     freq: Tenor::semi_annual(),
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
//!     .expect("should succeed");
//!
//! assert!(schedule.flows.len() > 0);
//! ```

// Internal modules
#[allow(clippy::module_inception)]
mod builder;
mod compiler;
mod emission;

// Public modules
pub mod credit_rates;
pub mod dataframe;
pub mod date_generation;
pub mod rate_helpers;
pub mod schedule;
pub mod specs;

// Export the builder as CashFlowBuilder
pub use builder::{CashFlowBuilder, PrincipalEvent};

// Re-export common types
pub use dataframe::{PeriodDataFrame, PeriodDataFrameOptions};
pub use date_generation::{build_dates, PeriodSchedule};
pub use rate_helpers::{
    project_floating_rate, project_floating_rate_from_market, FloatingRateParams,
};
pub use schedule::{CashFlowMeta, CashFlowSchedule};
pub use specs::{
    evaluate_fee_tiers, AmortizationSpec, CouponType, DefaultCurve, DefaultEvent, DefaultModelSpec,
    FeeBase, FeeSpec, FeeTier, FixedCouponSpec, FixedWindow, FloatCouponParams, FloatWindow,
    FloatingCouponSpec, FloatingRateSpec, Notional, PrepaymentCurve, PrepaymentModelSpec,
    RecoveryModelSpec, ScheduleParams,
};

// Re-export credit rate conversions (hazard-style CPR↔SMM helpers)
pub use credit_rates::{cpr_to_smm, smm_to_cpr};

// Re-export emission functions
pub use emission::{
    emit_commitment_fee_on, emit_default_on, emit_facility_fee_on, emit_prepayment_on,
    emit_usage_fee_on,
};
