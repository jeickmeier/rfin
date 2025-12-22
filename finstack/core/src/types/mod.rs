//! Core type definitions for the FinStack ecosystem.
//!
//! This module collects phantom-typed identifiers (`CurveId`, `InstrumentId`, …),
//! rate helpers (`Rate`, `Bps`), credit ratings, and convenient aliases used
//! throughout the platform.
//!
//! # Examples
//! ```rust
//! use finstack_core::types::{CurveId, Rate, Percentage};
//!
//! let curve_id = CurveId::from("USD-OIS");
//! let rate = Rate::from_percent(5.0);
//! let pct = Percentage::new(25.0);
//! assert_eq!(curve_id.as_str(), "USD-OIS");
//! assert_eq!(rate.as_decimal(), 0.05);
//! assert_eq!(pct.as_percent(), 25.0);
//! ```

pub mod id;
pub mod rates;
pub mod ratings;

pub use id::{CurveId, Id, IndexId, InstrumentId, PriceId, TypeTag, UnderlyingId};
pub use rates::{Bps, Percentage, Rate};
pub use ratings::{moodys_warf_factor, CreditRating, RatingFactorTable};

// Re-export commonly used types from other modules for convenience
pub use crate::currency::Currency;
pub use crate::dates::{Date, OffsetDateTime, PrimitiveDateTime};

// Type aliases for common usage patterns
/// Convenient type alias for timestamps
pub type Timestamp = crate::dates::OffsetDateTime;
