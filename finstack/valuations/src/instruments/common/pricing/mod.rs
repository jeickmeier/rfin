//! Common pricing patterns and shared infrastructure.
//!
//! This module provides generic pricer implementations and shared pricing utilities
//! to eliminate duplication across instrument pricing modules.
//!
//! ## Sub-modules
//!
//! - [`core`]: Generic pricers and TRS pricing engine
//! - [`swap_legs`]: Shared floating/fixed leg pricing for swaps

mod core;
pub mod swap_legs;

// Re-export everything from core for backward compatibility
pub use self::core::*;

