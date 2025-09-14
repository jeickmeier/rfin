//! MarketContext - Clean enum-based storage implementation
//!
//! This module provides the MarketContext implementation that uses enum-based storage
//! instead of trait objects, enabling complete serialization support and maximum performance.

pub mod core;
pub mod builder;
pub mod serde_support;
#[cfg(test)]
mod proof_of_concept;
#[cfg(test)]
mod demo;

pub use core::*;
pub use builder::*;
