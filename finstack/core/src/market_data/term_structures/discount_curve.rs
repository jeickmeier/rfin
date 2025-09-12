//! Piece-wise discount-factor curve with pluggable interpolation.
//!
//! A `DiscountCurve` stores discount factors at user-defined knot times (year
//! fractions) and interpolates between them using any
//! [`crate::market_data::interp::InterpStyle`].  The curve implements
//! [`crate::market_data::traits::Discount`] so downstream pricing code can
//! consume it polymorphically.
//!
//! ## Example
//! ```rust
//! use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
//! use finstack_core::dates::Date;
//! use time::Month;
//! # use finstack_core::market_data::interp::InterpStyle;
//!
//! let curve = DiscountCurve::builder("USD-OIS")
//!     .base_date(Date::from_calendar_date(2025, Month::January, 1).unwrap())
//!     .knots([(0.0, 1.0), (5.0, 0.9)])
//!     .set_interp(InterpStyle::MonotoneConvex)
//!     .build()
//!     .unwrap();
//! assert!(curve.df(3.0) < 1.0);
//! ```

extern crate alloc;

use crate::market_data::interp::{ExtrapolationPolicy, InterpStyle};
use crate::{
    dates::Date,
    market_data::interp::types::Interp,
    market_data::traits::{Discount, TermStructure},
    types::CurveId,
    F,
};
use alloc::{boxed::Box, vec::Vec};

/// Piece-wise discount factor curve supporting several interpolation styles.
#[derive(Debug)]
pub struct DiscountCurve {
    id: CurveId,
    base: Date,
    /// Knot times in **years**.
    knots: Box<[F]>,
    /// Discount factors (unitless).
    dfs: Box<[F]>,
    interp: Interp,
}

/// Serializable representation of DiscountCurve
#[cfg(feature = "serde")]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct DiscountCurveData {
    id: CurveId,
    base: Date,
    knots: Vec<F>,
    dfs: Vec<F>,
    interp_data: crate::market_data::interp::types::InterpData,
}

impl DiscountCurve {
    /// Convenience: discount factor on a specific date `date` given a curve and
    /// the curve base `base` and `day_count`.
    /// This is equivalent to `disc.df(t)` where `t` is the year fraction from `base` to `date`.
    #[inline]
    pub fn df_on(disc: &dyn Discount, base: Date, date: Date, dc: crate::dates::DayCount) -> F {
        let t = if date == base {
            0.0
        } else {
            dc.year_fraction(base, date, crate::dates::DayCountCtx::default())
                .unwrap_or(0.0)
        };
        disc.df(t)
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
    pub fn builder(id: &'static str) -> DiscountCurveBuilder {
        DiscountCurveBuilder {
            id,
            base: Date::from_calendar_date(1970, time::Month::January, 1).unwrap(),
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
        forward_id: &'static str,
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
        forward_id: &'static str,
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
}

/// Fluent builder for [`DiscountCurve`].
pub struct DiscountCurveBuilder {
    id: &'static str,
    base: Date,
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

        let (knots_vec, dfs_vec): (Vec<F>, Vec<F>) = self.points.into_iter().unzip();
        crate::math::interp::utils::validate_knots(&knots_vec)
            .map_err(|_| super::CurveError::NonMonotonicKnots)?;

        // Validate monotonic discount factors if required (critical for credit curves)
        if self.require_monotonic {
            crate::math::interp::utils::validate_monotone_nonincreasing(&dfs_vec)
                .map_err(|_| super::CurveError::Invalid)?;
        }

        let knots = knots_vec.into_boxed_slice();
        let dfs = dfs_vec.into_boxed_slice();

        let interp = self
            .style
            .build_enum(knots.clone(), dfs.clone(), self.extrapolation)
            .map_err(|_| super::CurveError::NonPositiveValue)?;

        Ok(DiscountCurve {
            id: CurveId::new(self.id),
            base: self.base,
            knots,
            dfs,
            interp,
        })
    }
}

// Interpolator helpers now centralised in InterpStyle – local factory fns removed.

// -----------------------------------------------------------------------------
// Trait impls – new generic trait family
// -----------------------------------------------------------------------------

impl TermStructure for DiscountCurve {
    #[inline]
    fn id(&self) -> &CurveId {
        &self.id
    }
}

impl Discount for DiscountCurve {
    fn base_date(&self) -> Date {
        self.base
    }

    #[inline]
    fn df(&self, t: F) -> F {
        self.interp.interp(t)
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
        let data = DiscountCurveData {
            id: self.id.clone(),
            base: self.base,
            knots: self.knots.to_vec(),
            dfs: self.dfs.to_vec(),
            interp_data: self.interp.to_interp_data(),
        };
        data.serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for DiscountCurve {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;
        
        let data = DiscountCurveData::deserialize(deserializer)?;
        let interp = Interp::from_interp_data(data.interp_data)
            .map_err(|e| D::Error::custom(format!("Failed to reconstruct interpolator: {}", e)))?;
        
        Ok(DiscountCurve {
            id: data.id,
            base: data.base,
            knots: data.knots.into_boxed_slice(),
            dfs: data.dfs.into_boxed_slice(),
            interp,
        })
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
