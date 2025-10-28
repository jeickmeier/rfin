//! Payoff specifications for Monte Carlo pricing.
//!
//! All payoffs return `Money` types for currency safety.

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
pub mod rates;

pub use traits::*;
pub use vanilla::*;

