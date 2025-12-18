//! Market data validation and no-arbitrage constraints.
//!
//! This module provides the infrastructure for performing runtime validation
//! of market data, calibration inputs, and calibrated results. It ensures
//! that results conform to financial reality (e.g., non-negative hazard rates,
//! positive discount factors, no-arbitrage volatility surfaces).
//!
//! # Submodules
//! - [`config`]: Configuration for validation thresholds and strictness modes.
//! - [`curves`]: Runtime validators for term structures (Yield, Hazard, Inflation).
//! - [`surfaces`]: Runtime validators for volatility and correlation surfaces.
//! - [`quotes`]: Logic for validating market quotes against settlement rules.

mod config;
mod curves;
mod points;
mod quotes;
mod surfaces;

pub use config::{
    default_rate_bounds_policy_for_serde, RateBounds, RateBoundsPolicy, ValidationConfig,
    ValidationMode,
};
pub use curves::CurveValidator;
pub use surfaces::SurfaceValidator;
