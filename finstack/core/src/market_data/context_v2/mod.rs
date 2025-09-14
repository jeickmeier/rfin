//! MarketContext V2 - Enum-based storage implementation
//!
//! This module provides a redesigned MarketContext that uses enum-based storage
//! instead of trait objects, enabling complete serialization support.

#[cfg(feature = "new-context")]
pub mod core;
#[cfg(feature = "new-context")]
pub mod builder;
#[cfg(feature = "new-context")]
pub mod serde_support;
#[cfg(all(feature = "new-context", test))]
mod proof_of_concept;
#[cfg(all(feature = "new-context", test))]
mod demo;

#[cfg(feature = "new-context")]
pub use core::*;
#[cfg(feature = "new-context")]
pub use builder::*;
