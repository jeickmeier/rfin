//! Time discretization schemes for SDEs.
//!
//! Implements exact, Euler, Milstein, and specialized schemes.

pub mod exact;

#[cfg(feature = "mc")]
pub mod euler;

#[cfg(feature = "mc")]
pub mod milstein;

#[cfg(feature = "mc")]
pub mod qe_heston;

pub use exact::*;

