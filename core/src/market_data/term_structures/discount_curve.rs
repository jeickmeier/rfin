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
//! use rfin_core::market_data::term_structures::discount_curve::DiscountCurve;
//! use rfin_core::dates::Date;
//! use time::Month;
//!
//! let curve = DiscountCurve::builder("USD-OIS")
//!     .base_date(Date::from_calendar_date(2025, Month::January, 1).unwrap())
//!     .knots([(0.0, 1.0), (5.0, 0.9)])
//!     .monotone_convex()
//!     .build()
//!     .unwrap();
//! assert!(curve.df(3.0) < 1.0);
//! ```

extern crate alloc;

use crate::market_data::interp::InterpStyle;
use crate::{
    dates::Date,
    market_data::id::CurveId,
    market_data::interp::InterpFn,
    market_data::traits::{Discount, TermStructure},
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
    interp: Box<dyn InterpFn>,
}

impl DiscountCurve {
    /// Discount factor at time `t` (helper calling the underlying interpolator).
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
        }
    }
}

/// Fluent builder for [`DiscountCurve`].
pub struct DiscountCurveBuilder {
    id: &'static str,
    base: Date,
    points: Vec<(F, F)>, // (t, df)
    style: InterpStyle,
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
    // Builder helpers to choose interpolation style
    /// Use **linear** DF interpolation.
    pub fn linear_df(mut self) -> Self {
        self.style = InterpStyle::Linear;
        self
    }
    /// Use **log‐linear** DF interpolation (constant zero rate).
    pub fn log_df(mut self) -> Self {
        self.style = InterpStyle::LogLinear;
        self
    }
    /// Use **monotone‐convex** interpolation.
    pub fn monotone_convex(mut self) -> Self {
        self.style = InterpStyle::MonotoneConvex;
        self
    }
    /// Use **cubic‐Hermite** (PCHIP) interpolation.
    pub fn cubic_hermite(mut self) -> Self {
        self.style = InterpStyle::CubicHermite;
        self
    }
    /// Use **flat‐forward** interpolation.
    pub fn flat_fwd(mut self) -> Self {
        self.style = InterpStyle::FlatFwd;
        self
    }

    /// Validate input and create the [`DiscountCurve`].
    #[allow(unused_mut)]
    pub fn build(mut self) -> core::result::Result<DiscountCurve, super::CurveError> {
        if self.points.len() < 2 {
            return Err(super::CurveError::TooFewPoints);
        }
        if self.points.iter().any(|&(_, df)| df <= 0.0) {
            return Err(super::CurveError::NonPositiveValue);
        }

        let (knots_vec, dfs_vec): (Vec<F>, Vec<F>) = self.points.into_iter().unzip();
        crate::market_data::utils::validate_knots(&knots_vec)
            .map_err(|_| super::CurveError::NonMonotonicKnots)?;
        let knots = knots_vec.into_boxed_slice();
        let dfs = dfs_vec.into_boxed_slice();

        let interp = self
            .style
            .make_interp(knots.clone(), dfs.clone())
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

    fn df(&self, t: F) -> F {
        self.interp.interp(t)
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
            .linear_df()
            .build()
            .unwrap()
    }

    fn sample_curve_log() -> DiscountCurve {
        DiscountCurve::builder("USD-OIS")
            .base_date(Date::from_calendar_date(2025, time::Month::June, 30).unwrap())
            .knots([(0.0, 1.0), (1.0, 0.98), (2.0, 0.95)])
            .log_df()
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
