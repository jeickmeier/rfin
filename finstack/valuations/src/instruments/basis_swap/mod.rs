//! Basis swap instrument module.
//!
//! Provides the implementation of basis swap instruments, which exchange two floating
//! rate payments with different tenors plus an optional spread. This module includes
//! pricing engines, metrics calculators, and type definitions.

pub mod metrics;
pub mod pricing;
mod types;

pub use pricing::engine::BasisEngine;
pub use types::{BasisSwap, BasisSwapLeg};
