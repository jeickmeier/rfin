//! Interest Rate Future instrument implementation.
//!
//! Represents exchange-traded interest rate futures like SOFR, Eurodollar,
//! or Short Sterling futures. Essential for calibrating the short end of
//! forward curves with proper convexity adjustments.

pub mod metrics;
pub mod pricer;
mod types;

pub use types::{FutureContractSpecs, InterestRateFuture, Position};

// Builder provided by FinancialBuilder derive
