//! Trait interface for interpolation functions.
//!
//! Defines the object-safe trait used by all interpolators, enabling
//! polymorphic interpolation with static or dynamic dispatch.

use core::fmt::Debug;

use super::types::{ExtrapolationPolicy, DERIVATIVE_EPSILON};

/// Object-safe interpolation trait.
pub trait InterpFn: Send + Sync + Debug {
    /// Interpolate at coordinate `x`.
    fn interp(&self, x: f64) -> f64;

    /// First derivative at `x`. Default via central finite differences.
    fn interp_prime(&self, x: f64) -> f64 {
        let h = (x.abs() * DERIVATIVE_EPSILON).max(1e-8);
        (self.interp(x + h) - self.interp(x - h)) / (2.0 * h)
    }
}

/// Strategy trait for interpolation algorithms.
///
/// Encapsulates the strategy-specific precomputed state (slopes, coefficients)
/// and evaluation logic. Used by the generic [`Interpolator<S>`] container
/// to delegate computation while sharing knot/value storage and validation.
///
/// # Design
///
/// Each concrete strategy (Linear, LogLinear, CubicHermite, MonotoneConvex)
/// implements this trait to provide:
/// - Construction from raw knots/values with validation
/// - Interpolation and derivative evaluation given knots/values/extrapolation
///
/// The generic `Interpolator<S>` struct holds the shared `knots`, `values`,
/// and `extrapolation` fields, while the strategy holds any precomputed data
/// (e.g., slopes for cubic, coefficients for monotone-convex).
pub trait InterpolationStrategy: Send + Sync + Debug {
    /// Build strategy-specific state from raw knots and values.
    ///
    /// # Arguments
    /// * `knots` – strictly ascending knot times (already validated by caller).
    /// * `values` – corresponding values (already validated by caller).
    /// * `extrapolation` – extrapolation policy.
    ///
    /// # Errors
    /// Strategy-specific validation errors (e.g., non-monotone for MonotoneConvex).
    fn from_raw(
        knots: &[f64],
        values: &[f64],
        extrapolation: ExtrapolationPolicy,
    ) -> crate::Result<Self>
    where
        Self: Sized;

    /// Interpolate at coordinate `x` using the strategy's algorithm.
    ///
    /// # Arguments
    /// * `x` – evaluation point.
    /// * `knots` – knot times (from parent Interpolator).
    /// * `values` – values (from parent Interpolator).
    /// * `extrapolation` – extrapolation policy (from parent Interpolator).
    fn interp(
        &self,
        x: f64,
        knots: &[f64],
        values: &[f64],
        extrapolation: ExtrapolationPolicy,
    ) -> f64;

    /// First derivative at `x` using the strategy's algorithm.
    ///
    /// # Arguments
    /// * `x` – evaluation point.
    /// * `knots` – knot times (from parent Interpolator).
    /// * `values` – values (from parent Interpolator).
    /// * `extrapolation` – extrapolation policy (from parent Interpolator).
    fn interp_prime(
        &self,
        x: f64,
        knots: &[f64],
        values: &[f64],
        extrapolation: ExtrapolationPolicy,
    ) -> f64;
}
