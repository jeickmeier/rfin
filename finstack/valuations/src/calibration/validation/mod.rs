//! Market data validation and no-arbitrage constraints.
//!
//! This module is intentionally **imports/re-exports only**. Implementation lives in:
//! - `curves.rs`: curve validators (discount/forward/hazard/inflation/base-correlation)
//! - `surfaces.rs`: surface validators (volatility surfaces)
//! - `config.rs`: `ValidationConfig`
//! - `points.rs`: shared test grids/constants
//! - `quotes.rs`: quote validation utilities for `CalibrationPricer`

mod config;
mod curves;
mod points;
mod quotes;
mod surfaces;

#[cfg(test)]
mod tests;

pub use config::{
    default_rate_bounds_policy_for_serde, RateBounds, RateBoundsPolicy, ValidationConfig,
    ValidationMode,
};
pub use curves::CurveValidator;
pub use surfaces::SurfaceValidator;
