//! Payoff definitions for Monte Carlo pricing.
//!
//! Start with [`vanilla`] for European call / put, digital, and forward-style
//! payoffs. Under the `mc` feature this module adds path-dependent payoffs such
//! as Asian, barrier, basket, and lookback contracts.
//!
//! All payoffs return [`finstack_core::money::Money`] for currency safety and
//! are evaluated on a mutable [`crate::traits::PathState`], which lets them
//! inspect named state variables and record path-level cashflows.

pub mod traits;
pub mod vanilla;

#[cfg(feature = "mc")]
pub mod asian;

#[cfg(feature = "mc")]
pub mod barrier;

#[cfg(feature = "mc")]
pub mod lookback;

#[cfg(feature = "mc")]
pub mod basket;

#[cfg(feature = "mc")]
pub use basket::*;

pub use traits::*;
pub use vanilla::*;
