//! Variance swap instrument implementation.
//!
//! Variance swaps are forward contracts on realized variance, allowing
//! direct exposure to volatility without delta hedging.

pub mod metrics;
pub mod pricer;
pub mod types;

pub use pricer::SimpleVarianceSwapDiscountingPricer;
pub use types::{PayReceive, VarianceSwap};

// Re-export from core
pub use finstack_core::math::stats::{
    realized_variance, realized_variance_ohlc, RealizedVarMethod,
};
