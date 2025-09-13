//! Interest Rate Future instrument implementation.
//!
//! Represents exchange-traded interest rate futures like SOFR, Eurodollar,
//! or Short Sterling futures. Essential for calibrating the short end of
//! forward curves with proper convexity adjustments.

mod builder;
pub mod metrics;
mod types;

pub use types::{FutureContractSpecs, InterestRateFuture};

// Provide a distinct path for types.rs to reference this builder
#[allow(unused_imports)]
pub(crate) mod mod_ir_future {
    pub use super::builder::IRFutureBuilder;
}
