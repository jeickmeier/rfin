//! Basis swap instrument module.
//!
//! Provides the implementation of basis swap instruments, which exchange two floating
//! rate payments with different tenors plus an optional spread. This module includes
//! the instrument type, pricer for registry integration, and metrics calculators.

pub mod metrics;
pub mod pricer;
mod types;

pub use pricer::SimpleBasisSwapDiscountingPricer;
pub use types::{BasisSwap, BasisSwapLeg};
