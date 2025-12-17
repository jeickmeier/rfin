//! Shared logic for curve bumping via re-calibration.

pub mod hazard;
pub mod inflation;
pub mod rates;

mod plan;

pub use plan::PlanBumper;

/// Request for a curve bump operation.
#[derive(Clone, Debug, PartialEq)]
pub enum BumpRequest {
    /// Parallel shift in basis points (additive to rates/spreads).
    Parallel(f64),
    /// Node-specific shifts in basis points.
    /// Vector of (Tenor in Years, Shift in BP).
    Tenors(Vec<(f64, f64)>),
}
