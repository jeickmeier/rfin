//! Shock modes and bucket selectors for risk metrics.
//!
//! Defines how curves and surfaces are perturbed when computing sensitivities:
//! - `ShockMode`: Parallel (all points) vs. KeyRate (bucket-by-bucket).
//! - `BucketSelector`: Which buckets/knots to use for key-rate shocks.

/// Shock mode for sensitivity calculations.
///
/// Determines whether to apply parallel shocks (entire curve/surface) or
/// key-rate shocks (individual buckets/tenors).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShockMode {
    /// Key-rate shock: bump individual buckets/knots one at a time.
    /// Stores bucketed series in `MetricContext` and returns the total.
    KeyRate,

    /// Parallel shock: bump the entire curve/surface uniformly.
    /// Returns a single scalar sensitivity value.
    Parallel,
}

/// Bucket selector for key-rate shocks.
///
/// Determines which time points to use when applying key-rate shocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BucketSelector {
    /// Use standard buckets defined for the asset class.
    /// - IR: [0.25, 0.5, 1, 2, 3, 5, 7, 10, 15, 20, 30] years
    /// - Credit: [0.25, 0.5, 1, 2, 3, 5, 7, 10, 15, 20, 30] years
    /// - Equity vol: [1m, 3m, 6m, 1y, 2y, 3y, 5y]
    Standard,

    /// Derive buckets from the curve's knot points.
    /// Uses the actual knot times from the discount/hazard curve.
    CurveKnots,

    /// Derive buckets from the volatility surface grid.
    /// Uses the surface's expiry and strike grid points.
    SurfaceGrid,
}

impl Default for ShockMode {
    fn default() -> Self {
        Self::KeyRate
    }
}

impl Default for BucketSelector {
    fn default() -> Self {
        Self::Standard
    }
}
