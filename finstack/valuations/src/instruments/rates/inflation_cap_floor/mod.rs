//! Inflation cap/floor options on YoY inflation.
//!
//! Provides simple Black-76 and Bachelier pricing on forward YoY inflation
//! derived from the inflation curve or index fixings.

/// Pricer implementations for inflation caps and floors.
pub mod pricer;
/// Type definitions for inflation caps and floors.
pub mod types;

pub use types::{InflationCapFloor, InflationCapFloorBuilder, InflationCapFloorType};
