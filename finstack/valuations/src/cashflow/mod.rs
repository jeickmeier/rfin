//! Cashflow primitive types, builder, aggregation, and more.
//!
//! Rustfin **CashFlow** module
//!
//! # Example
//! ```rust
//! ```

/// Cash-flow primitives (`CashFlow`, `CFKind`, etc.).
pub mod primitives;

/// Currency-preserving aggregation utilities for cashflows.
pub mod aggregation;

/// Composable cashflow builder (phase 1: principal, amortization, fixed coupons).
pub mod builder;
