//! Higher-level pricing entry points built on top of [`crate::engine::McEngine`].
//!
//! Start with [`european`] for a compact GBM-only API. Under the `mc` feature,
//! [`path_dependent`] and [`lsmc`] expose richer workflows for path-dependent
//! contracts and early-exercise problems.
//!
//! These pricers bundle process, discretization, and engine choices for common
//! use cases; the lower-level engine remains the more flexible option when you
//! need custom combinations.

pub mod european;

#[cfg(feature = "mc")]
pub mod path_dependent;

#[cfg(feature = "mc")]
pub mod lsmc;

#[cfg(feature = "mc")]
pub mod basis;

#[cfg(feature = "mc")]
pub mod lsq;

#[allow(unused_imports)]
pub use european::*;
