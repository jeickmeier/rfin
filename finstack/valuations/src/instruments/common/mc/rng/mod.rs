//! Random number generation for Monte Carlo simulation.
//!
//! Provides counter-based RNGs (Philox) for deterministic parallel simulation
//! and quasi-Monte Carlo sequences (Sobol with Owen scrambling).

pub mod philox;
pub mod transforms;

#[cfg(feature = "mc")]
pub mod sobol;

#[cfg(feature = "mc")]
pub mod poisson;

pub use philox::PhiloxRng;
pub use transforms::*;

