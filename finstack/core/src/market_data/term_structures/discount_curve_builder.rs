//! Builder for [`DiscountCurve`].

use super::common::{build_interp_input_error, split_points};
use crate::math::interp::{ExtrapolationPolicy, InterpStyle};
use crate::{
    dates::{Date, DayCount},
    types::CurveId,
};

use super::discount_curve::DiscountCurve;

/// Fluent builder for [`DiscountCurve`].
///
/// Typical usage chains `base_date`, `knots`, and `interp` (optional)
/// before calling [`DiscountCurveBuilder::build`].
///
/// # Examples
/// ```rust
/// use finstack_core::market_data::term_structures::DiscountCurve;
/// use finstack_core::math::interp::InterpStyle;
/// use finstack_core::dates::Date;
/// use time::Month;
///
/// let base = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");
/// let curve = DiscountCurve::builder("USD-OIS")
///     .base_date(base)
///     .knots([(0.0, 1.0), (5.0, 0.9)])
///     .interp(InterpStyle::Linear)
///     .build()
///     .expect("DiscountCurve builder should succeed");
/// assert!(curve.df(2.0) < 1.0);
/// ```
pub struct DiscountCurveBuilder {
    pub(crate) id: CurveId,
    pub(crate) base: Date,
    pub(crate) base_is_set: bool,
    pub(crate) day_count: DayCount,
    pub(crate) points: Vec<(f64, f64)>, // (t, df)
    pub(crate) style: InterpStyle,
    pub(crate) extrapolation: ExtrapolationPolicy,
    pub(crate) min_forward_rate: Option<f64>,
    pub(crate) allow_non_monotonic: bool,
    pub(crate) min_forward_tenor: f64,
}

impl DiscountCurveBuilder {
    /// Override the default **base date** (valuation date).
    pub fn base_date(mut self, d: Date) -> Self {
        self.base = d;
        self.base_is_set = true;
        self
    }
    /// Choose the day-count basis for discount time mapping.
    pub fn day_count(mut self, dc: DayCount) -> Self {
        self.day_count = dc;
        self
    }
    /// Supply knot points `(t, df)` where *t* is the year fraction and *df*
    /// the discount factor.
    pub fn knots<I>(mut self, pts: I) -> Self
    where
        I: IntoIterator<Item = (f64, f64)>,
    {
        self.points.extend(pts);
        self
    }
    /// Select interpolation style for this curve.
    pub fn interp(mut self, style: InterpStyle) -> Self {
        self.style = style;
        self
    }

    /// Set the extrapolation policy for out-of-bounds evaluation.
    pub fn extrapolation(mut self, policy: ExtrapolationPolicy) -> Self {
        self.extrapolation = policy;
        self
    }

    /// Enforce comprehensive no-arbitrage checks on the discount curve.
    ///
    /// This enables:
    /// - Monotonic (non-increasing) discount factors
    /// - Forward rate floor at -50bp to prevent unrealistic negative rates
    ///
    /// # Example
    /// ```
    /// use finstack_core::market_data::term_structures::DiscountCurve;
    /// use finstack_core::dates::Date;
    /// use time::Month;
    ///
    /// let base = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");
    /// let curve = DiscountCurve::builder("USD-OIS")
    ///     .base_date(base)
    ///     .knots([(0.0, 1.0), (1.0, 0.95), (5.0, 0.80)])
    ///     .enforce_no_arbitrage()
    ///     .build()
    ///     .expect("DiscountCurve builder should succeed");
    /// ```
    pub fn enforce_no_arbitrage(mut self) -> Self {
        self.min_forward_rate = Some(-0.005); // -50bp floor
        self
    }

    /// Set a custom minimum forward rate (in decimal).
    ///
    /// Forward rates below this threshold will trigger a validation error.
    /// This prevents unrealistic negative rate scenarios that could indicate
    /// data errors or create arbitrage opportunities.
    ///
    /// # Example
    /// ```
    /// use finstack_core::market_data::term_structures::DiscountCurve;
    /// use finstack_core::dates::Date;
    /// use time::Month;
    ///
    /// let base = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");
    /// let curve = DiscountCurve::builder("USD-OIS")
    ///     .base_date(base)
    ///     .knots([(0.0, 1.0), (1.0, 0.98), (5.0, 0.85)])
    ///     .min_forward_rate(-0.01)  // Floor at -100bp
    ///     .build()
    ///     .expect("DiscountCurve builder should succeed");
    /// ```
    pub fn min_forward_rate(mut self, min_rate: f64) -> Self {
        self.min_forward_rate = Some(min_rate);
        self
    }

    /// Allow non-monotonic discount factors (use with extreme caution).
    ///
    /// This disables the default monotonicity validation and should only be used
    /// in exceptional circumstances where you need to work with malformed market data.
    ///
    /// **Warning:** Non-monotonic discount factors create arbitrage opportunities
    /// and will produce incorrect pricing results. Only use this override if you
    /// understand the implications.
    ///
    /// For negative rate environments, prefer [`allow_non_monotonic_with_floor`](Self::allow_non_monotonic_with_floor)
    /// which adds a -5% safety floor on implied forward rates.
    pub fn allow_non_monotonic(mut self) -> Self {
        self.allow_non_monotonic = true;
        self
    }

    /// Allow non-monotonic discount factors with a safety floor on forward rates.
    ///
    /// This is the recommended way to handle negative rate environments.
    /// Disables monotonicity validation but sets a -5% floor on implied forward
    /// rates to catch data errors.
    ///
    /// The -5% floor is a conservative bound that accommodates historical negative
    /// rate regimes (e.g., ECB deposit facility at -0.50%) while catching obviously
    /// erroneous data.
    ///
    /// For full override without any floor, use [`allow_non_monotonic`](Self::allow_non_monotonic)
    /// or chain with `.min_forward_rate(f64::NEG_INFINITY)`.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use finstack_core::market_data::term_structures::DiscountCurve;
    /// # use time::macros::date;
    /// # fn main() -> finstack_core::Result<()> {
    /// let curve = DiscountCurve::builder("EUR-OIS")
    ///     .base_date(date!(2025-01-01))
    ///     .knots([(0.0, 1.0), (1.0, 1.002), (5.0, 0.99)])
    ///     .allow_non_monotonic_with_floor()
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn allow_non_monotonic_with_floor(mut self) -> Self {
        self.allow_non_monotonic = true;
        if self.min_forward_rate.is_none() {
            self.min_forward_rate = Some(-0.05);
        }
        self
    }

    /// Set a custom minimum tenor for forward rate calculations.
    ///
    /// The forward rate calculation `f(t1, t2) = (z2*t2 - z1*t1) / (t2 - t1)` suffers
    /// from catastrophic cancellation when `(t2 - t1)` is very small. This threshold
    /// prevents such precision issues.
    ///
    /// # Default
    ///
    /// The default value is [`DEFAULT_MIN_FORWARD_TENOR`](crate::market_data::term_structures::DEFAULT_MIN_FORWARD_TENOR)
    /// (~30 seconds or 1e-6 years).
    ///
    /// # Use Cases
    ///
    /// - Set to a smaller value (e.g., `1e-8`) for high-frequency intraday operations
    /// - Set to a larger value (e.g., `1e-4`) for daily curve operations with coarse data
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use finstack_core::market_data::term_structures::DiscountCurve;
    /// # use time::macros::date;
    /// # fn main() -> finstack_core::Result<()> {
    /// let curve = DiscountCurve::builder("USD")
    ///     .base_date(date!(2025-01-01))
    ///     .knots([(0.0, 1.0), (1.0, 0.95)])
    ///     .min_forward_tenor(1e-8)  // Allow sub-second tenors
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn min_forward_tenor(mut self, tenor: f64) -> Self {
        self.min_forward_tenor = tenor;
        self
    }

    pub(crate) fn apply_non_monotonic_settings(
        mut self,
        allow_non_monotonic: bool,
        min_forward_rate: Option<f64>,
    ) -> Self {
        self.allow_non_monotonic = allow_non_monotonic;
        self.min_forward_rate = min_forward_rate;
        self
    }

    /// Build the curve with minimal validation for solver use.
    ///
    /// This method skips monotonicity validation and forward rate checks, providing
    /// faster curve construction for iterative solving where the curve is temporary.
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    /// - At least 2 knot points are provided
    /// - All discount factors are positive
    /// - Knots are sorted in ascending order
    ///
    /// This is an internal optimization for calibration solvers.
    /// For general use, prefer [`Self::build`] which includes full validation.
    #[doc(hidden)]
    pub fn build_for_solver(self) -> crate::Result<DiscountCurve> {
        if self.points.len() < 2 {
            return Err(crate::error::InputError::TooFewPoints.into());
        }

        if self.points.iter().any(|&(_, df)| df <= 0.0) {
            return Err(crate::error::InputError::NonPositiveValue.into());
        }

        let (knots_vec, dfs_vec): (Vec<f64>, Vec<f64>) = split_points(self.points);

        let knots = knots_vec.into_boxed_slice();
        let dfs = dfs_vec.into_boxed_slice();

        let interp = build_interp_input_error(
            self.style,
            knots.clone(),
            dfs.clone(),
            self.extrapolation,
            true,
        )?;

        Ok(DiscountCurve {
            id: self.id,
            base: self.base,
            day_count: self.day_count,
            knots,
            dfs,
            interp,
            style: self.style,
            extrapolation: self.extrapolation,
            min_forward_rate: self.min_forward_rate,
            allow_non_monotonic: self.allow_non_monotonic,
            min_forward_tenor: self.min_forward_tenor,
        })
    }

    /// Validate input and create the [`DiscountCurve`].
    ///
    /// If the first knot time is `> 0.0`, automatically prepends `(0.0, 1.0)` to
    /// ensure the round-trip invariant `DF(0) = 1.0` (ISDA/QuantLib standard).
    pub fn build(mut self) -> crate::Result<DiscountCurve> {
        if !self.base_is_set {
            return Err(crate::error::InputError::Invalid.into());
        }
        if !self.points.is_empty() {
            self.points.sort_by(|a, b| a.0.total_cmp(&b.0));
            let first_t = self.points[0].0;
            if first_t > 1e-14 {
                self.points.insert(0, (0.0, 1.0));
            }
        }

        if self.points.len() < 2 {
            return Err(crate::error::InputError::TooFewPoints.into());
        }
        if self.points.iter().any(|&(_, df)| df <= 0.0) {
            return Err(crate::error::InputError::NonPositiveValue.into());
        }

        let (knots_vec, dfs_vec): (Vec<f64>, Vec<f64>) = split_points(self.points);
        crate::math::interp::utils::validate_knots(&knots_vec)?;

        if !self.allow_non_monotonic {
            validate_monotonic_df(&knots_vec, &dfs_vec)?;
        } else if self.style == InterpStyle::MonotoneConvex {
            validate_monotone_convex_compatible_df(&knots_vec, &dfs_vec)?;
        }

        if let Some(min_fwd) = self.min_forward_rate {
            validate_forward_rates(&knots_vec, &dfs_vec, min_fwd)?;
        }

        let knots = knots_vec.into_boxed_slice();
        let dfs = dfs_vec.into_boxed_slice();

        let interp = build_interp_input_error(
            self.style,
            knots.clone(),
            dfs.clone(),
            self.extrapolation,
            true,
        )?;

        Ok(DiscountCurve {
            id: self.id,
            base: self.base,
            day_count: self.day_count,
            knots,
            dfs,
            interp,
            style: self.style,
            extrapolation: self.extrapolation,
            min_forward_rate: self.min_forward_rate,
            allow_non_monotonic: self.allow_non_monotonic,
            min_forward_tenor: self.min_forward_tenor,
        })
    }
}

// ---------------------------------------------------------------------------
// Validation helper functions
// ---------------------------------------------------------------------------

/// Validate that discount factors are monotone (non-increasing) within tolerance.
///
/// Non-monotonic discount factors violate no-arbitrage conditions and will
/// produce incorrect pricing results.
fn validate_monotonic_df(knots: &[f64], dfs: &[f64]) -> crate::Result<()> {
    if let Some((i, prev, curr)) = crate::math::interp::utils::find_monotone_violation(dfs, 1e-14) {
        return Err(crate::Error::Validation(format!(
            "Discount factors must be non-increasing: DF(t={:.4}) = {:.12} > DF(t={:.4}) = {:.12}",
            knots[i + 1],
            curr,
            knots[i],
            prev
        )));
    }
    Ok(())
}

/// Validate DF input compatibility with MonotoneConvex interpolation.
///
/// MonotoneConvex (Hagan-West) requires a positive, non-increasing DF term structure.
fn validate_monotone_convex_compatible_df(knots: &[f64], dfs: &[f64]) -> crate::Result<()> {
    if let Some((i, prev, curr)) = crate::math::interp::utils::find_monotone_violation(dfs, 1e-14) {
        return Err(crate::Error::Validation(format!(
            "InterpStyle::MonotoneConvex requires non-increasing discount factors. \
             Found DF(t={:.4}) = {:.12} > DF(t={:.4}) = {:.12}. \
             Use LogLinear/Linear (and allow_non_monotonic) for negative-rate / increasing-DF inputs, \
             or fix the input curve.",
            knots[i + 1],
            curr,
            knots[i],
            prev
        )));
    }
    Ok(())
}

/// Validate that implied forward rates are above a minimum threshold.
///
/// Forward rates are calculated as: f(t1, t2) = -ln(DF(t2)/DF(t1)) / (t2 - t1)
///
/// Excessively negative forward rates (below the specified floor) indicate
/// either data errors or unrealistic market conditions.
fn validate_forward_rates(knots: &[f64], dfs: &[f64], min_rate: f64) -> crate::Result<()> {
    for (knot_pair, df_pair) in knots.windows(2).zip(dfs.windows(2)) {
        let dt = knot_pair[1] - knot_pair[0];
        if dt <= 0.0 {
            continue;
        }

        let fwd = -(df_pair[1] / df_pair[0]).ln() / dt;

        if fwd < min_rate {
            return Err(crate::Error::Validation(format!(
                "Forward rate {:.4}% (decimal: {:.6}) between t={:.4} and t={:.4} is below minimum {:.4}% (decimal: {:.6}). \
                 This may indicate a data error or create arbitrage opportunities.",
                fwd * 100.0, fwd, knot_pair[0], knot_pair[1], min_rate * 100.0, min_rate
            )));
        }
    }
    Ok(())
}
