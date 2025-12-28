//! Cashflow emission helpers.
//!
//! This module contains functions that emit cashflows on specific dates based on
//! coupon schedules, amortization specs, and fee specifications. These functions
//! are called by the build pipeline to generate deterministic cashflow sequences.
//!
//! ## Responsibilities
//!
//! - Emit fixed coupon cashflows with PIK capitalization
//! - Emit floating coupon cashflows with forward rate lookups
//! - Emit amortization payments according to various schedules
//! - Emit periodic and fixed fee cashflows
//! - Emit credit event cashflows (defaults, prepayments, recoveries)
//! - Track outstanding balances through PIK and amortization
//!
//! ## Organization
//!
//! This module is organized into submodules by emission type:
//! - `helpers`: Common helper functions for PIK flows and reset date calculation
//! - `coupons`: Fixed and floating coupon emission
//! - `amortization`: Principal amortization emission
//! - `fees`: Periodic, commitment, usage, and facility fee emission
//! - `credit`: Default, prepayment, and recovery emission
//! - `tests`: Comprehensive test suite for emission functions
//!
//! ## Design
//!
//! Each `emit_*_on` function takes a date and relevant schedules, computes the
//! appropriate cashflows for that date, and returns both the flows and any PIK
//! amount that should capitalize into the outstanding balance.

mod amortization;
mod coupons;
mod credit;
mod fees;
mod helpers;
#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests;

// Re-export coupon emission (internal to builder module)
pub(super) use coupons::{emit_fixed_coupons_on, emit_float_coupons_on};

// Re-export amortization emission and types (internal to builder module)
pub(super) use amortization::{emit_amortization_on, AmortizationParams};

// Re-export fee emission (internal to builder module)
pub(super) use fees::emit_fees_on;

// Re-export public fee emission functions
pub use fees::{emit_commitment_fee_on, emit_facility_fee_on, emit_usage_fee_on};

// Re-export public credit event emission
pub use credit::{emit_default_on, emit_prepayment_on};
