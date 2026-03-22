//! Concrete extension implementations.
//!
//! These extensions implement the [`finstack_statements::extensions::Extension`] trait
//! to provide analysis and validation capabilities.

mod corkscrew;
mod scorecards;

pub use corkscrew::{AccountType, CorkscrewAccount, CorkscrewConfig, CorkscrewExtension};
pub use scorecards::{CreditScorecardExtension, ScorecardConfig, ScorecardMetric};
