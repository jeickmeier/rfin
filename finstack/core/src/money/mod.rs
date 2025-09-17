//! Money primitives and FX conversion utilities.
//!
//! This module provides:
//! - [`Money`] – a currency-tagged monetary amount with safe arithmetic
//! - [`fx`] – foreign-exchange interfaces and helpers used by conversions
//!
//! Arithmetic refuses to mix currencies unless converted explicitly using an
//! [`fx::FxProvider`]. Rounding follows per-currency scale with configurable
//! policies via [`crate::config`].
//!
//! # Examples
//! ```rust
//! use finstack_core::money::Money;
//! use finstack_core::currency::Currency;
//!
//! let gross = Money::new(125.25, Currency::USD);
//! let tax   = Money::new(10.00, Currency::USD);
//! let total = (gross + tax).unwrap();
//! assert_eq!(format!("{}", total), "USD 135.25");
//! ```
/// Submodule for FX interfaces.
pub mod fx;

mod rounding;
mod types;

pub use types::Money;
