//! Forward-rate curve for a fixed-tenor index (e.g. 3-month SOFR).
//!
//! Stores simple forward rates at knot times and interpolates them with a
//! chosen [`crate::math::interp::InterpStyle`].  Implements
//! [`crate::market_data::traits::Forward`] which provides helper methods such
//! as [`crate::market_data::traits::Forward::rate_period`].
//!
//! ## Example
//! ```rust
//! use finstack_core::market_data::term_structures::forward_curve::ForwardCurve;
//! use finstack_core::math::interp::InterpStyle;
//! use finstack_core::dates::Date;
//! use time::Month;
//!
//! let base = Date::from_calendar_date(2025, Month::January, 1).unwrap();
//! let fc = ForwardCurve::builder("USD-SOFR3M", 0.25)
//!     .base_date(base)
//!     .knots([(0.0, 0.03), (5.0, 0.04)])
//!     .set_interp(InterpStyle::Linear)
//!     .build()
//!     .unwrap();
//! assert!(fc.rate(1.0) > 0.0);
//! ```

use super::common::{build_interp, split_points};
use crate::math::interp::{ExtrapolationPolicy, InterpStyle};
use crate::{
    dates::{Date, DayCount},
    error::InputError,
    market_data::traits::{Forward, TermStructure},
    math::interp::types::Interp,
    types::CurveId,
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
    tenor: f64,
    /// Knot times in **years** (strictly increasing, first may be 0.0).
    knots: Box<[f64]>,
    /// Simple forward rates (e.g. 0.025 = 2.5 %).
    forwards: Box<[f64]>,
    interp: Interp,
}

/// Serializable state of ForwardCurve
#[cfg(feature = "serde")]
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ForwardCurveState {
    #[cfg_attr(feature = "serde", serde(flatten))]
    common_id: super::common::StateId,
    /// Base date
    pub base: Date,
    /// Reset lag in calendar days
    pub reset_lag: i32,
    /// Day count convention
    pub day_count: DayCount,
    /// Index tenor in years
    pub tenor: f64,
    #[cfg_attr(feature = "serde", serde(flatten))]
    points: super::common::StateKnotPoints,
    #[cfg_attr(feature = "serde", serde(flatten))]
    interp: super::common::StateInterp,
}

impl ForwardCurve {
    /// Start building a forward curve for `id` with tenor `tenor_years`.
    pub fn builder(id: impl Into<CurveId>, tenor_years: f64) -> ForwardCurveBuilder {
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
    pub fn rate(&self, t: f64) -> f64 {
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
    pub fn tenor(&self) -> f64 {
        self.tenor
    }

    /// Raw knot times used to bootstrap the curve.
    #[inline]
    pub fn knots(&self) -> &[f64] {
        &self.knots
    }

    /// Raw simple forward rates at each knot.
    #[inline]
    pub fn forwards(&self) -> &[f64] {
        &self.forwards
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

    /// Average rate over `[t1, t2]`.
    #[inline]
    pub fn rate_period(&self, t1: f64, t2: f64) -> f64 {
        debug_assert!(t2 > t1, "t2 must be after t1");
        (self.rate(t1) + self.rate(t2)) * 0.5
    }

    #[cfg(feature = "serde")]
    /// Extract serializable state
    pub fn to_state(&self) -> ForwardCurveState {
        let knot_points: Vec<(f64, f64)> = self
            .knots
            .iter()
            .zip(self.forwards.iter())
            .map(|(&t, &fwd)| (t, fwd))
            .collect();

        ForwardCurveState {
            common_id: super::common::StateId {
                id: self.id.to_string(),
            },
            base: self.base,
            reset_lag: self.reset_lag,
            day_count: self.day_count,
            tenor: self.tenor,
            points: super::common::StateKnotPoints { knot_points },
            interp: super::common::StateInterp {
                interp_style: self.interp.style(),
                extrapolation: self.interp.extrapolation(),
            },
        }
    }

    #[cfg(feature = "serde")]
    /// Create from serialized state
    pub fn from_state(state: ForwardCurveState) -> crate::Result<Self> {
        ForwardCurve::builder(state.common_id.id, state.tenor)
            .base_date(state.base)
            .reset_lag(state.reset_lag)
            .day_count(state.day_count)
            .knots(state.points.knot_points)
            .set_interp(state.interp.interp_style)
            .build()
    }
}

/// Fluent builder for [`ForwardCurve`].
pub struct ForwardCurveBuilder {
    id: CurveId,
    base: Date,
    reset_lag: i32,
    day_count: DayCount,
    tenor: f64,
    points: Vec<(f64, f64)>,
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
        I: IntoIterator<Item = (f64, f64)>,
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
    pub fn build(self) -> crate::Result<ForwardCurve> {
        if self.points.len() < 2 {
            return Err(InputError::TooFewPoints.into());
        }
        let (kvec, fvec): (Vec<f64>, Vec<f64>) = split_points(self.points);
        crate::math::interp::utils::validate_knots(&kvec)?;
        let knots = kvec.into_boxed_slice();
        let forwards = fvec.into_boxed_slice();
        let interp = build_interp(
            self.style,
            knots.clone(),
            forwards.clone(),
            ExtrapolationPolicy::default(),
        )?;
        Ok(ForwardCurve {
            id: self.id,
            base: self.base,
            reset_lag: self.reset_lag,
            day_count: self.day_count,
            tenor: self.tenor,
            knots,
            forwards,
            interp,
        })
    }
}

// -----------------------------------------------------------------------------
// Minimal trait implementations for polymorphism where needed
// -----------------------------------------------------------------------------

impl Forward for ForwardCurve {
    #[inline]
    fn rate(&self, t: f64) -> f64 {
        self.rate(t)
    }
}

impl TermStructure for ForwardCurve {
    #[inline]
    fn id(&self) -> &CurveId {
        &self.id
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
        self.to_state().serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for ForwardCurve {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let state = ForwardCurveState::deserialize(deserializer)?;
        ForwardCurve::from_state(state).map_err(serde::de::Error::custom)
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    fn sample_forward() -> ForwardCurve {
        ForwardCurve::builder("USD-LIB3M", 0.25)
            .knots([(0.0, 0.03), (1.0, 0.04)])
            .build()
            .unwrap()
    }

    #[test]
    fn interpolates_rate() {
        let fc = sample_forward();
        assert!((fc.rate(0.5) - 0.035).abs() < 1e-12);
    }
}
