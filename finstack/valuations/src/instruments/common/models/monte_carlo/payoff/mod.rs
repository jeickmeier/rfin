//! Payoff specifications for Monte Carlo pricing.
//!
//! All payoffs return `Money` types for currency safety.

pub(crate) mod traits;
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

#[cfg(feature = "mc")]
pub mod rates;

#[cfg(feature = "mc")]
pub mod swaption;

#[cfg(feature = "mc")]
pub mod quanto;

#[cfg(feature = "mc")]
pub mod autocallable;

#[cfg(feature = "mc")]
pub mod cms;

#[cfg(feature = "mc")]
pub mod cliquet;

#[cfg(feature = "mc")]
pub mod range_accrual;

#[cfg(feature = "mc")]
pub mod fx_barrier;

#[cfg(feature = "mc")]
pub use rates::*;

#[cfg(feature = "mc")]
pub use swaption::*;

#[cfg(feature = "mc")]
pub use quanto::*;

#[cfg(feature = "mc")]
pub use autocallable::*;

#[cfg(feature = "mc")]
pub use cms::*;

#[cfg(feature = "mc")]
pub use cliquet::*;

#[cfg(feature = "mc")]
pub use range_accrual::*;

#[cfg(feature = "mc")]
pub use fx_barrier::*;

pub use traits::*;
pub use vanilla::*;
