//! Real / breakeven consumer-price index (CPI) curve expressed as index levels.
//!
//! Provides interpolated CPI values and derived annualised inflation rates via
//! [`crate::market_data::traits::Inflation`].  Accepts any interpolation style
//! supported by the [`crate::market_data::interp`] subsystem although
//! `LogLinear` is the most common choice for exponential CPI growth.
//!
//! ## Example
//! ```rust
//! use rfin_core::market_data::term_structures::inflation::InflationCurve;
//! let ic = InflationCurve::builder("US-CPI")
//!     .base_cpi(300.0)
//!     .knots([(0.0, 300.0), (5.0, 327.0)])
//!     .log_df()
//!     .build()
//!     .unwrap();
//! assert!(ic.inflation_rate(0.0, 5.0) > 0.0);
//! ```

extern crate alloc;
use alloc::{boxed::Box, vec::Vec};

use crate::market_data::interp::InterpStyle;
use crate::{
    error::InputError,
    market_data::id::CurveId,
    market_data::interp::InterpFn,
    market_data::traits::{Inflation as InflationTrait, TermStructure},
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
    interp: Box<dyn InterpFn>,
}

impl InflationCurve {
    /// Start building an inflation curve with identifier `id`.
    pub fn builder(id: &'static str) -> InflationCurveBuilder {
        InflationCurveBuilder {
            id,
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
}

impl TermStructure for InflationCurve {
    fn id(&self) -> &crate::market_data::id::CurveId {
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
    id: &'static str,
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
    /// Use linear interpolation.
    pub fn linear_df(mut self) -> Self {
        self.style = InterpStyle::Linear;
        self
    }
    /// Use log‐linear interpolation (default).
    pub fn log_df(mut self) -> Self {
        self.style = InterpStyle::LogLinear;
        self
    }
    /// Use monotone‐convex interpolation.
    pub fn monotone_convex(mut self) -> Self {
        self.style = InterpStyle::MonotoneConvex;
        self
    }
    /// Use cubic‐Hermite interpolation.
    pub fn cubic_hermite(mut self) -> Self {
        self.style = InterpStyle::CubicHermite;
        self
    }
    /// Use flat‐forward interpolation.
    pub fn flat_fwd(mut self) -> Self {
        self.style = InterpStyle::FlatFwd;
        self
    }

    /// Validate input and build the [`InflationCurve`].
    pub fn build(self) -> crate::Result<InflationCurve> {
        if self.points.is_empty() {
            return Err(InputError::TooFewPoints.into());
        }
        crate::market_data::utils::validate_knots(
            &self.points.iter().map(|p| p.0).collect::<Vec<_>>(),
        )?;
        if self.points.iter().any(|&(_, c)| c <= 0.0) {
            return Err(InputError::NonPositiveValue.into());
        }
        let (kvec, cvec): (Vec<F>, Vec<F>) = self.points.into_iter().unzip();
        crate::market_data::utils::validate_knots(&kvec)?;
        let knots = kvec.into_boxed_slice();
        let cpi_levels = cvec.into_boxed_slice();
        let interp = self.style.make_interp(knots.clone(), cpi_levels.clone())?;
        Ok(InflationCurve {
            id: CurveId::new(self.id),
            base_cpi: self.base_cpi,
            knots,
            cpi_levels,
            interp,
        })
    }
}

// Interpolator helpers centralised in InterpStyle – local factory fns removed.

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
