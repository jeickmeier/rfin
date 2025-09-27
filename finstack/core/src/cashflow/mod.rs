//! Shared cashflow primitives and discounting helpers.
//!
//! This module hosts foundational cashflow types (`CashFlow`, `Notional`) and
//! lightweight helpers for discounting dated cashflows. Higher-level pricing
//! crates build on these to construct instrument-specific schedules.

pub mod discounting;
pub mod primitives;

pub use discounting::{npv_static, Discountable};
pub use primitives::{AmortizationSpec, CFKind, CashFlow, Notional};
