//! Concrete extension implementations.
//!
//! Each extension lives in its own submodule and implements the
//! [`finstack_statements::extensions::Extension`] trait. To add a new extension,
//! create a directory under `extensions/` with a `mod.rs` and register it here.
//!
//! - [`corkscrew`] — roll-forward validation for balance-sheet accounts
//! - [`scorecards`] — credit scorecard rating assignment

pub mod corkscrew;
pub mod scorecards;

pub use corkscrew::{AccountType, CorkscrewAccount, CorkscrewConfig, CorkscrewExtension};
pub use scorecards::{CreditScorecardExtension, ScorecardConfig, ScorecardMetric};
