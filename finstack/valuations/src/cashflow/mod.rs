#![allow(missing_docs)]

//! Rustfin **CashFlow** module — *Phase 1 Bootstrap* (in-core variant).

/// Cash-flow primitives (`CashFlow`, `CFKind`, etc.).
pub mod primitives;

/// Cash-flow leg builder and related utilities.
pub mod leg;

/// Amortization specifications shared across instruments and legs.
pub mod amortization;

/// Notional amount types and amortisation rules.
pub mod notional;

/// Currency-preserving aggregation utilities for cashflows.
pub mod aggregation;

/// Composable cashflow builder (phase 1: principal, amortization, fixed coupons).
pub mod builder;
