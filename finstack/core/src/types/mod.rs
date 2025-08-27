//! Core type definitions for the finstack ecosystem
//!
//! This module provides fundamental types used throughout finstack:
//! - Phantom-typed identifiers for type safety
//! - Rate and percentage types with conversions
//! - Re-exports of commonly used types from other modules

pub mod id;
pub mod rates;

pub use id::{
    CounterpartyId, CurveId, Id, InstrumentId, PortfolioId, PositionId, ScenarioId, TradeId,
    TypeTag,
};
pub use rates::{Bps, Percentage, Rate};

// Re-export commonly used types from other modules for convenience
pub use crate::currency::Currency;
pub use crate::dates::{Date, OffsetDateTime, PrimitiveDateTime};
pub use crate::money::Money as Amount;

// Type aliases for common usage patterns
/// Convenient type alias for timestamps
pub type Timestamp = crate::dates::OffsetDateTime;
