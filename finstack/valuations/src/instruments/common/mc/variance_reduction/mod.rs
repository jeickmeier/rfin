//! Variance reduction techniques.
//!
//! Implements antithetic variates, control variates, moment matching,
//! and importance sampling.

pub mod antithetic;
pub mod control_variate;

#[cfg(feature = "mc")]
pub mod moment_matching;

#[cfg(feature = "mc")]
pub mod importance_sampling;

pub use antithetic::*;
pub use control_variate::*;
