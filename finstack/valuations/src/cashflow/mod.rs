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
//! let aggregated = aggregate_cashflows_precise_checked(&flows, Currency::USD)?.expect("should succeed");
//! assert_eq!(aggregated.amount(), Money::new(100_000.0, Currency::USD).amount());
//! # Ok(())
//! # }
//! ```
//!
//! ## Periodized Present Value Aggregation
//!
//! Compute present values aggregated by reporting period for analysis and reconciliation.
//!
//! ### From an Instrument (Recommended)
//!
//! Any instrument implementing [`CashflowProvider`] + [`crate::instruments::common::pricing::HasDiscountCurve`]
//! automatically gains periodized PV methods via the [`crate::instruments::common::period_pv::PeriodizedPvExt`] trait:
//!
//! ```rust
//! use finstack_valuations::instruments::Bond;
//! use finstack_valuations::instruments::common::period_pv::PeriodizedPvExt;
//! use finstack_core::dates::{Date, Period, PeriodId, DayCount};
//! use finstack_core::currency::Currency;
//! use finstack_core::money::Money;
//! use finstack_core::market_data::context::MarketContext;
//! use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
//! use time::Month;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let issue = Date::from_calendar_date(2025, Month::January, 1)?;
//! let maturity = Date::from_calendar_date(2026, Month::January, 1)?;
//!
//! let bond = Bond::fixed(
//!     "BOND-001",
//!     Money::new(1_000_000.0, Currency::USD),
//!     0.05,
//!     issue,
//!     maturity,
//!     "USD-OIS",
//! );
//!
//! let disc_curve = DiscountCurve::builder("USD-OIS")
//!     .base_date(issue)
//!     .knots([(0.0, 1.0), (1.0, 0.95)])
//!     .set_interp(finstack_core::math::interp::InterpStyle::Linear)
//!     .build()?;
//! let market = MarketContext::new().insert_discount(disc_curve);
//!
//! let periods = vec![
//!     Period {
//!         id: PeriodId::quarter(2025, 1),
//!         start: Date::from_calendar_date(2025, Month::January, 1)?,
//!         end: Date::from_calendar_date(2025, Month::April, 1)?,
//!         is_actual: true,
//!     },
//! ];
//!
//! // Compute periodized PV using the extension trait
//! let pv_by_period = bond.periodized_pv(&periods, &market, issue, DayCount::Act365F)?;
//!
//! // Access PV for a specific period
//! if let Some(q1_pvs) = pv_by_period.get(&PeriodId::quarter(2025, 1)) {
//!     if let Some(usd_pv) = q1_pvs.get(&Currency::USD) {
//!         println!("Q1 PV: ${:.2}", usd_pv.amount());
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ### From a Schedule Directly
//!
//! For lower-level control, use [`builder::CashFlowSchedule::pre_period_pv`] or
//! [`builder::CashFlowSchedule::pre_period_pv_with_market`] directly.
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

/// Generic schedule-driven interest accrual engine.
pub mod accrual;

// -----------------------------------------------------------------------------
// Canonical flow aliases (deduplicated across the cashflow module)
// -----------------------------------------------------------------------------

// Re-export primary builder type at the cashflow root for ergonomic access.
pub use builder::CashFlowBuilder;

pub use finstack_core::money::Money;
pub use finstack_core::prelude::Date;

/// Single dated amount in a specific currency.
pub type DatedFlow = (Date, Money);

/// Currency-preserving schedule as a list of dated amounts.
pub type DatedFlows = Vec<DatedFlow>;
