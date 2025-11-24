//! Capital Structure Integration
//!
//! This module provides integration between financial models and capital structure
//! (debt instruments like bonds, swaps, loans) via the `finstack-valuations` crate.
//!
//! ## Features
//! - ✅ Construct bonds, swaps, and other debt instruments from specifications
//! - ✅ Generate cashflow schedules from instruments
//! - ✅ Aggregate cashflows by period (interest, principal, fees)
//! - ✅ DSL access to capital structure metrics via `cs.*` namespace
//!
//! ## DSL Integration
//! The `cs.*` namespace provides access to capital structure data in formulas:
//! - `cs.interest_expense.{instrument_id}` - Interest expense for specific instrument
//! - `cs.interest_expense.total` - Total interest expense across all instruments
//! - `cs.principal_payment.{instrument_id}` - Principal payment for specific instrument
//! - `cs.principal_payment.total` - Total principal payments across all instruments
//! - `cs.debt_balance.{instrument_id}` - Outstanding debt balance for specific instrument
//! - `cs.debt_balance.total` - Total outstanding debt balance
//!
//! ## ✅ Integration Status: Complete
//! - Capital structure cashflow computation fully integrated with valuations
//! - Precise CFKind-based classification (no heuristics)
//! - Outstanding balance tracking via `outstanding_by_date()`
//! - 100% leverage of valuations infrastructure achieved
//!
//! ## Example
//! ```ignore
//! use finstack_statements::prelude::*;
//!
//! let model = ModelBuilder::new("LBO Model")
//!     .periods("2025Q1..2025Q4", Some("2025Q1"))?
//!     .value("revenue", &[...])?
//!     
//!     // Add debt instruments
//!     .add_bond(
//!         "BOND-001",
//!         Money::new(10_000_000.0, Currency::USD),
//!         0.05,  // 5% coupon
//!         issue_date,
//!         maturity_date,
//!         "USD-OIS",
//!     )?
//!     
//!     // Reference in formulas
//!     .compute("interest_expense", "cs.interest_expense.BOND-001")?
//!     .compute("total_debt_service", "cs.interest_expense.total + cs.principal_payment.total")?
//!     .build()?;
//! ```

mod builder;
pub mod integration;
mod types;
pub mod waterfall;

pub use integration::*;
pub use types::*;
pub use waterfall::*;
