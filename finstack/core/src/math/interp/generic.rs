//! Generic interpolator container with strategy pattern.
//!
//! Provides [`Interpolator<S>`], a unified storage container for all interpolation
//! algorithms, using the strategy pattern to avoid code duplication while maintaining
//! flexibility and type safety.

use super::{
    traits::{InterpFn, InterpolationStrategy},
    types::ExtrapolationPolicy,
    utils::{validate_knots, validate_positive_series},
};
use crate::error::InputError;

/// Generic interpolator with strategy pattern.
///
/// Unifies knot/value storage and validation across all interpolation methods.
/// The strategy `S` encapsulates algorithm-specific precomputed data (slopes,
/// coefficients) and evaluation logic.
///
/// # Design
///
/// - **Shared state**: `knots`, `values`, `extrapolation` are common to all interpolators.
/// - **Strategy-specific state**: Stored in the generic `strategy: S` field.
/// - **Validation**: Centralized in the constructor; strategies can add extra checks.
/// - **Serialization**: Preserved via custom serde on wrapper types (LinearDf, etc.).
///
/// # Example
///
/// ```ignore
/// use finstack_core::math::interp::{Interpolator, LinearStrategy};
///
/// let interp = Interpolator::<LinearStrategy>::new(
///     vec![0.0, 1.0, 2.0].into_boxed_slice(),
///     vec![1.0, 0.95, 0.90].into_boxed_slice(),
///     ExtrapolationPolicy::FlatZero,
/// )?;
/// let value = interp.interp(0.5);
/// ```
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(bound(serialize = "S: serde::Serialize")))]
#[cfg_attr(feature = "serde", serde(bound(deserialize = "S: serde::de::DeserializeOwned")))]
pub struct Interpolator<S: InterpolationStrategy> {
    /// Strictly increasing knot times.
    knots: Box<[f64]>,
    /// Values at each knot (semantics depend on strategy).
    values: Box<[f64]>,
    /// Strategy-specific precomputed state.
    strategy: S,
    /// Extrapolation policy for out-of-bounds evaluation.
    extrapolation: ExtrapolationPolicy,
}

impl<S: InterpolationStrategy> Interpolator<S> {
    /// Construct a new interpolator with the given strategy.
    ///
    /// # Arguments
    /// * `knots` – strictly ascending knot times.
    /// * `values` – corresponding values (semantics depend on strategy).
    /// * `extrapolation` – extrapolation policy.
    ///
    /// # Errors
    /// * `InputError::TooFewPoints` – fewer than two knots.
    /// * `InputError::NonMonotonicKnots` – knots not strictly increasing.
    /// * `InputError::NonPositiveValue` – values not positive (if required by strategy).
    /// * Strategy-specific errors from `S::from_raw`.
    #[allow(clippy::boxed_local)]
    pub fn new(
        knots: Box<[f64]>,
        values: Box<[f64]>,
        extrapolation: ExtrapolationPolicy,
    ) -> crate::Result<Self> {
        debug_assert_eq!(knots.len(), values.len());
        
        // Centralized validation
        if knots.len() < 2 {
            return Err(InputError::TooFewPoints.into());
        }
        validate_knots(&knots)?;
        validate_positive_series(&values)?;

        // Build strategy-specific state
        let strategy = S::from_raw(&knots, &values, extrapolation)?;

        Ok(Self {
            knots,
            values,
            strategy,
            extrapolation,
        })
    }

    /// Construct a new interpolator from pre-built components.
    ///
    /// This is a lower-level constructor that skips validation and strategy
    /// construction, used when the caller has already validated and built
    /// the strategy (e.g., with custom parameters).
    ///
    /// # Safety
    /// The caller must ensure:
    /// - `knots` and `values` have the same length (≥ 2)
    /// - `knots` are strictly increasing
    /// - `values` are positive (if required)
    /// - `strategy` is correctly built from the same knots/values
    #[allow(clippy::boxed_local)]
    #[allow(dead_code)]
    pub(crate) fn from_parts(
        knots: Box<[f64]>,
        values: Box<[f64]>,
        strategy: S,
        extrapolation: ExtrapolationPolicy,
    ) -> Self {
        debug_assert_eq!(knots.len(), values.len());
        Self {
            knots,
            values,
            strategy,
            extrapolation,
        }
    }

    /// Access the knots (for internal use or serialization).
    #[inline]
    #[allow(dead_code)]
    pub(crate) fn knots(&self) -> &[f64] {
        &self.knots
    }

    /// Access the values (for internal use or serialization).
    #[inline]
    #[allow(dead_code)]
    pub(crate) fn values(&self) -> &[f64] {
        &self.values
    }

    /// Access the strategy (for internal use or serialization).
    #[inline]
    #[allow(dead_code)]
    pub(crate) fn strategy(&self) -> &S {
        &self.strategy
    }

    /// Get the extrapolation policy.
    #[inline]
    pub fn extrapolation(&self) -> ExtrapolationPolicy {
        self.extrapolation
    }
}

impl<S: InterpolationStrategy> InterpFn for Interpolator<S> {
    #[inline]
    fn interp(&self, x: f64) -> f64 {
        self.strategy
            .interp(x, &self.knots, &self.values, self.extrapolation)
    }

    #[inline]
    fn interp_prime(&self, x: f64) -> f64 {
        self.strategy
            .interp_prime(x, &self.knots, &self.values, self.extrapolation)
    }
}

