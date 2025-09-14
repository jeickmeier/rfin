//! Unified curve storage system for MarketContext
//!
//! This module provides an enum-based storage system that replaces trait object
//! storage, enabling complete serialization support for all curve types.

pub mod curve_storage;
pub mod curve_state;

pub use curve_storage::*;

#[cfg(feature = "serde")]
pub use curve_state::*;
