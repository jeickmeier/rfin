//! Fourier pricing methods for European options.
//!
//! This module provides model-agnostic Fourier pricing engines that work
//! with any model implementing the [`CharacteristicFunction`] trait from
//! `finstack-core`. Two methods are implemented:
//!
//! - **COS method** ([`cos::CosPricer`]): Fang-Oosterlee (2008) cosine series
//!   expansion. O(N) per strike, fastest single-strike Fourier method.
//!   Automatic truncation range from cumulants. No external dependencies.
//!
//! - **Lewis method** ([`lewis::LewisPricer`]): Lewis (2001) single-integral
//!   formula. Gauss-Legendre quadrature for the semi-infinite integral.
//!   No grid, arbitrary strike, no interpolation needed.
//!
//! # Method Selection Guide
//!
//! | Use Case | Recommended Method |
//! |----------|-------------------|
//! | Single-strike pricing | COS (fastest) |
//! | Strip pricing (5-50 strikes) | COS with strip reuse |
//! | Arbitrary strike, no grid | Lewis |
//! | WASM / latency-critical | COS (smallest N) |
//!
//! # Example
//!
//! ```rust,no_run
//! use finstack_core::math::characteristic_function::BlackScholesCf;
//! use finstack_valuations::pricer::fourier::cos::{CosPricer, CosConfig};
//! use finstack_valuations::pricer::fourier::lewis::{LewisPricer, LewisConfig};
//!
//! let cf = BlackScholesCf { r: 0.05, q: 0.0, sigma: 0.2 };
//!
//! // COS method
//! let cos = CosPricer::new(&cf, CosConfig::default());
//! let call = cos.price_call(100.0, 100.0, 0.05, 0.0, 1.0);
//!
//! // Lewis method
//! let lewis = LewisPricer::new(&cf, LewisConfig::default());
//! let call2 = lewis.price_call(100.0, 100.0, 0.05, 0.0, 1.0);
//! ```
//!
//! [`CharacteristicFunction`]: finstack_core::math::characteristic_function::CharacteristicFunction

pub mod cos;
pub mod lewis;

pub use cos::{CosConfig, CosPricer};
pub use lewis::{LewisConfig, LewisPricer};
