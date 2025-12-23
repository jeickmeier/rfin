//! Dollar roll risk metrics.
//!
//! Dollar roll specific metrics include implied financing rate,
//! roll specialness, and break-even analysis.

// Metrics are implemented in the carry module
// Re-export key functions here

pub use super::carry::{break_even_drop, implied_financing_rate, roll_specialness, CarryResult};
