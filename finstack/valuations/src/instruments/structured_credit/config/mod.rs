//! Configuration and constants for structured credit instruments.
//!
//! This module is split into two submodules:
//! - [`constants`]: Industry-standard constants, fee defaults, and model parameters
//! - [`structures`]: Configuration structs (DealConfig, DealDates, DealFees, etc.)

pub mod constants;
pub mod structures;

// Re-export all constants
pub use constants::*;

// Re-export all structures
pub use structures::*;
