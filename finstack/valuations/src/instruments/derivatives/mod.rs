//! Derivative instruments including total return swaps and other structured products.
//!
//! This module provides implementations of derivative instruments that reference
//! underlying assets or indices.

pub mod trs;
pub mod variance_swap;

// Re-export main types
pub use trs::{EquityTotalReturnSwap, FIIndexTotalReturnSwap};
pub use variance_swap::{VarianceSwap, VarianceSwapBuilder, PayReceive, RealizedVarMethod};
