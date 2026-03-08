//! Shared logic for curve bumping via re-calibration.
//!
//! This module provides infrastructure for performing "what-if" analysis by
//! applying shifts to market data and observing the impact on calibrated objects.
//!
//! # Submodules
//! - `rates`: Bumping logic for Interest Rate curves.
//! - `hazard`: Bumping logic for Credit (Hazard) curves.
//! - `inflation`: Bumping logic for Inflation curves.
//! - `vol`: Bumping logic for Volatility surfaces (vega risk).

pub(crate) mod hazard;
pub(crate) mod inflation;
pub(crate) mod rates;
pub(crate) mod vol;

pub use hazard::{bump_hazard_shift, bump_hazard_spreads};
pub use inflation::{
    bump_inflation_rates, infer_currency_from_curve_id, observation_lag_from_curve,
};
pub use rates::{
    bump_discount_curve, bump_discount_curve_synthetic, infer_currency_from_discount_curve_id,
};
pub use vol::{bump_vol_surface, VolBumpRequest};

/// Request for a curve bump operation.
///
/// Defines the type and magnitude of a shift to be applied to market quotes
/// before re-calibration.
#[derive(Debug, Clone, PartialEq)]
pub enum BumpRequest {
    /// Parallel shift in basis points (additive to rates/spreads).
    ///
    /// # Example
    /// ```rust
    /// # use finstack_valuations::calibration::bumps::BumpRequest;
    /// let bp_10 = BumpRequest::Parallel(10.0);
    /// ```
    Parallel(f64),
    /// Node-specific shifts in basis points.
    ///
    /// Vector of `(Tenor in Years, Shift in BP)`. Used for key-rate durations
    /// and bucketed risk.
    Tenors(Vec<(f64, f64)>),
}
