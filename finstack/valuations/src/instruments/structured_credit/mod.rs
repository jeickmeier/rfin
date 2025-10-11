//! Unified structured credit instrument module.
//!
//! This module consolidates the previously separate ABS, CLO, CMBS, and RMBS
//! implementations into a single `StructuredCredit` type, eliminating ~1,400 lines
//! of near-duplicate code.

pub mod metrics;
pub mod pricer;
pub mod types;

pub use pricer::StructuredCreditDiscountingPricer;
pub use types::{InstrumentSpecificFields, StructuredCredit};

