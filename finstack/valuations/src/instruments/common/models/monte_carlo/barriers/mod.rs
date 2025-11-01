//! Barrier handling for Monte Carlo.
//!
//! Implements Brownian bridge corrections and barrier adjustments.

#[cfg(feature = "mc")]
pub mod bridge;

#[cfg(feature = "mc")]
pub mod corrections;
