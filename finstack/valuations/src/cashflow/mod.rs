//! Cashflow schedule generation, aggregation, and currency-safe operations.
//!
//! Provides comprehensive cashflow modeling infrastructure for bonds, swaps,
//! loans, and structured products. All cashflows use currency-tagged [`Money`]
//! types to prevent accidental cross-currency arithmetic.
//!
//! # Core Components
//!
//! - **Primitives**: Fundamental [`CashFlow`] type with date, amount, and classification
//! - **Builder**: Composable schedule builder for coupons, principal, and amortization
//! - **Aggregation**: Currency-preserving cashflow merging and rollup
//! - **Traits**: [`CashflowProvider`] trait for instruments to expose schedules
//!
//! # Cashflow Classification
//!
//! Each cashflow is tagged with [`CFKind`]:
//! - `Principal`: Principal repayments
//! - `Interest`: Coupon or interest payments
//! - `Fee`: Management, servicing, or structuring fees
//! - `Fixed`: Generic fixed payment
//! - `Floating`: Floating-rate payment projected from index
//!
//! # Currency Safety
//!
//! All aggregation operations enforce currency matching:
//! - Cannot sum cashflows in different currencies without explicit FX
//! - Currency preserved through aggregation pipeline
//! - FX conversion requires explicit policy
//!
//! # Examples
//!
//! ## Building a Cashflow Schedule
//!
//! ```rust
//! use finstack_valuations::cashflow::builder::CashFlowSchedule;
//! use finstack_core::currency::Currency;
//! use finstack_core::money::Money;
//! use finstack_core::dates::{create_date, DayCount};
//! use time::Month;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a cashflow schedule using the builder pattern
//! let mut builder = CashFlowSchedule::builder();
//!
//! // Add principal, coupons, etc. using builder methods
//! // (See CashFlowSchedule::builder() for full API)
//! # Ok(())
//! # }
//! ```
//!
//! ## Aggregating Cashflows
//!
//! ```rust
//! use finstack_valuations::cashflow::aggregation::aggregate_cashflows_precise_checked;
//! use finstack_core::money::Money;
//! use finstack_core::currency::Currency;
//! use finstack_core::dates::create_date;
//! use time::Month;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let date1 = create_date(2025, Month::January, 15)?;
//! let date2 = create_date(2025, Month::July, 15)?;
//!
//! let flows = vec![
//!     (date1, Money::new(50_000.0, Currency::USD)),
//!     (date2, Money::new(50_000.0, Currency::USD)),
//! ];
//!
//! // Aggregate using explicit target currency
//! let aggregated = aggregate_cashflows_precise_checked(&flows, Currency::USD)?.unwrap();
//! assert_eq!(aggregated.amount(), Money::new(100_000.0, Currency::USD).amount());
//! # Ok(())
//! # }
//! ```
//!
//! ## Pre-Period Present Value Aggregation
//!
//! Compute present values aggregated by reporting period for debugging and reconciliation:
//!
//! ```rust
//! use finstack_valuations::cashflow::builder::CashFlowSchedule;
//! use finstack_valuations::cashflow::aggregation::pv_by_period;
//! use finstack_core::dates::{Period, PeriodId, DayCount};
//! use finstack_core::market_data::traits::Discounting;
//! use time::Month;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Build schedule and periods
//! let schedule = CashFlowSchedule::builder()/* ... */.build()?;
//! let periods = vec![/* ... */];
//!
//! // Compute PV by period
//! let disc_curve: &dyn Discounting = /* ... */;
//! let base = disc_curve.base_date();
//! let pv_map = schedule.pre_period_pv(&periods, disc_curve, base, DayCount::Act365F);
//!
//! // Sum of period PVs equals total NPV
//! let total_pv: f64 = pv_map.values()
//!     .flat_map(|ccy_map| ccy_map.values().map(|m| m.amount()))
//!     .sum();
//! # Ok(())
//! # }
//! ```
//!
//! For credit-adjusted discounting, use `pre_period_pv_with_market` with a hazard curve ID.
//!
//! # See Also
//!
//! - [`primitives`] for core [`CashFlow`] type from finstack-core
//! - [`builder`] for schedule construction
//! - [`aggregation`] for currency-safe merging and [`pv_by_period`](aggregation::pv_by_period)
//! - [`traits`] for [`CashflowProvider`] and [`DatedFlows`]

/// Cash-flow primitives (`CashFlow`, `CFKind`, etc.).
pub mod primitives {
    pub use finstack_core::cashflow::primitives::*;
}

/// Currency-preserving aggregation utilities for cashflows.
pub mod aggregation;

/// Composable cashflow builder (phase 1: principal, amortization, fixed coupons).
pub mod builder;

/// Cashflow-related traits and aliases.
pub mod traits;

// -----------------------------------------------------------------------------
// Canonical flow aliases (deduplicated across the cashflow module)
// -----------------------------------------------------------------------------

pub use finstack_core::money::Money;
pub use finstack_core::prelude::Date;

/// Single dated amount in a specific currency.
pub type DatedFlow = (Date, Money);

/// Currency-preserving schedule as a list of dated amounts.
pub type DatedFlows = Vec<DatedFlow>;
