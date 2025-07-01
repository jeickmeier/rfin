//! Core financial primitives.
//!
//! This module provides fundamental types for financial computations,
//! such as money, currency, and other basic financial concepts.

pub mod currency;
pub mod money;

// Re-export commonly used types
pub use currency::Currency;
pub use money::Money;
