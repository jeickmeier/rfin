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

pub mod tree_valuator;

pub use tree_valuator::BermudanSwaptionTreeValuator;
