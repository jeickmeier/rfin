//! Equity instruments for valuation and analysis.
//!
//! This module provides implementations for equity spot instruments,
//! and private equity investments.

mod instrument;
pub mod metrics;
pub mod private_equity;

// Re-export equity types
pub use instrument::Equity;
pub use private_equity::PrivateEquityInvestment;
