//! Cashflow schedule generation, aggregation, and currency-safe operations.
//!
//! Provides comprehensive cashflow modeling infrastructure for bonds, swaps,
//! loans, and structured products. All cashflows use currency-tagged `Money`
//! types to prevent accidental cross-currency arithmetic.
//!
//! # Core Components
//!
//! - **Primitives**: Fundamental `CashFlow` type with date, amount, and classification
//! - **Builder**: Composable schedule builder for coupons, principal, and amortization
//! - **Aggregation**: Currency-preserving cashflow merging and rollup
//! - **Traits**: `CashflowProvider` trait for instruments to expose schedules
//!
//! # Currency Safety
//!
//! All aggregation operations enforce currency matching:
//! - Cannot sum cashflows in different currencies without explicit FX
//! - Currency preserved through aggregation pipeline
//! - FX conversion requires explicit policy
//!
//! # Conventions
//!
//! - Coupon rates are generally decimals (for example `0.05` for 5%).
//! - Spreads and fee quotes are often expressed in basis points on the
//!   specification types.
//! - Schedule timing depends on explicit day-count, business-day, calendar, and
//!   lag settings carried by the builder specs.
//! - `CFKind` behavior should be read from the authoritative
//!   `finstack_core::cashflow::CFKind` definition rather than copied here. The
//!   enum is `#[non_exhaustive]` and downstream logic should not assume a fixed
//!   exhaustive list of variants.
//!
//! # Examples
//!
//! ## Building a Cashflow Schedule
//!
//! ```rust,ignore
//! use finstack_cashflows::builder::CashFlowSchedule;
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
//! use finstack_cashflows::aggregation::aggregate_cashflows_precise_checked;
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
//! let aggregated = aggregate_cashflows_precise_checked(&flows, Currency::USD)?;
//! assert_eq!(aggregated.amount(), Money::new(100_000.0, Currency::USD).amount());
//! # Ok(())
//! # }
//! ```
//!
//! ## Periodized Present Value Aggregation
//!
//! Compute present values aggregated by reporting period for analysis and reconciliation.
//!
//! ### From a Schedule (Recommended In This Crate)
//!
//! The stable public interface in this crate is the schedule-level API on
//! [`builder::CashFlowSchedule`]:
//!
//! ```rust,no_run
//! use finstack_cashflows::builder::CashFlowSchedule;
//! use finstack_core::dates::{Date, DayCount, DayCountCtx, Period};
//! use finstack_core::market_data::traits::Discounting;
//!
//! fn periodized_pv(
//!     schedule: &CashFlowSchedule,
//!     periods: &[Period],
//!     disc: &dyn Discounting,
//!     base: Date,
//! ) -> finstack_core::Result<()> {
//!     let pv_map = schedule.pv_by_period_with_ctx(
//!         periods,
//!         disc,
//!         base,
//!         DayCount::Act365F,
//!         DayCountCtx::default(),
//!     )?;
//!
//!     let _ = pv_map;
//!     Ok(())
//! }
//! ```
//!
//! ### From Higher-Level Instruments
//!
//! Higher-level instrument adapters may exist in other crates, such as
//! `finstack-valuations`, but they are outside this crate's direct public API.
//!
//! # See Also
//!
//! - `primitives` for core `CashFlow` type from finstack-core
//! - `builder` for schedule construction
//! - `aggregation` for currency-safe dated-flow aggregation
//! - `CashflowProvider` and `schedule_from_dated_flows` for schedule interfaces

/// Cash-flow primitives (`CashFlow`, `CFKind`, etc.).
pub mod primitives {
    pub use finstack_core::cashflow::*;
}

/// Currency-preserving aggregation utilities for cashflows.
pub mod aggregation;

/// Composable cashflow builder (phase 1: principal, amortization, fixed coupons).
pub mod builder;

/// Cashflow-related traits and aliases.
pub mod traits;

/// Generic schedule-driven interest accrual engine.
pub mod accrual;

// -----------------------------------------------------------------------------
// Canonical flow aliases (deduplicated across the cashflow module)
// -----------------------------------------------------------------------------

// Re-export primary builder type at the cashflow root for ergonomic access.
pub use accrual::{accrued_interest_amount, AccrualConfig, AccrualMethod, ExCouponRule};
pub use builder::CashFlowBuilder;
pub use traits::{
    empty_schedule, empty_schedule_with_representation, schedule_from_classified_flows,
    schedule_from_classified_flows_with_representation, schedule_from_dated_flows,
    schedule_from_dated_flows_with_representation, CashflowProvider,
};

pub use finstack_core::dates::Date;
pub use finstack_core::money::Money;

/// Single dated amount in a specific currency.
pub type DatedFlow = (Date, Money);

/// Currency-preserving schedule as a list of dated amounts.
pub type DatedFlows = Vec<DatedFlow>;
