//! Inflation-Linked Bond (ILB) instrument implementation.
//!
//! Provides comprehensive support for inflation-indexed bonds including
//! TIPS, UK Index-Linked Gilts, and other inflation-protected securities.

mod builder;
pub mod metrics;
mod types;

pub use types::{DeflationProtection, IndexationMethod, InflationLinkedBond};
