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

/// Extrapolation policy for evaluation outside the knot range.
#[derive(Copy, Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum ExtrapolationPolicy {
    /// Constant value extension.
    #[default]
    FlatZero,
    /// Tangent/forward extension using boundary slope.
    FlatForward,
}

/// Enum of supported interpolation styles. The default is `Linear`.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
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
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
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
    #[cfg(feature = "serde")]
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
    #[cfg(feature = "serde")]
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
    pub fn build(
        self,
        knots: Box<[f64]>,
        values: Box<[f64]>,
        extrapolation: ExtrapolationPolicy,
    ) -> crate::Result<Box<dyn InterpFn>> {
        match self {
            InterpStyle::Linear => Ok(Box::new(LinearDf::new(knots, values, extrapolation)?)),
            InterpStyle::LogLinear => Ok(Box::new(LogLinearDf::new(knots, values, extrapolation)?)),
            InterpStyle::MonotoneConvex => {
                Ok(Box::new(MonotoneConvex::new(knots, values, extrapolation)?))
            }
            InterpStyle::CubicHermite => {
                Ok(Box::new(CubicHermite::new(knots, values, extrapolation)?))
            }
            InterpStyle::PiecewiseQuadraticForward => Ok(Box::new(PiecewiseQuadraticForward::new(
                knots,
                values,
                extrapolation,
            )?)),
        }
    }

    /// Build an enum-backed interpolator enabling static dispatch.
    #[inline]
    pub(crate) fn build_enum(
        self,
        knots: Box<[f64]>,
        values: Box<[f64]>,
        extrapolation: ExtrapolationPolicy,
    ) -> crate::Result<Interp> {
        let interp = match self {
            InterpStyle::Linear => Interp::Linear(LinearDf::new(knots, values, extrapolation)?),
            InterpStyle::LogLinear => {
                Interp::LogLinear(LogLinearDf::new(knots, values, extrapolation)?)
            }
            InterpStyle::MonotoneConvex => {
                Interp::MonotoneConvex(MonotoneConvex::new(knots, values, extrapolation)?)
            }
            InterpStyle::CubicHermite => {
                Interp::CubicHermite(CubicHermite::new(knots, values, extrapolation)?)
            }
            InterpStyle::PiecewiseQuadraticForward => Interp::PiecewiseQuadraticForward(
                PiecewiseQuadraticForward::new(knots, values, extrapolation)?,
            ),
        };
        Ok(interp)
    }

    /// Build an enum-backed interpolator allowing any values (including negative).
    ///
    /// This is useful for forward rate curves where negative rates are allowed
    /// (e.g., EUR, CHF, JPY markets since 2014).
    ///
    /// **Note:** LogLinear and MonotoneConvex require positive values for mathematical
    /// reasons (log transform, monotonicity). Using them with negative values will fail.
    #[inline]
    pub(crate) fn build_enum_allow_any_values(
        self,
        knots: Box<[f64]>,
        values: Box<[f64]>,
        extrapolation: ExtrapolationPolicy,
    ) -> crate::Result<Interp> {
        let interp =
            match self {
                InterpStyle::Linear => Interp::Linear(LinearDf::new_allow_any_values(
                    knots,
                    values,
                    extrapolation,
                )?),
                InterpStyle::LogLinear => {
                    // LogLinear requires positive values for log transform
                    Interp::LogLinear(LogLinearDf::new(knots, values, extrapolation)?)
                }
                InterpStyle::MonotoneConvex => {
                    // MonotoneConvex typically requires positive values
                    Interp::MonotoneConvex(MonotoneConvex::new(knots, values, extrapolation)?)
                }
                InterpStyle::CubicHermite => Interp::CubicHermite(
                    CubicHermite::new_allow_any_values(knots, values, extrapolation)?,
                ),
                InterpStyle::PiecewiseQuadraticForward => Interp::PiecewiseQuadraticForward(
                    PiecewiseQuadraticForward::new(knots, values, extrapolation)?,
                ),
            };
        Ok(interp)
    }
}

#[cfg(test)]
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
    // build_enum Tests
    // ========================================================================

    #[test]
    fn build_enum_linear() {
        let result = InterpStyle::Linear.build_enum(
            standard_knots(),
            standard_dfs(),
            ExtrapolationPolicy::FlatZero,
        );
        assert!(result.is_ok());
        let interp = result.expect("should build in test");
        assert!(approx_eq(interp.interp(0.0), 1.0, 1e-12));
    }

    #[test]
    fn build_enum_log_linear() {
        let result = InterpStyle::LogLinear.build_enum(
            standard_knots(),
            standard_dfs(),
            ExtrapolationPolicy::FlatForward,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn build_enum_monotone_convex() {
        let result = InterpStyle::MonotoneConvex.build_enum(
            standard_knots(),
            standard_dfs(),
            ExtrapolationPolicy::FlatZero,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn build_enum_cubic_hermite() {
        let result = InterpStyle::CubicHermite.build_enum(
            standard_knots(),
            standard_dfs(),
            ExtrapolationPolicy::FlatForward,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn build_enum_piecewise_quadratic() {
        let result = InterpStyle::PiecewiseQuadraticForward.build_enum(
            standard_knots(),
            standard_dfs(),
            ExtrapolationPolicy::FlatZero,
        );
        assert!(result.is_ok());
    }

    // ========================================================================
    // build_enum_allow_any_values Tests
    // ========================================================================

    #[test]
    fn allow_any_linear_with_negative() {
        let knots = vec![0.0, 1.0, 2.0].into_boxed_slice();
        let values = vec![-0.5, -0.2, 0.1].into_boxed_slice();
        let result = InterpStyle::Linear.build_enum_allow_any_values(
            knots,
            values,
            ExtrapolationPolicy::FlatZero,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn allow_any_cubic_with_negative() {
        let knots = vec![0.0, 1.0, 2.0, 3.0].into_boxed_slice();
        let values = vec![-1.0, -0.5, 0.0, 0.5].into_boxed_slice();
        let result = InterpStyle::CubicHermite.build_enum_allow_any_values(
            knots,
            values,
            ExtrapolationPolicy::FlatForward,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn allow_any_log_linear_rejects_negative() {
        let knots = vec![0.0, 1.0, 2.0].into_boxed_slice();
        let values = vec![-0.5, 0.2, 0.5].into_boxed_slice();
        let result = InterpStyle::LogLinear.build_enum_allow_any_values(
            knots,
            values,
            ExtrapolationPolicy::FlatZero,
        );
        // LogLinear still requires positive values
        assert!(result.is_err());
    }

    #[test]
    fn allow_any_monotone_convex_rejects_negative() {
        let knots = vec![0.0, 1.0, 2.0].into_boxed_slice();
        let values = vec![-1.0, -0.5, -0.2].into_boxed_slice();
        let result = InterpStyle::MonotoneConvex.build_enum_allow_any_values(
            knots,
            values,
            ExtrapolationPolicy::FlatZero,
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
                )
                .expect("should build in test");
            assert!(approx_eq(interp.interp(0.0), 1.0, 1e-10));
        }
    }

    #[test]
    #[cfg(feature = "serde")]
    fn style_method_returns_correct_type() {
        let linear = InterpStyle::Linear
            .build_enum(
                standard_knots(),
                standard_dfs(),
                ExtrapolationPolicy::FlatZero,
            )
            .expect("should build in test");
        assert_eq!(linear.style(), InterpStyle::Linear);

        let log_linear = InterpStyle::LogLinear
            .build_enum(
                standard_knots(),
                standard_dfs(),
                ExtrapolationPolicy::FlatZero,
            )
            .expect("should build in test");
        assert_eq!(log_linear.style(), InterpStyle::LogLinear);
    }

    #[test]
    #[cfg(feature = "serde")]
    fn extrapolation_method_works() {
        let flat_zero = InterpStyle::Linear
            .build_enum(
                standard_knots(),
                standard_dfs(),
                ExtrapolationPolicy::FlatZero,
            )
            .expect("should build in test");
        let _ = flat_zero.extrapolation();

        let flat_fwd = InterpStyle::LogLinear
            .build_enum(
                standard_knots(),
                standard_dfs(),
                ExtrapolationPolicy::FlatForward,
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
