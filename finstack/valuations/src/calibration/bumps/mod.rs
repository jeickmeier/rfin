//! Shared logic for curve bumping via re-calibration.
//!
//! This module provides infrastructure for performing "what-if" analysis by
//! applying shifts to market data and observing the impact on calibrated objects.
//!
//! # Submodules
//! - [`rates`]: Bumping logic for Interest Rate curves.
//! - [`hazard`]: Bumping logic for Credit (Hazard) curves.
//! - [`inflation`]: Bumping logic for Inflation curves.

pub mod hazard;
pub mod inflation;
pub mod rates;

pub use hazard::{bump_hazard_shift, bump_hazard_spreads};
pub use inflation::bump_inflation_rates;
pub use rates::{bump_discount_curve, bump_discount_curve_synthetic};

/// Request for a curve bump operation.
///
/// Defines the type and magnitude of a shift to be applied to market quotes
/// before re-calibration.
#[derive(Clone, Debug, PartialEq)]
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
