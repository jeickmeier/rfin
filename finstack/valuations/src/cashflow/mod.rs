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
//! # See Also
//!
//! - [`primitives`] for core [`CashFlow`] type from finstack-core
//! - [`builder`] for schedule construction
//! - [`aggregation`] for currency-safe merging
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
