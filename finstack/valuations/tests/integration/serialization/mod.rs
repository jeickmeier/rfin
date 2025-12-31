//! Serialization Integration Tests
//!
//! Round-trip serialization tests ensuring data integrity:
//!
//! - [`instrument_roundtrip`]: JSON serialization for all instrument types
//! - [`result_roundtrip`]: ValuationResult serialization
//! - [`market_compliance`]: Golden tests for pricing parity against reference values

pub mod instrument_roundtrip;
pub mod market_compliance;
#[cfg(feature = "serde")]
pub mod result_roundtrip;
