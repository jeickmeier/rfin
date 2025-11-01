//! Shared cashflow primitives and discounting helpers.
//!
//! This module hosts foundational cashflow types (`CashFlow`, `Notional`) and
//! lightweight helpers for discounting dated cashflows. Higher-level pricing
//! crates build on these to construct instrument-specific schedules.

pub mod discounting;
pub mod performance;
pub mod primitives;
pub mod xirr;

pub use discounting::{npv_static, Discountable};
pub use performance::{irr_periodic, npv as npv_performance};
pub use primitives::{AmortizationSpec, CFKind, CashFlow, Notional};
pub use xirr::xirr;
