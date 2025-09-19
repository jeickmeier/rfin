//! Basis swap instrument module: declares submodules and re-exports types.

pub mod metrics;
mod types;

pub use types::{BasisSwap, BasisSwapLeg};
