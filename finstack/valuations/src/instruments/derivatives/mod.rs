//! Derivative instruments including total return swaps and other structured products.
//!
//! This module provides implementations of derivative instruments that reference
//! underlying assets or indices.

pub mod trs;

// Re-export main types
pub use trs::{EquityTotalReturnSwap, FIIndexTotalReturnSwap};
