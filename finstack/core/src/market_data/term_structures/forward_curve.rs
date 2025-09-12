//! Forward-rate curve for a fixed-tenor index (e.g. 3-month SOFR).
//!
//! Stores simple forward rates at knot times and interpolates them with a
//! chosen [`crate::market_data::interp::InterpStyle`].  Implements
//! [`crate::market_data::traits::Forward`] which provides helper methods such
//! as [`crate::market_data::traits::Forward::rate_period`].
//!
//! ## Example
//! ```rust
//! use finstack_core::market_data::term_structures::forward_curve::ForwardCurve;
//! // 3-month tenor expressed in years
//! # use finstack_core::market_data::interp::InterpStyle;
//! let fc = ForwardCurve::builder("USD-SOFR3M", 0.25)
//!     .knots([(0.0, 0.03), (5.0, 0.04)])
//!     .set_interp(InterpStyle::Linear)
//!     .build()
//!     .unwrap();
//! assert!(fc.rate(1.0) > 0.0);
//! ```

extern crate alloc;
use alloc::{boxed::Box, vec::Vec};

use crate::market_data::interp::{ExtrapolationPolicy, InterpStyle};
use super::common::{build_interp, split_points, OneDGrid};
use crate::{
    dates::{Date, DayCount},
    error::InputError,
    market_data::interp::types::Interp,
    market_data::traits::{Forward, TermStructure},
    types::CurveId,
    F,
};

/// Forward-rate curve for an index with fixed tenor (e.g. 3-month SOFR).
#[derive(Debug)]
pub struct ForwardCurve {
    id: CurveId,
    base: Date,
    /// Calendar days from fixing to spot.
    reset_lag: i32,
    /// Day-count basis used for accrual.
    day_count: DayCount,
    /// Index tenor in **years** (0.25 = 3M).
    tenor: F,
    /// Knot times in **years** (strictly increasing, first may be 0.0).
    knots: Box<[F]>,
    /// Simple forward rates (e.g. 0.025 = 2.5 %).
    fwds: Box<[F]>,
    interp: Interp,
}

/// Serializable representation of ForwardCurve
#[cfg(feature = "serde")]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct ForwardCurveData {
    id: CurveId,
    base: Date,
    reset_lag: i32,
    day_count: DayCount,
    tenor: F,
    knots: Vec<F>,
    fwds: Vec<F>,
    interp_style: InterpStyle,
    extrapolation: ExtrapolationPolicy,
}

impl ForwardCurve {
    /// Start building a forward curve for `id` with tenor `tenor_years`.
    pub fn builder(id: impl Into<CurveId>, tenor_years: F) -> ForwardCurveBuilder {
        ForwardCurveBuilder {
            id: id.into(),
            base: Date::from_calendar_date(1970, time::Month::January, 1).unwrap(),
            reset_lag: 2,
            day_count: DayCount::Act360,
            tenor: tenor_years,
            points: Vec::new(),
            style: InterpStyle::Linear,
        }
    }

    /// Forward rate starting at time `t` (in years) for the curve’s tenor.
    #[inline]
    pub fn rate(&self, t: F) -> F {
        self.interp.interp(t)
    }

    /// Reset lag in calendar days from fixing to spot.
    #[inline]
    pub fn reset_lag(&self) -> i32 {
        self.reset_lag
    }

    /// Day-count convention used for this index.
    #[inline]
    pub fn day_count(&self) -> DayCount {
        self.day_count
    }

    /// Index tenor in **years** (e.g. 0.25 = 3M).
    #[inline]
    pub fn tenor(&self) -> F {
        self.tenor
    }

    /// Raw knot times used to bootstrap the curve.
    #[inline]
    pub fn knots(&self) -> &[F] {
        &self.knots
    }

    /// Raw simple forward rates at each knot.
    #[inline]
    pub fn fwds(&self) -> &[F] {
        &self.fwds
    }

    /// Curve identifier.
    #[inline]
    pub fn id(&self) -> &CurveId {
        &self.id
    }
    /// Valuation **base date**.
    #[inline]
    pub fn base_date(&self) -> Date {
        self.base
    }
}

/// Fluent builder for [`ForwardCurve`].
pub struct ForwardCurveBuilder {
    id: CurveId,
    base: Date,
    reset_lag: i32,
    day_count: DayCount,
    tenor: F,
    points: Vec<(F, F)>,
    style: InterpStyle,
}

impl ForwardCurveBuilder {
    /// Set the curve’s valuation **base date**.
    pub fn base_date(mut self, d: Date) -> Self {
        self.base = d;
        self
    }
    /// Override the **reset lag** (fixing → spot) in calendar days.
    pub fn reset_lag(mut self, lag: i32) -> Self {
        self.reset_lag = lag;
        self
    }
    /// Choose the **day-count** convention.
    pub fn day_count(mut self, dc: DayCount) -> Self {
        self.day_count = dc;
        self
    }
    /// Supply knot points `(t, fwd)`.
    pub fn knots<I>(mut self, pts: I) -> Self
    where
        I: IntoIterator<Item = (F, F)>,
    {
        self.points.extend(pts);
        self
    }
    /// Select interpolation style for this forward curve.
    pub fn set_interp(mut self, style: InterpStyle) -> Self {
        self.style = style;
        self
    }

    /// Validate input and build the [`ForwardCurve`].
    #[must_use]
    pub fn build(self) -> crate::Result<ForwardCurve> {
        if self.points.len() < 2 {
            return Err(InputError::TooFewPoints.into());
        }
        let (kvec, fvec): (Vec<F>, Vec<F>) = split_points(self.points);
        crate::math::interp::utils::validate_knots(&kvec)?;
        let knots = kvec.into_boxed_slice();
        let fwds = fvec.into_boxed_slice();
        let grid = OneDGrid::new(knots.clone(), fwds.clone());
        let interp = build_interp(self.style, &grid, ExtrapolationPolicy::default())?;
        Ok(ForwardCurve {
            id: self.id,
            base: self.base,
            reset_lag: self.reset_lag,
            day_count: self.day_count,
            tenor: self.tenor,
            knots,
            fwds,
            interp,
        })
    }
}

// Interpolator helpers moved to InterpStyle – factory fns removed.

// -----------------------------------------------------------------------------
// Trait impls – new generic family
// -----------------------------------------------------------------------------

impl TermStructure for ForwardCurve {
    fn id(&self) -> &CurveId {
        &self.id
    }
}

impl Forward for ForwardCurve {
    #[inline]
    fn rate(&self, t: F) -> F {
        ForwardCurve::rate(self, t)
    }
}

// -----------------------------------------------------------------------------
// Serialization support
// -----------------------------------------------------------------------------

#[cfg(feature = "serde")]
impl serde::Serialize for ForwardCurve {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let data = ForwardCurveData {
            id: self.id.clone(),
            base: self.base,
            reset_lag: self.reset_lag,
            day_count: self.day_count,
            tenor: self.tenor,
            knots: self.knots.to_vec(),
            fwds: self.fwds.to_vec(),
            interp_style: self.interp.style(),
            extrapolation: self.interp.extrapolation(),
        };
        data.serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for ForwardCurve {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;

        let data = ForwardCurveData::deserialize(deserializer)?;
        let grid = OneDGrid::new(
            data.knots.clone().into_boxed_slice(),
            data.fwds.clone().into_boxed_slice(),
        );
        let interp = build_interp(data.interp_style, &grid, data.extrapolation)
            .map_err(|_| D::Error::custom("Failed to reconstruct interpolator"))?;

        Ok(ForwardCurve {
            id: data.id,
            base: data.base,
            reset_lag: data.reset_lag,
            day_count: data.day_count,
            tenor: data.tenor,
            knots: data.knots.into_boxed_slice(),
            fwds: data.fwds.into_boxed_slice(),
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

    fn sample_fwd() -> ForwardCurve {
        ForwardCurve::builder("USD-LIB3M", 0.25)
            .knots([(0.0, 0.03), (1.0, 0.04)])
            .build()
            .unwrap()
    }

    #[test]
    fn interpolates_rate() {
        let fc = sample_fwd();
        assert!((fc.rate(0.5) - 0.035).abs() < 1e-12);
    }
}
