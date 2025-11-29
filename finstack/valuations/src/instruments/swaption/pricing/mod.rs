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
//! ```rust,no_run
//! use finstack_valuations::instruments::swaption::BermudanSwaption;
//! use finstack_valuations::instruments::swaption::pricing::BermudanSwaptionTreeValuator;
//!
//! let swaption = BermudanSwaption::example();
//! // Use with HullWhiteTree for backward induction pricing
//! ```

pub mod tree_valuator;

pub use tree_valuator::BermudanSwaptionTreeValuator;
