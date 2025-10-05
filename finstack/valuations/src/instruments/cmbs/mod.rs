//! Commercial Mortgage-Backed Security (CMBS) instrument module.
//!
//! Wraps the shared structured credit engine to model CMBS transactions with
//! commercial mortgage pools and tranche waterfalls.

pub mod metrics;
pub mod pricer;
mod types;

pub use types::Cmbs;
