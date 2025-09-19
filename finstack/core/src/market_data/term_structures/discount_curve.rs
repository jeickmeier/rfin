//! Piece-wise discount-factor curve with pluggable interpolation.
//!
//! A `DiscountCurve` stores discount factors at user-defined knot times (year
//! fractions) and interpolates between them using any
//! [`crate::math::interp::InterpStyle`].  The curve implements
//! [`crate::market_data::traits::Discount`] so downstream pricing code can
//! consume it polymorphically.
//!
//! ## Example
//! ```rust
//! use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
//! use finstack_core::dates::Date;
//! use time::Month;
//! # use finstack_core::math::interp::InterpStyle;
//!
//! let curve = DiscountCurve::builder("USD-OIS")
//!     .base_date(Date::from_calendar_date(2025, Month::January, 1).unwrap())
//!     .knots([(0.0, 1.0), (5.0, 0.9)])
//!     .set_interp(InterpStyle::MonotoneConvex)
//!     .build()
//!     .unwrap();
//! assert!(curve.df(3.0) < 1.0);
//! ```

use super::common::{build_interp_curve_error, split_points, OneDGrid};
use crate::math::interp::{ExtrapolationPolicy, InterpStyle};
use crate::{
    dates::{Date, DayCount, DayCountCtx},
    market_data::traits::{Discounting, TermStructure},
    math::interp::types::Interp,
    types::CurveId,
    F,
};

/// Piece-wise discount factor curve supporting several interpolation styles.
#[derive(Debug)]
pub struct DiscountCurve {
    id: CurveId,
    base: Date,
    /// Day-count basis used to convert dates → time for discounting.
    day_count: DayCount,
    /// Knot times in **years**.
    knots: Box<[F]>,
    /// Discount factors (unitless).
    dfs: Box<[F]>,
    interp: Interp,
    /// Interpolation style (stored for serialization and bumping)
    style: InterpStyle,
    /// Extrapolation policy (stored for serialization and bumping)
    extrapolation: ExtrapolationPolicy,
}

/// Serializable state of DiscountCurve
#[cfg(feature = "serde")]
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DiscountCurveState {
    #[cfg_attr(feature = "serde", serde(flatten))]
    common_id: super::common::StateId,
    /// Base date
    pub base: Date,
    /// Day count convention for discount time basis
    #[cfg_attr(feature = "serde", serde(default = "default_discount_day_count"))]
    pub day_count: DayCount,
    #[cfg_attr(feature = "serde", serde(flatten))]
    points: super::common::StateKnotPoints,
    #[cfg_attr(feature = "serde", serde(flatten))]
    interp: super::common::StateInterp,
    /// Whether monotonic discount factors were required
    pub require_monotonic: bool,
}

#[inline]
fn default_discount_day_count() -> DayCount { DayCount::Act365F }

impl DiscountCurve {
    /// Unique identifier of the curve.
    #[inline]
    pub fn id(&self) -> &CurveId {
        &self.id
    }

    /// Base (valuation) date of the curve.
    #[inline]
    pub fn base_date(&self) -> Date {
        self.base
    }

    /// Day-count basis used for discount time mapping.
    #[inline]
    pub fn day_count(&self) -> DayCount { self.day_count }

    /// Continuously-compounded zero rate.
    #[inline]
    pub fn zero(&self, t: F) -> F {
        if t == 0.0 {
            return 0.0;
        }
        -self.df(t).ln() / t
    }

    /// Simple forward rate between `t1` and `t2`.
    #[inline]
    pub fn forward(&self, t1: F, t2: F) -> F {
        debug_assert!(t2 > t1, "forward requires t2 > t1");
        let z1 = self.zero(t1) * t1;
        let z2 = self.zero(t2) * t2;
        (z1 - z2) / (t2 - t1)
    }

    /// Batch evaluation helper (parallel over `times` slice when compiled
    /// with the `parallel` feature).
    #[cfg_attr(docsrs, doc(cfg(feature = "parallel")))]
    pub fn df_batch(&self, times: &[F]) -> Vec<F> {
        #[cfg(feature = "parallel")]
        {
            use rayon::prelude::*;
            // Parallel iteration is required to be order-stable; results must be bit-identical
            // to the sequential path. We therefore only parallelize the map, preserving order.
            times.par_iter().map(|&t| self.df(t)).collect()
        }
        #[cfg(not(feature = "parallel"))]
        {
            times.iter().map(|&t| self.df(t)).collect()
        }
    }

    /// Convenience: discount factor on a specific date `date` given a curve and
    /// the curve base `base` and `day_count`.
    /// This is equivalent to `disc.df(t)` where `t` is the year fraction from `base` to `date`.
    #[inline]
    pub fn df_on_date(&self, date: Date, dc: crate::dates::DayCount) -> F {
        let t = if date == self.base {
            0.0
        } else {
            dc.year_fraction(self.base, date, DayCountCtx::default())
                .unwrap_or(0.0)
        };
        self.df(t)
    }

    /// Convenience: discount factor on a specific date using the curve's own day-count.
    #[inline]
    pub fn df_on_date_curve(&self, date: Date) -> F {
        let t = if date == self.base { 0.0 } else { self.day_count.year_fraction(self.base, date, DayCountCtx::default()).unwrap_or(0.0) };
        self.df(t)
    }

    /// Static convenience: discount factor on a specific date given any discount curve.
    /// For backward compatibility with existing code.
    #[inline]
    pub fn df_on(disc: &dyn Discounting, base: Date, date: Date, dc: crate::dates::DayCount) -> F {
        let t = if date == base {
            0.0
        } else {
            dc.year_fraction(base, date, DayCountCtx::default())
                .unwrap_or(0.0)
        };
        disc.df(t)
    }

    /// Create a new curve with a parallel rate bump applied in basis points.
    ///
    /// Uses df_bumped(t) = df_original(t) * exp(-bump * t), where bump = bp / 10_000.
    pub fn with_parallel_bump(&self, bp: F) -> Self {
        let bump_rate = bp / 10_000.0;
        let bumped_points: Vec<(F, F)> = self
            .knots
            .iter()
            .zip(self.dfs.iter())
            .map(|(&t, &df)| (t, df * (-bump_rate * t).exp()))
            .collect();

        // Derive new ID with suffix
        let new_id = crate::market_data::bumps::id_bump_bp(self.id.as_str(), bp);

        // Rebuild preserving base date, interpolation, and extrapolation policies
        DiscountCurve::builder(new_id)
            .base_date(self.base)
            .knots(bumped_points)
            .set_interp(self.style)
            .extrapolation(self.extrapolation)
            .build()
            .expect("building bumped discount curve should not fail")
    }
    /// Discount factor at time `t` (helper calling the underlying interpolator).
    #[inline]
    pub fn df(&self, t: F) -> F {
        self.interp.interp(t)
    }

    /// Raw knot times (t) in **years** passed at construction.
    #[inline]
    pub fn knots(&self) -> &[F] {
        &self.knots
    }

    /// Raw discount factors corresponding to each knot.
    #[inline]
    pub fn dfs(&self) -> &[F] {
        &self.dfs
    }

    /// Builder entry-point.
    pub fn builder(id: impl Into<CurveId>) -> DiscountCurveBuilder {
        DiscountCurveBuilder {
            id: id.into(),
            base: Date::from_calendar_date(1970, time::Month::January, 1).unwrap(),
            day_count: DayCount::Act365F,
            points: Vec::new(),
            style: InterpStyle::Linear,
            extrapolation: ExtrapolationPolicy::default(),
            require_monotonic: false,
        }
    }

    /// Create a forward curve from this discount curve.
    ///
    /// For single-curve bootstrapping, this creates a forward curve from the
    /// discount factors using the formula:
    /// f(t) = -d/dt[ln(DF(t))] = -1/DF(t) * dDF/dt
    ///
    /// For discrete points, we use: f(t) ≈ (DF(t) - DF(t+dt)) / (dt * DF(t+dt))
    pub fn to_forward_curve(
        &self,
        forward_id: impl Into<CurveId>,
        tenor_years: F,
    ) -> crate::Result<super::forward_curve::ForwardCurve> {
        use super::forward_curve::ForwardCurve;

        // Calculate forward rates at each knot point
        let mut forward_rates = Vec::with_capacity(self.knots.len());

        // Ensure we have enough points
        if self.knots.len() < 2 {
            return Err(crate::error::InputError::TooFewPoints.into());
        }

        for i in 0..self.knots.len() {
            let t = self.knots[i];
            let forward_rate = if i == 0 {
                // First point: use next point for forward difference
                let t_next = self.knots[1];
                let df = self.dfs[0];
                let df_next = self.dfs[1];
                let dt = t_next - t;

                if dt > 0.0 && df_next > 0.0 && df > 0.0 {
                    (df / df_next - 1.0) / dt
                } else if t > 0.0 && df > 0.0 {
                    // Use spot rate
                    (-df.ln()) / t
                } else {
                    0.045 // Default rate
                }
            } else if i < self.knots.len() - 1 {
                // Interior points: use central difference
                let t_prev = self.knots[i - 1];
                let t_next = self.knots[i + 1];
                let df_prev = self.dfs[i - 1];
                let df_next = self.dfs[i + 1];

                // Use instantaneous forward rate approximation
                let dt = t_next - t_prev;
                if dt > 0.0 && df_next > 0.0 && df_prev > 0.0 {
                    (df_prev.ln() - df_next.ln()) / dt
                } else {
                    0.045 // Default rate
                }
            } else {
                // Last point: use backward difference
                let t_prev = self.knots[i - 1];
                let df = self.dfs[i];
                let df_prev = self.dfs[i - 1];
                let dt = t - t_prev;

                if dt > 0.0 && df > 0.0 && df_prev > 0.0 {
                    (df_prev / df - 1.0) / dt
                } else {
                    0.045 // Default rate
                }
            };

            forward_rates.push((t, forward_rate.clamp(0.0, 0.5))); // Clamp to reasonable range
        }

        // Build forward curve with linear interpolation (more stable)
        ForwardCurve::builder(forward_id, tenor_years)
            .base_date(self.base)
            .knots(forward_rates)
            .set_interp(InterpStyle::Linear)
            .build()
    }

    /// Create a forward curve from this discount curve using a specific interpolation style.
    pub fn to_forward_curve_with_interp(
        &self,
        forward_id: impl Into<CurveId>,
        tenor_years: F,
        interp_style: InterpStyle,
    ) -> crate::Result<super::forward_curve::ForwardCurve> {
        use super::forward_curve::ForwardCurve;

        // Calculate forward rates at each knot point (same as to_forward_curve)
        let mut forward_rates = Vec::with_capacity(self.knots.len());

        if self.knots.len() < 2 {
            return Err(crate::error::InputError::TooFewPoints.into());
        }

        for i in 0..self.knots.len() {
            let t = self.knots[i];
            let forward_rate = if i == 0 {
                let t_next = self.knots[1];
                let df = self.dfs[0];
                let df_next = self.dfs[1];
                let dt = t_next - t;

                if dt > 0.0 && df_next > 0.0 && df > 0.0 {
                    (df / df_next - 1.0) / dt
                } else if t > 0.0 && df > 0.0 {
                    (-df.ln()) / t
                } else {
                    0.045
                }
            } else if i < self.knots.len() - 1 {
                let t_prev = self.knots[i - 1];
                let t_next = self.knots[i + 1];
                let df_prev = self.dfs[i - 1];
                let df_next = self.dfs[i + 1];

                let dt = t_next - t_prev;
                if dt > 0.0 && df_next > 0.0 && df_prev > 0.0 {
                    (df_prev.ln() - df_next.ln()) / dt
                } else {
                    0.045
                }
            } else {
                let t_prev = self.knots[i - 1];
                let df = self.dfs[i];
                let df_prev = self.dfs[i - 1];
                let dt = t - t_prev;

                if dt > 0.0 && df > 0.0 && df_prev > 0.0 {
                    (df_prev / df - 1.0) / dt
                } else {
                    0.045
                }
            };

            forward_rates.push((t, forward_rate.clamp(0.0, 0.5)));
        }

        ForwardCurve::builder(forward_id, tenor_years)
            .base_date(self.base)
            .knots(forward_rates)
            .set_interp(interp_style)
            .build()
    }

    #[cfg(feature = "serde")]
    /// Extract serializable state
    pub fn to_state(&self) -> DiscountCurveState {
        let knot_points: Vec<(F, F)> = self
            .knots
            .iter()
            .zip(self.dfs.iter())
            .map(|(&t, &df)| (t, df))
            .collect();

        DiscountCurveState {
            common_id: super::common::StateId {
                id: self.id.to_string(),
            },
            base: self.base,
            day_count: self.day_count,
            points: super::common::StateKnotPoints { knot_points },
            interp: super::common::StateInterp {
                interp_style: self.style,
                extrapolation: self.extrapolation,
            },
            require_monotonic: false, // Default - we can't recover this info from existing curves
        }
    }

    #[cfg(feature = "serde")]
    /// Create from serialized state
    pub fn from_state(state: DiscountCurveState) -> core::result::Result<Self, super::CurveError> {
        let mut builder = DiscountCurve::builder(state.common_id.id)
            .base_date(state.base)
            .day_count(state.day_count)
            .knots(state.points.knot_points)
            .set_interp(state.interp.interp_style)
            .extrapolation(state.interp.extrapolation);

        if state.require_monotonic {
            builder = builder.require_monotonic();
        }

        builder.build()
    }
}

/// Fluent builder for [`DiscountCurve`].
///
/// Typical usage chains `base_date`, `knots`, and `set_interp` (optional)
/// before calling [`DiscountCurveBuilder::build`]. The builder implements the
/// shared [`super::CurveBuilder`] trait, so it can participate in generic
/// helper code.
///
/// # Examples
/// ```rust
/// use finstack_core::market_data::term_structures::{discount_curve::DiscountCurve, CurveBuilder};
/// use finstack_core::math::interp::InterpStyle;
/// use finstack_core::dates::Date;
/// use time::Month;
///
/// let base = Date::from_calendar_date(2025, Month::January, 1).unwrap();
/// let curve = DiscountCurve::builder("USD-OIS")
///     .base_date(base)
///     .knots([(0.0, 1.0), (5.0, 0.9)])
///     .set_interp(InterpStyle::Linear)
///     .build()
///     .unwrap();
/// assert!(curve.df(2.0) < 1.0);
/// ```
pub struct DiscountCurveBuilder {
    id: CurveId,
    base: Date,
    day_count: DayCount,
    points: Vec<(F, F)>, // (t, df)
    style: InterpStyle,
    extrapolation: ExtrapolationPolicy,
    require_monotonic: bool, // Critical for credit curves
}

impl DiscountCurveBuilder {
    /// Override the default **base date** (valuation date).
    pub fn base_date(mut self, d: Date) -> Self {
        self.base = d;
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
        I: IntoIterator<Item = (F, F)>,
    {
        self.points.extend(pts);
        self
    }
    /// Select interpolation style for this curve.
    pub fn set_interp(mut self, style: InterpStyle) -> Self {
        self.style = style;
        self
    }

    /// Set the extrapolation policy for out-of-bounds evaluation.
    pub fn extrapolation(mut self, policy: ExtrapolationPolicy) -> Self {
        self.extrapolation = policy;
        self
    }

    /// Require monotonic (strictly decreasing) discount factors.
    /// This is critical for credit curves to ensure arbitrage-free pricing.
    pub fn require_monotonic(mut self) -> Self {
        self.require_monotonic = true;
        self
    }

    /// Validate input and create the [`DiscountCurve`].
    pub fn build(self) -> core::result::Result<DiscountCurve, super::CurveError> {
        if self.points.len() < 2 {
            return Err(super::CurveError::TooFewPoints);
        }
        if self.points.iter().any(|&(_, df)| df <= 0.0) {
            return Err(super::CurveError::NonPositiveValue);
        }

        let (knots_vec, dfs_vec): (Vec<F>, Vec<F>) = split_points(self.points);
        crate::math::interp::utils::validate_knots(&knots_vec)
            .map_err(|_| super::CurveError::NonMonotonicKnots)?;

        // Validate monotonic discount factors if required (critical for credit curves)
        if self.require_monotonic {
            crate::math::interp::utils::validate_monotone_nonincreasing(&dfs_vec)
                .map_err(|_| super::CurveError::Invalid)?;
        }

        let knots = knots_vec.into_boxed_slice();
        let dfs = dfs_vec.into_boxed_slice();

        let grid = OneDGrid::new(knots.clone(), dfs.clone());
        let interp = build_interp_curve_error(self.style, &grid, self.extrapolation)?;

        Ok(DiscountCurve {
            id: self.id,
            base: self.base,
            day_count: self.day_count,
            knots,
            dfs,
            interp,
            style: self.style,
            extrapolation: self.extrapolation,
        })
    }
}

// Implement unified builder trait for DiscountCurveBuilder
impl super::common::CurveBuilder for DiscountCurveBuilder {
    type Output = DiscountCurve;

    fn base_date(self, d: Date) -> Self {
        DiscountCurveBuilder::base_date(self, d)
    }

    fn knots<I>(self, pts: I) -> Self
    where
        I: IntoIterator<Item = (F, F)>,
    {
        DiscountCurveBuilder::knots(self, pts)
    }

    fn set_interp(self, style: InterpStyle) -> Self {
        DiscountCurveBuilder::set_interp(self, style)
    }

    fn build(self) -> crate::Result<Self::Output> {
        DiscountCurveBuilder::build(self).map_err(crate::error::Error::from)
    }
}

// Interpolator helpers now centralised in InterpStyle – local factory fns removed.

// -----------------------------------------------------------------------------
// Minimal trait implementation for polymorphism where needed
// -----------------------------------------------------------------------------

impl Discounting for DiscountCurve {
    #[inline]
    fn base_date(&self) -> Date {
        self.base
    }

    #[inline]
    fn df(&self, t: F) -> F {
        self.interp.interp(t)
    }
}

impl TermStructure for DiscountCurve {
    #[inline]
    fn id(&self) -> &CurveId {
        &self.id
    }
}

// -----------------------------------------------------------------------------
// Serialization support
// -----------------------------------------------------------------------------

#[cfg(feature = "serde")]
impl serde::Serialize for DiscountCurve {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_state().serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for DiscountCurve {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let state = DiscountCurveState::deserialize(deserializer)?;
        DiscountCurve::from_state(state).map_err(serde::de::Error::custom)
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    fn sample_curve_linear() -> DiscountCurve {
        DiscountCurve::builder("USD-OIS")
            .base_date(Date::from_calendar_date(2025, time::Month::June, 30).unwrap())
            .knots([(0.0, 1.0), (1.0, 0.98), (2.0, 0.95)])
            .set_interp(InterpStyle::Linear)
            .build()
            .unwrap()
    }

    fn sample_curve_log() -> DiscountCurve {
        DiscountCurve::builder("USD-OIS")
            .base_date(Date::from_calendar_date(2025, time::Month::June, 30).unwrap())
            .knots([(0.0, 1.0), (1.0, 0.98), (2.0, 0.95)])
            .set_interp(InterpStyle::LogLinear)
            .build()
            .unwrap()
    }

    #[test]
    fn hits_knots_exactly() {
        let yc = sample_curve_linear();
        for (t, df) in [(0.0, 1.0), (1.0, 0.98), (2.0, 0.95)] {
            assert!((yc.df(t) - df).abs() < 1e-12);
        }
    }

    #[test]
    fn rejects_unsorted_knots() {
        let res = DiscountCurve::builder("USD")
            .knots([(1.0, 0.99), (0.5, 0.995)])
            .build();
        assert!(res.is_err());
    }

    #[test]
    fn logdf_interpolator_behaves() {
        let yc = sample_curve_log();
        let mid = yc.df(0.5);
        assert!(mid < 1.0 && mid > 0.98);
    }
}
