//! Stochastic-process definitions used by the Monte Carlo engine.
//!
//! Start with [`gbm`] for vanilla equity / FX-style simulations and
//! [`brownian`] for additive Gaussian dynamics. When the `mc` feature is
//! enabled this module also exposes Heston, CIR, Hull-White / Vasicek,
//! jump-diffusion, Bates, and Schwartz-Smith models.
//!
//! Important assumptions such as time units, rate / volatility quoting, and
//! state-vector layout are documented in each process module. Use
//! [`metadata::ProcessMetadata`] when captured paths need a stable schema for
//! downstream consumers.

pub mod brownian;
pub mod correlation;
pub mod gbm;
pub mod gbm_dividends;
pub mod metadata;
pub mod multi_ou;

#[cfg(feature = "mc")]
pub mod heston;

#[cfg(feature = "mc")]
pub mod ou;

#[cfg(feature = "mc")]
pub mod schwartz_smith;

#[cfg(feature = "mc")]
pub mod cir;

#[cfg(feature = "mc")]
pub mod jump_diffusion;

#[cfg(feature = "mc")]
pub mod bates;

#[cfg(feature = "mc")]
pub mod lmm;

pub use brownian::*;
pub use correlation::*;
pub use gbm::*;
pub use gbm_dividends::*;
pub use metadata::*;
pub use multi_ou::*;
