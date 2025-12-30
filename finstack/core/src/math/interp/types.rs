//! Interpolation types and style enumeration.
//!
//! Defines the `InterpStyle` enum for selecting interpolation methods and
//! the internal `Interp` enum for static dispatch in hot paths.

use super::traits::InterpFn;
use super::wrappers::{
    CubicHermite, LinearDf, LogLinearDf, MonotoneConvex, PiecewiseQuadraticForward,
};

/// Epsilon for finite difference derivative calculations.
///
/// This is the default value used in [`InterpFn::interp_prime`] for numerical
/// derivatives.
pub const DERIVATIVE_EPSILON: f64 = 1e-6;

/// Validation policy for interpolator input values.
///
/// Controls whether the interpolator validates that all values are positive
/// (required for discount factors) or allows any finite values (needed for
/// forward rate curves where negative rates are possible).
///
/// # Examples
///
/// ```rust
/// use finstack_core::math::interp::{ValidationPolicy, LinearDf, ExtrapolationPolicy};
///
/// # fn main() -> finstack_core::Result<()> {
/// // Strict validation (default) - requires positive values
/// let strict = LinearDf::new(
///     vec![0.0, 1.0, 2.0].into_boxed_slice(),
///     vec![1.0, 0.95, 0.90].into_boxed_slice(),
///     ExtrapolationPolicy::FlatZero,
///     ValidationPolicy::Strict,
/// )?;
///
/// // AllowNegative - permits negative values (e.g., for forward curves)
/// let relaxed = LinearDf::new(
///     vec![0.0, 1.0, 2.0].into_boxed_slice(),
///     vec![-0.01, 0.0, 0.01].into_boxed_slice(),
///     ExtrapolationPolicy::FlatZero,
///     ValidationPolicy::AllowNegative,
/// )?;
/// # Ok(())
/// # }
/// ```
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ValidationPolicy {
    /// Require all values to be positive (default).
    ///
    /// This is appropriate for discount factors and other quantities that must
    /// remain positive to avoid arbitrage.
    #[default]
    Strict,
    /// Allow any finite values, including negative and zero.
    ///
    /// Use this for forward rate curves where negative rates are allowed
    /// (e.g., EUR, CHF, JPY markets since 2014).
    ///
    /// **Note:** Some interpolation methods (LogLinear, MonotoneConvex) require
    /// positive values for mathematical reasons and will still reject negative
    /// values even with this policy.
    AllowNegative,
}

/// Extrapolation policy for evaluation outside the knot range.
#[derive(Copy, Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ExtrapolationPolicy {
    /// Constant value extension.
    #[default]
    FlatZero,
    /// Tangent/forward extension using boundary slope.
    FlatForward,
}

/// Enum of supported interpolation styles. The default is `Linear`.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum InterpStyle {
    /// Linear interpolation in values.
    #[default]
    Linear,
    /// Log‐linear interpolation of positive values.
    LogLinear,
    /// Monotone‐convex interpolation of positive, non-increasing values.
    MonotoneConvex,
    /// Cubic Hermite interpolation (monotone-preserving slopes).
    CubicHermite,
    /// Piecewise quadratic forwards (smooth forward curve, C²).
    PiecewiseQuadraticForward,
}

/// Crate-private enum enabling static dispatch for interpolation in hot loops.
///
/// Storing this enum (instead of `Box<dyn InterpFn>`) allows the compiler to
/// inline calls to `interp` and `interp_prime` for each concrete variant.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Interp {
    Linear(LinearDf),
    LogLinear(LogLinearDf),
    MonotoneConvex(MonotoneConvex),
    CubicHermite(CubicHermite),
    PiecewiseQuadraticForward(PiecewiseQuadraticForward),
}

impl Interp {
    #[inline]
    pub(crate) fn interp(&self, x: f64) -> f64 {
        match self {
            Interp::Linear(i) => i.interp(x),
            Interp::LogLinear(i) => i.interp(x),
            Interp::MonotoneConvex(i) => i.interp(x),
            Interp::CubicHermite(i) => i.interp(x),
            Interp::PiecewiseQuadraticForward(i) => i.interp(x),
        }
    }

    /// Get the interpolation style of this Interp
    pub(crate) fn style(&self) -> InterpStyle {
        match self {
            Interp::Linear(_) => InterpStyle::Linear,
            Interp::LogLinear(_) => InterpStyle::LogLinear,
            Interp::MonotoneConvex(_) => InterpStyle::MonotoneConvex,
            Interp::CubicHermite(_) => InterpStyle::CubicHermite,
            Interp::PiecewiseQuadraticForward(_) => InterpStyle::PiecewiseQuadraticForward,
        }
    }

    /// Get the extrapolation policy of this Interp
    pub fn extrapolation(&self) -> ExtrapolationPolicy {
        match self {
            Interp::Linear(i) => i.extrapolation(),
            Interp::LogLinear(i) => i.extrapolation(),
            Interp::MonotoneConvex(i) => i.extrapolation(),
            Interp::CubicHermite(i) => i.extrapolation(),
            Interp::PiecewiseQuadraticForward(i) => i.extrapolation(),
        }
    }
}

impl InterpStyle {
    /// Build a boxed interpolator implementing [`InterpFn`].
    ///
    /// # Arguments
    /// * `knots` – strictly ascending knot times.
    /// * `values` – corresponding values.
    /// * `extrapolation` – extrapolation policy.
    /// * `validation` – validation policy controlling whether negative values are allowed.
    pub fn build(
        self,
        knots: Box<[f64]>,
        values: Box<[f64]>,
        extrapolation: ExtrapolationPolicy,
        validation: ValidationPolicy,
    ) -> crate::Result<Box<dyn InterpFn>> {
        // LogLinear and MonotoneConvex require positive values for mathematical reasons
        let effective_validation = match self {
            InterpStyle::LogLinear | InterpStyle::MonotoneConvex => ValidationPolicy::Strict,
            _ => validation,
        };

        match self {
            InterpStyle::Linear => Ok(Box::new(LinearDf::new(
                knots,
                values,
                extrapolation,
                effective_validation,
            )?)),
            InterpStyle::LogLinear => Ok(Box::new(LogLinearDf::new(
                knots,
                values,
                extrapolation,
                effective_validation,
            )?)),
            InterpStyle::MonotoneConvex => Ok(Box::new(MonotoneConvex::new(
                knots,
                values,
                extrapolation,
                effective_validation,
            )?)),
            InterpStyle::CubicHermite => Ok(Box::new(CubicHermite::new(
                knots,
                values,
                extrapolation,
                effective_validation,
            )?)),
            InterpStyle::PiecewiseQuadraticForward => Ok(Box::new(PiecewiseQuadraticForward::new(
                knots,
                values,
                extrapolation,
                effective_validation,
            )?)),
        }
    }

    /// Build an enum-backed interpolator enabling static dispatch.
    ///
    /// # Arguments
    /// * `knots` – strictly ascending knot times.
    /// * `values` – corresponding values.
    /// * `extrapolation` – extrapolation policy.
    /// * `validation` – validation policy controlling whether negative values are allowed.
    ///
    /// **Note:** LogLinear and MonotoneConvex require positive values for mathematical
    /// reasons (log transform, monotonicity). These will enforce `Strict` validation
    /// regardless of the passed policy.
    #[inline]
    pub(crate) fn build_enum(
        self,
        knots: Box<[f64]>,
        values: Box<[f64]>,
        extrapolation: ExtrapolationPolicy,
        validation: ValidationPolicy,
    ) -> crate::Result<Interp> {
        // LogLinear and MonotoneConvex require positive values for mathematical reasons
        let effective_validation = match self {
            InterpStyle::LogLinear | InterpStyle::MonotoneConvex => ValidationPolicy::Strict,
            _ => validation,
        };

        let interp = match self {
            InterpStyle::Linear => Interp::Linear(LinearDf::new(
                knots,
                values,
                extrapolation,
                effective_validation,
            )?),
            InterpStyle::LogLinear => Interp::LogLinear(LogLinearDf::new(
                knots,
                values,
                extrapolation,
                effective_validation,
            )?),
            InterpStyle::MonotoneConvex => Interp::MonotoneConvex(MonotoneConvex::new(
                knots,
                values,
                extrapolation,
                effective_validation,
            )?),
            InterpStyle::CubicHermite => Interp::CubicHermite(CubicHermite::new(
                knots,
                values,
                extrapolation,
                effective_validation,
            )?),
            InterpStyle::PiecewiseQuadraticForward => Interp::PiecewiseQuadraticForward(
                PiecewiseQuadraticForward::new(knots, values, extrapolation, effective_validation)?,
            ),
        };
        Ok(interp)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, clippy::indexing_slicing)]
mod tests {
    use super::*;

    fn standard_knots() -> Box<[f64]> {
        vec![0.0, 1.0, 2.0, 3.0].into_boxed_slice()
    }

    fn standard_dfs() -> Box<[f64]> {
        vec![1.0, 0.95, 0.9, 0.85].into_boxed_slice()
    }

    fn approx_eq(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() < tol
    }

    // ========================================================================
    // build_enum Tests with ValidationPolicy
    // ========================================================================

    #[test]
    fn build_enum_linear_strict() {
        let result = InterpStyle::Linear.build_enum(
            standard_knots(),
            standard_dfs(),
            ExtrapolationPolicy::FlatZero,
            ValidationPolicy::Strict,
        );
        assert!(result.is_ok());
        let interp = result.expect("should build in test");
        assert!(approx_eq(interp.interp(0.0), 1.0, 1e-12));
    }

    #[test]
    fn build_enum_log_linear_strict() {
        let result = InterpStyle::LogLinear.build_enum(
            standard_knots(),
            standard_dfs(),
            ExtrapolationPolicy::FlatForward,
            ValidationPolicy::Strict,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn build_enum_monotone_convex_strict() {
        let result = InterpStyle::MonotoneConvex.build_enum(
            standard_knots(),
            standard_dfs(),
            ExtrapolationPolicy::FlatZero,
            ValidationPolicy::Strict,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn build_enum_cubic_hermite_strict() {
        let result = InterpStyle::CubicHermite.build_enum(
            standard_knots(),
            standard_dfs(),
            ExtrapolationPolicy::FlatForward,
            ValidationPolicy::Strict,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn build_enum_piecewise_quadratic_strict() {
        let result = InterpStyle::PiecewiseQuadraticForward.build_enum(
            standard_knots(),
            standard_dfs(),
            ExtrapolationPolicy::FlatZero,
            ValidationPolicy::Strict,
        );
        assert!(result.is_ok());
    }

    // ========================================================================
    // ValidationPolicy::AllowNegative Tests
    // ========================================================================

    #[test]
    fn allow_negative_linear_with_negative_values() {
        let knots = vec![0.0, 1.0, 2.0].into_boxed_slice();
        let values = vec![-0.5, -0.2, 0.1].into_boxed_slice();
        let result = InterpStyle::Linear.build_enum(
            knots,
            values,
            ExtrapolationPolicy::FlatZero,
            ValidationPolicy::AllowNegative,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn allow_negative_cubic_with_negative_values() {
        let knots = vec![0.0, 1.0, 2.0, 3.0].into_boxed_slice();
        let values = vec![-1.0, -0.5, 0.0, 0.5].into_boxed_slice();
        let result = InterpStyle::CubicHermite.build_enum(
            knots,
            values,
            ExtrapolationPolicy::FlatForward,
            ValidationPolicy::AllowNegative,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn log_linear_rejects_negative_even_with_allow_negative_policy() {
        let knots = vec![0.0, 1.0, 2.0].into_boxed_slice();
        let values = vec![-0.5, 0.2, 0.5].into_boxed_slice();
        let result = InterpStyle::LogLinear.build_enum(
            knots,
            values,
            ExtrapolationPolicy::FlatZero,
            ValidationPolicy::AllowNegative, // Should still enforce Strict for LogLinear
        );
        // LogLinear requires positive values for log transform
        assert!(result.is_err());
    }

    #[test]
    fn monotone_convex_rejects_negative_even_with_allow_negative_policy() {
        let knots = vec![0.0, 1.0, 2.0].into_boxed_slice();
        let values = vec![-1.0, -0.5, -0.2].into_boxed_slice();
        let result = InterpStyle::MonotoneConvex.build_enum(
            knots,
            values,
            ExtrapolationPolicy::FlatZero,
            ValidationPolicy::AllowNegative, // Should still enforce Strict for MonotoneConvex
        );
        assert!(result.is_err());
    }

    #[test]
    fn strict_rejects_negative_for_linear() {
        let knots = vec![0.0, 1.0, 2.0].into_boxed_slice();
        let values = vec![-0.5, -0.2, 0.1].into_boxed_slice();
        let result = InterpStyle::Linear.build_enum(
            knots,
            values,
            ExtrapolationPolicy::FlatZero,
            ValidationPolicy::Strict,
        );
        assert!(result.is_err());
    }

    // ========================================================================
    // Interp Enum Method Tests
    // ========================================================================

    #[test]
    fn interp_method_all_variants() {
        let styles = vec![
            InterpStyle::Linear,
            InterpStyle::LogLinear,
            InterpStyle::MonotoneConvex,
            InterpStyle::CubicHermite,
            InterpStyle::PiecewiseQuadraticForward,
        ];

        for style in styles {
            let interp = style
                .build_enum(
                    standard_knots(),
                    standard_dfs(),
                    ExtrapolationPolicy::FlatZero,
                    ValidationPolicy::Strict,
                )
                .expect("should build in test");
            assert!(approx_eq(interp.interp(0.0), 1.0, 1e-10));
        }
    }

    #[test]
    fn style_method_returns_correct_type() {
        let linear = InterpStyle::Linear
            .build_enum(
                standard_knots(),
                standard_dfs(),
                ExtrapolationPolicy::FlatZero,
                ValidationPolicy::Strict,
            )
            .expect("should build in test");
        assert_eq!(linear.style(), InterpStyle::Linear);

        let log_linear = InterpStyle::LogLinear
            .build_enum(
                standard_knots(),
                standard_dfs(),
                ExtrapolationPolicy::FlatZero,
                ValidationPolicy::Strict,
            )
            .expect("should build in test");
        assert_eq!(log_linear.style(), InterpStyle::LogLinear);
    }

    #[test]
    fn extrapolation_method_works() {
        let flat_zero = InterpStyle::Linear
            .build_enum(
                standard_knots(),
                standard_dfs(),
                ExtrapolationPolicy::FlatZero,
                ValidationPolicy::Strict,
            )
            .expect("should build in test");
        let _ = flat_zero.extrapolation();

        let flat_fwd = InterpStyle::LogLinear
            .build_enum(
                standard_knots(),
                standard_dfs(),
                ExtrapolationPolicy::FlatForward,
                ValidationPolicy::Strict,
            )
            .expect("should build in test");
        let _ = flat_fwd.extrapolation();
    }

    // ========================================================================
    // ExtrapolationPolicy Tests
    // ========================================================================

    #[test]
    fn extrapolation_default_is_flat_zero() {
        let default = ExtrapolationPolicy::default();
        match default {
            ExtrapolationPolicy::FlatZero => {}
            _ => panic!("Default should be FlatZero"),
        }
    }

    // ========================================================================
    // ValidationPolicy Tests
    // ========================================================================

    #[test]
    fn validation_policy_default_is_strict() {
        let default = ValidationPolicy::default();
        assert_eq!(default, ValidationPolicy::Strict);
    }

    #[test]
    fn validation_policy_equality() {
        assert_eq!(ValidationPolicy::Strict, ValidationPolicy::Strict);
        assert_eq!(ValidationPolicy::AllowNegative, ValidationPolicy::AllowNegative);
        assert_ne!(ValidationPolicy::Strict, ValidationPolicy::AllowNegative);
    }

    // ========================================================================
    // DERIVATIVE_EPSILON Tests
    // ========================================================================

    #[test]
    fn derivative_epsilon_defined() {
        assert_eq!(DERIVATIVE_EPSILON, 1e-6);
    }

    #[test]
    fn derivative_epsilon_usage() {
        let epsilon = DERIVATIVE_EPSILON;
        let f = |x: f64| x * x;
        let x = 2.0;
        let true_deriv = 2.0 * x;
        let approx_deriv = (f(x + epsilon) - f(x)) / epsilon;
        assert!((approx_deriv - true_deriv).abs() < 1e-5);
    }

    // ========================================================================
    // InterpStyle PartialEq Tests
    // ========================================================================

    #[test]
    fn interp_style_equality() {
        assert_eq!(InterpStyle::Linear, InterpStyle::Linear);
        assert_eq!(InterpStyle::LogLinear, InterpStyle::LogLinear);
    }

    #[test]
    fn interp_style_inequality() {
        assert_ne!(InterpStyle::Linear, InterpStyle::LogLinear);
        assert_ne!(InterpStyle::LogLinear, InterpStyle::MonotoneConvex);
    }
}
