//! Advanced cashflow builder with full programmatic control.
//!
//! This module provides the full-featured cashflow builder that supports
//! complex scenarios like segmented coupon programs, payment windows, and
//! PIK toggles. For simple use cases, prefer the main CashflowBuilder.

pub use super::state::CashflowBuilder as CashflowBuilderAdvanced;
pub use super::types::*;

// Re-export all the advanced types for power users
pub use super::schedule::*;
pub use super::schedule_utils::*;
