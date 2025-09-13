//! Equity instruments for valuation and analysis.
//!
//! This module provides implementations for equity spot instruments,
//! private equity investments, and basket/ETF instruments.

pub mod basket;
mod instrument;
pub mod metrics;
pub mod private_equity;

// Re-export equity types
pub use basket::Basket;
pub use instrument::Equity;
pub use private_equity::PrivateEquityInvestment;
