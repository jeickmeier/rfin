//! Residential Mortgage-Backed Security (RMBS) instrument module.
//!
//! Uses the shared structured credit components to represent RMBS structures with
//! mortgage-specific pool behavior and waterfall logic.

pub mod metrics;
pub mod pricer;
mod types;

pub use types::Rmbs;
