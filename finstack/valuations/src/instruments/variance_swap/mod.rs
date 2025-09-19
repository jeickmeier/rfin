//! Variance swap instrument implementation.
//!
//! Variance swaps are forward contracts on realized variance, allowing
//! direct exposure to volatility without delta hedging.

pub mod metrics;
pub mod types;

pub use types::{VarianceSwap, PayReceive};

// Re-export from core
pub use finstack_core::math::stats::{RealizedVarMethod, realized_variance, realized_variance_ohlc};
