//! Real / breakeven consumer-price index (CPI) curve expressed as index levels.
//!
//! Provides interpolated CPI values and derived annualised inflation rates via
//! [`crate::market_data::traits::Inflation`].  Accepts any interpolation style
//! supported by the [`crate::market_data::interp`] subsystem although
//! `LogLinear` is the most common choice for exponential CPI growth.
//!
//! ## Example
//! ```rust
//! use finstack_core::market_data::term_structures::inflation::InflationCurve;
//! # use finstack_core::market_data::interp::InterpStyle;
//! let ic = InflationCurve::builder("US-CPI")
//!     .base_cpi(300.0)
//!     .knots([(0.0, 300.0), (5.0, 327.0)])
//!     .set_interp(InterpStyle::LogLinear)
//!     .build()
//!     .unwrap();
//! assert!(ic.inflation_rate(0.0, 5.0) > 0.0);
//! ```

extern crate alloc;
use alloc::{boxed::Box, vec::Vec};

use super::common::{build_interp, split_points, OneDGrid};
use crate::market_data::interp::{ExtrapolationPolicy, InterpStyle};
use crate::{
    error::InputError,
    market_data::interp::types::Interp,
    market_data::traits::{Inflation as InflationTrait, TermStructure},
    types::CurveId,
    F,
};

/// Real or breakeven inflation curve expressed as CPI index levels.
#[derive(Debug)]
pub struct InflationCurve {
    id: CurveId,
    base_cpi: F,
    /// Knot times in **years**.
    knots: Box<[F]>,
    /// CPI index levels at each knot.
    cpi_levels: Box<[F]>,
    interp: Interp,
}

/// Serializable representation of InflationCurve
#[cfg(feature = "serde")]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct InflationCurveData {
    id: CurveId,
    base_cpi: F,
    knots: Vec<F>,
    cpi_levels: Vec<F>,
    interp_style: InterpStyle,
    extrapolation: ExtrapolationPolicy,
}

impl InflationCurve {
    /// Start building an inflation curve with identifier `id`.
    pub fn builder(id: impl Into<CurveId>) -> InflationCurveBuilder {
        InflationCurveBuilder {
            id: id.into(),
            base_cpi: 100.0,
            points: Vec::new(),
            style: InterpStyle::LogLinear,
        }
    }

    /// CPI level at time `t` (years).
    pub fn cpi(&self, t: F) -> F {
        if t <= 0.0 {
            return self.base_cpi;
        }
        self.interp.interp(t)
    }

    /// Simple annualised inflation rate between `t1` and `t2`.
    pub fn inflation_rate(&self, t1: F, t2: F) -> F {
        debug_assert!(t2 > t1);
        let c1 = self.cpi(t1);
        let c2 = self.cpi(t2);
        (c2 / c1 - 1.0) / (t2 - t1)
    }

    /// Curve identifier.
    #[inline]
    pub fn id(&self) -> &CurveId {
        &self.id
    }

    /// Underlying bootstrap knot times (years).
    #[inline]
    pub fn knots(&self) -> &[F] {
        &self.knots
    }

    /// CPI levels provided at each knot.
    #[inline]
    pub fn cpi_levels(&self) -> &[F] {
        &self.cpi_levels
    }

    /// Base CPI level at t = 0.
    #[inline]
    pub fn base_cpi(&self) -> F {
        self.base_cpi
    }
}

impl TermStructure for InflationCurve {
    fn id(&self) -> &crate::types::CurveId {
        &self.id
    }
}

impl InflationTrait for InflationCurve {
    fn cpi(&self, t: F) -> F {
        InflationCurve::cpi(self, t)
    }

    fn inflation_rate(&self, t1: F, t2: F) -> F {
        InflationCurve::inflation_rate(self, t1, t2)
    }
}

/// Fluent builder for [`InflationCurve`].
pub struct InflationCurveBuilder {
    id: CurveId,
    base_cpi: F,
    points: Vec<(F, F)>, // (t, cpi)
    style: InterpStyle,
}

impl InflationCurveBuilder {
    /// Set the **base CPI** level at t = 0.
    pub fn base_cpi(mut self, cpi: F) -> Self {
        self.base_cpi = cpi;
        self
    }
    /// Supply knot points `(t, cpi_level)`.
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

    /// Validate input and build the [`InflationCurve`].
    pub fn build(self) -> crate::Result<InflationCurve> {
        if self.points.is_empty() {
            return Err(InputError::TooFewPoints.into());
        }
        crate::math::interp::utils::validate_knots(
            &self.points.iter().map(|p| p.0).collect::<Vec<_>>(),
        )?;
        if self.points.iter().any(|&(_, c)| c <= 0.0) {
            return Err(InputError::NonPositiveValue.into());
        }
        let (kvec, cvec): (Vec<F>, Vec<F>) = split_points(self.points);
        crate::math::interp::utils::validate_knots(&kvec)?;
        let knots = kvec.into_boxed_slice();
        let cpi_levels = cvec.into_boxed_slice();
        let grid = OneDGrid::new(knots.clone(), cpi_levels.clone());
        let interp = build_interp(self.style, &grid, ExtrapolationPolicy::default())?;
        Ok(InflationCurve {
            id: self.id,
            base_cpi: self.base_cpi,
            knots,
            cpi_levels,
            interp,
        })
    }
}

// Interpolator helpers centralised in InterpStyle – local factory fns removed.

// -----------------------------------------------------------------------------
// Serialization support
// -----------------------------------------------------------------------------

#[cfg(feature = "serde")]
impl serde::Serialize for InflationCurve {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let data = InflationCurveData {
            id: self.id.clone(),
            base_cpi: self.base_cpi,
            knots: self.knots.to_vec(),
            cpi_levels: self.cpi_levels.to_vec(),
            interp_style: self.interp.style(),
            extrapolation: self.interp.extrapolation(),
        };
        data.serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for InflationCurve {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;

        let data = InflationCurveData::deserialize(deserializer)?;
        let grid = OneDGrid::new(
            data.knots.clone().into_boxed_slice(),
            data.cpi_levels.clone().into_boxed_slice(),
        );
        let interp = build_interp(data.interp_style, &grid, data.extrapolation)
            .map_err(|_| D::Error::custom("Failed to reconstruct interpolator"))?;

        Ok(InflationCurve {
            id: data.id,
            base_cpi: data.base_cpi,
            knots: data.knots.into_boxed_slice(),
            cpi_levels: data.cpi_levels.into_boxed_slice(),
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

    fn sample_curve() -> InflationCurve {
        InflationCurve::builder("US-CPI")
            .base_cpi(300.0)
            .knots([(0.0, 300.0), (1.0, 306.0), (2.0, 312.0)])
            .build()
            .unwrap()
    }

    #[test]
    fn cpi_hits_knots() {
        let ic = sample_curve();
        assert!((ic.cpi(1.0) - 306.0).abs() < 1e-9);
    }

    #[test]
    fn inflation_rate_positive() {
        let ic = sample_curve();
        let r = ic.inflation_rate(0.0, 1.0);
        assert!(r > 0.0);
    }
}
