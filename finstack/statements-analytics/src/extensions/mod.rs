//! Concrete extension implementations.
//!
//! These modules still implement the deprecated
//! [`finstack_statements::extensions::Extension`] trait for backwards
//! compatibility. Prefer the inherent methods on the concrete extension
//! structs instead.
//!
//! - [`corkscrew`] — roll-forward validation for balance-sheet accounts
//! - [`scorecards`] — credit scorecard rating assignment

#![allow(deprecated)]

pub mod corkscrew;
pub mod scorecards;

pub use corkscrew::{AccountType, CorkscrewAccount, CorkscrewConfig, CorkscrewExtension};
pub use scorecards::{CreditScorecardExtension, ScorecardConfig, ScorecardMetric};
