//! Asset-Backed Security (ABS) instrument module.
//!
//! Built on the shared structured credit components for pools, tranches, coverage tests,
//! and waterfall logic, providing a reusable ABS instrument representation.

pub mod metrics;
pub mod pricer;
mod types;

pub use types::Abs;
