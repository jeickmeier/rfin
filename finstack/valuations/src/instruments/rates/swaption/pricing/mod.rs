//! Bermudan swaption pricing engines.
//!
//! This module provides specialized pricing engines for Bermudan swaptions
//! using tree-based and Monte Carlo methods.
//!
//! # Available Engines
//!
//! - [`tree_valuator`] - Hull-White tree-based pricing (industry standard)
//!
//! # Example
//!
//! ```text
//! use finstack_valuations::instruments::rates::swaption::BermudanSwaption;
//! use finstack_valuations::instruments::rates::swaption::pricing::BermudanSwaptionTreeValuator;
//!
//! let swaption = BermudanSwaption::example();
//! // Use with HullWhiteTree for backward induction pricing
//! ```

#[cfg(feature = "mc")]
pub mod lmm_bermudan;
#[cfg(feature = "mc")]
pub mod monte_carlo_lsmc;
#[cfg(feature = "mc")]
pub mod monte_carlo_payoff;
#[cfg(feature = "mc")]
pub mod swap_rate_utils;
pub mod tree_valuator;

pub use tree_valuator::BermudanSwaptionTreeValuator;
