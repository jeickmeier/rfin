//! Type aliases for interpolators with semantic names.
//!
//! Provides type-safe, semantically meaningful names for different interpolation
//! strategies using simple type aliases over the generic `Interpolator<S>`.

use super::{
    generic::Interpolator,
    strategies::{
        CubicHermiteStrategy, LinearStrategy, LogLinearStrategy, MonotoneConvexStrategy,
        PiecewiseQuadraticForwardStrategy,
    },
};

/// Piecewise linear interpolation on discount factors.
///
/// Simple linear interpolation between knot points. Fast and straightforward
/// but may produce negative forward rates (arbitrage) if discount factors
/// aren't carefully spaced. Prefer [`LogLinearDf`] or [`MonotoneConvex`] for yield curves.
///
/// # Example
///
/// ```rust
/// use finstack_core::math::interp::{LinearDf, ExtrapolationPolicy};
///
/// # fn main() -> finstack_core::Result<()> {
/// let knots = vec![0.0, 1.0, 2.0].into_boxed_slice();
/// let dfs = vec![1.0, 0.95, 0.90].into_boxed_slice();
/// let _interp = LinearDf::new(knots, dfs, ExtrapolationPolicy::FlatZero)?;
/// # Ok(())
/// # }
/// ```
pub type LinearDf = Interpolator<LinearStrategy>;

/// Log-linear interpolation for discount factors.
///
/// Performs linear interpolation on ln(DF), equivalent to piecewise-constant
/// zero rates. Guarantees positive forward rates and is commonly used for
/// government bond curves and simple yield curve construction.
///
/// # Example
///
/// ```rust
/// use finstack_core::math::interp::{LogLinearDf, ExtrapolationPolicy};
///
/// # fn main() -> finstack_core::Result<()> {
/// let knots = vec![0.0, 1.0, 2.0].into_boxed_slice();
/// let dfs = vec![1.0, 0.95, 0.90].into_boxed_slice();
/// let _interp = LogLinearDf::new(knots, dfs, ExtrapolationPolicy::FlatZero)?;
/// # Ok(())
/// # }
/// ```
pub type LogLinearDf = Interpolator<LogLinearStrategy>;

/// Monotone cubic Hermite interpolation (PCHIP).
///
/// Implements the Piecewise Cubic Hermite Interpolating Polynomial with
/// Fritsch-Carlson slope selection. Preserves monotonicity of input data,
/// ensuring no spurious oscillations. Requires monotone input discount factors.
///
/// # Algorithm
///
/// Uses weighted harmonic mean for slope selection:
/// - Ensures C¹ continuity (smooth first derivatives)
/// - Preserves monotonicity (no overshoots)
/// - Efficient O(log n) evaluation via binary search
///
/// # References
///
/// - Fritsch, F. N., & Carlson, R. E. (1980). "Monotone Piecewise Cubic Interpolation."
///   *SIAM Journal on Numerical Analysis*, 17(2), 238-246.
///
/// # Example
///
/// ```rust
/// use finstack_core::math::interp::{CubicHermite, ExtrapolationPolicy};
///
/// # fn main() -> finstack_core::Result<()> {
/// let knots = vec![0.0, 1.0, 2.0].into_boxed_slice();
/// let dfs = vec![1.0, 0.95, 0.90].into_boxed_slice();
/// let _interp = CubicHermite::new(knots, dfs, ExtrapolationPolicy::FlatZero)?;
/// # Ok(())
/// # }
/// ```
pub type CubicHermite = Interpolator<CubicHermiteStrategy>;

/// Monotone-convex discount-factor interpolator (Hagan & West, 2006).
///
/// Implements the full Hagan–West slope-selecting, monotone-convex cubic
/// interpolation in natural-log discount-factor space. This is the industry
/// standard for yield curve construction as it guarantees positive and
/// continuous forward rates.
///
/// ## Algorithm Overview
/// 1. Convert discount factors to continuously compounded yields: y = -ln(P)
/// 2. Compute initial derivatives using weighted harmonic means for smoothness
/// 3. Apply convexity constraint g(α,β) = α² + β² ≤ 9 to ensure positive forwards
/// 4. Build cubic coefficients for each segment: y(s) = a + bs + cs² + ds³
///
/// ## Numerical Stability
/// - Uses epsilon protection (100 × machine epsilon) for near-zero slopes
/// - Harmonic mean calculation protected against division by very small values
/// - Supports configurable extrapolation policy beyond input domain
///
/// The constructor computes and stores per-segment cubic coefficients guaranteeing
/// positivity, monotonicity and convexity when the input curve is arbitrage-free
/// (non-increasing). Evaluation is O(log N) due to binary search on the knot vector.
///
/// # References
///
/// - Hagan, P. S., & West, G. (2006). "Interpolation Methods for Curve Construction."
///   *Applied Mathematical Finance*, 13(2), 89-129.
/// - Hagan, P. S., & West, G. (2008). "Methods for Constructing a Yield Curve."
///   *Wilmott Magazine*, May 2008.
///
/// # Example
///
/// ```rust
/// use finstack_core::math::interp::{MonotoneConvex, ExtrapolationPolicy};
///
/// # fn main() -> finstack_core::Result<()> {
/// let knots = vec![0.0, 1.0, 2.0].into_boxed_slice();
/// let dfs = vec![1.0, 0.95, 0.90].into_boxed_slice();
/// let _interp = MonotoneConvex::new(knots, dfs, ExtrapolationPolicy::FlatZero)?;
/// # Ok(())
/// # }
/// ```
pub type MonotoneConvex = Interpolator<MonotoneConvexStrategy>;

/// Piecewise quadratic forward interpolation (smooth forward curve).
///
/// Builds a natural cubic spline in log-discount space, producing C² continuous
/// forward rates with flat-forward extrapolation behavior.
pub type PiecewiseQuadraticForward = Interpolator<PiecewiseQuadraticForwardStrategy>;
