//! Equity instruments for valuation and analysis.
//!
//! This module provides implementations for equity spot instruments,
//! private equity investments, and basket/ETF instruments.

pub mod basket;
pub mod spot;
pub mod metrics;
pub mod private_equity;
pub mod underlying;

// Re-export equity types
pub use basket::Basket;
pub use spot::Equity;
pub use private_equity::PrivateEquityInvestment;
pub use underlying::EquityUnderlyingParams;
