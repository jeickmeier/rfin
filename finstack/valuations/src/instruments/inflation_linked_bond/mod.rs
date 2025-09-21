//! Inflation-Linked Bond (ILB) instrument implementation.
//!
//! Provides comprehensive support for inflation-indexed bonds including
//! TIPS, UK Index-Linked Gilts, and other inflation-protected securities.

pub mod parameters;
mod types;
pub mod pricing;
pub mod metrics;

pub use types::{DeflationProtection, IndexationMethod, InflationLinkedBond};
