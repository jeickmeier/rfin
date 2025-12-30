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
/// ```rust
/// use finstack_core::math::interp::{ExtrapolationPolicy, InterpFn, LinearDf};
///
/// # fn main() -> finstack_core::Result<()> {
/// let interp = LinearDf::new(
///     vec![0.0, 1.0, 2.0].into_boxed_slice(),
///     vec![1.0, 0.95, 0.90].into_boxed_slice(),
///     ExtrapolationPolicy::FlatZero,
/// )?;
/// let value = interp.interp(0.5);
/// # let _ = value;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(bound(serialize = "S: serde::Serialize"))]
#[serde(bound(deserialize = "S: serde::de::DeserializeOwned"))]
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
        // Centralized validation
        if knots.len() != values.len() {
            return Err(InputError::DimensionMismatch.into());
        }
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

    /// Construct a new interpolator allowing any values (including negative).
    ///
    /// This is useful for forward rate curves where negative rates are allowed
    /// (e.g., EUR, CHF, JPY markets since 2014).
    ///
    /// **Note:** LogLinear interpolation requires positive values (for log transform),
    /// so this constructor will fail for LogLinear with negative values.
    ///
    /// # Arguments
    /// * `knots` – strictly ascending knot times.
    /// * `values` – corresponding values (can be negative for Linear/LogLinear).
    /// * `extrapolation` – extrapolation policy.
    ///
    /// # Errors
    /// * `InputError::TooFewPoints` – fewer than two knots.
    /// * `InputError::NonMonotonicKnots` – knots not strictly increasing.
    /// * Strategy-specific errors from `S::from_raw`.
    #[allow(clippy::boxed_local)]
    pub fn new_allow_any_values(
        knots: Box<[f64]>,
        values: Box<[f64]>,
        extrapolation: ExtrapolationPolicy,
    ) -> crate::Result<Self> {
        // Validate knots but NOT values (allow negative rates)
        if knots.len() != values.len() {
            return Err(InputError::DimensionMismatch.into());
        }
        if knots.len() < 2 {
            return Err(InputError::TooFewPoints.into());
        }
        validate_knots(&knots)?;
        // Skip validate_positive_series for forward curves

        // Build strategy-specific state
        let strategy = S::from_raw(&knots, &values, extrapolation)?;

        Ok(Self {
            knots,
            values,
            strategy,
            extrapolation,
        })
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
