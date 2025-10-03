//! Capital Structure Integration
//!
//! This module provides integration between financial models and capital structure
//! (debt instruments like bonds, swaps, loans) via the `finstack-valuations` crate.
//!
//! ## Features
//! - Construct bonds, swaps, and other debt instruments from specifications
//! - Generate cashflow schedules from instruments
//! - Aggregate cashflows by period (interest, principal, fees)
//! - Provide DSL access to capital structure metrics via `cs.*` namespace
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

pub use integration::*;
pub use types::*;

