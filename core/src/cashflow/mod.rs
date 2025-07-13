#![allow(missing_docs)]

//! Rustfin **CashFlow** module — *Phase 1 Bootstrap* (in-core variant).

/// Cash-flow primitives (`CashFlow`, `CFKind`, etc.).
pub mod primitives;

/// Cash-flow leg builder and related utilities.
pub mod leg;

/// Net-present-value helpers and discountable traits.
pub mod npv;

/// Day-count accrual factor caching.
pub mod accrual;

/// Notional amount types and amortisation rules.
pub mod notional;

/// Stub period detection and handling utilities.
pub mod stub {}
