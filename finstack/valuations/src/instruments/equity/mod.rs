//! Equity instruments for valuation and analysis.
//!
//! This module provides implementations for equity spot instruments
//! and related equity derivatives.

mod instrument;

// Re-export equity types
pub use instrument::Equity;
