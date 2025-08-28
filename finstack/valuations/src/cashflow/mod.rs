//! Cashflow primitive types, builder, aggregation, and more.
//! 
//! Rustfin **CashFlow** module — *Phase 1 Bootstrap* (in-core variant).
//! 
//! # Example
//! ```rust
//! use finstack_core::currency::Currency;
//! use finstack_core::money::Money;
//! use finstack_core::dates::{Date, Frequency, DayCount, BusinessDayConvention};
//! use finstack_core::dates::StubKind;
//! use time::Month;
//! 
//! let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
//! let maturity = Date::from_calendar_date(2026, Month::January, 15).unwrap();
//! // Note: Cashflow builder would be used here
//! ```

/// Cash-flow primitives (`CashFlow`, `CFKind`, etc.).
pub mod primitives;

/// Amortization and notional definitions (merged).
pub mod amortization_notional;

/// Currency-preserving aggregation utilities for cashflows.
pub mod aggregation;

/// Composable cashflow builder (phase 1: principal, amortization, fixed coupons).
pub mod builder;
