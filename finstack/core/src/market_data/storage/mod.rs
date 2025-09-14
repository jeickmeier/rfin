//! Unified curve storage system for MarketContext
//!
//! This module provides an enum-based storage system that replaces trait object
//! storage, enabling complete serialization support for all curve types.

#[cfg(feature = "new-context")]
pub mod curve_storage;
#[cfg(feature = "new-context")]
pub mod curve_state;

#[cfg(feature = "new-context")]
pub use curve_storage::*;

#[cfg(all(feature = "new-context", feature = "serde"))]
pub use curve_state::*;
