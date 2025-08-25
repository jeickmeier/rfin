//! Piece-wise constant credit *hazard curve* λ(t).
//!
//! The hazard rate is assumed constant between successive knot times making
//! survival probabilities analytical and **fast** to compute.  The curve
//! implements [`crate::market_data::traits::Survival`].
//!
//! ## Example
//! ```rust
//! use rfin_core::market_data::term_structures::hazard_curve::HazardCurve;
//! let hc = HazardCurve::builder("USD-CRED")
//!     .knots([(0.0, 0.01), (10.0, 0.015)])
//!     .build()
//!     .unwrap();
//! assert!(hc.sp(5.0) < 1.0);
//! ```

extern crate alloc;
use alloc::{boxed::Box, vec::Vec};

use crate::{
    dates::Date,
    error::InputError,
    market_data::id::CurveId,
    market_data::traits::{Survival, TermStructure},
    F,
};

/// Piecewise‐constant credit hazard curve.
///
/// λ(t) is assumed constant in each interval between knots. The survival
/// probability is therefore
/// `S(t) = exp(-∫_0^t λ(u) du)` which for piecewise‐constant λ simplifies
/// to `exp(-∑ λ_i * Δt_i)`.
#[derive(Debug)]
pub struct HazardCurve {
    id: CurveId,
    base: Date,
    knots: Box<[F]>,   // times in years, strictly increasing (first may be 0.0)
    lambdas: Box<[F]>, // hazard rates λ ≥ 0 with same length as knots
}

impl HazardCurve {
    /// Start building a hazard curve with identifier `id`.
    pub fn builder(id: &'static str) -> HazardCurveBuilder {
        HazardCurveBuilder {
            id,
            base: Date::from_calendar_date(1970, time::Month::January, 1).unwrap(),
            points: Vec::new(),
        }
    }

    /// Survival probability S(t) up to time `t` (in **years**).
    #[must_use]
    pub fn sp(&self, t: F) -> F {
        if t <= 0.0 {
            return 1.0;
        }
        let mut accum = 0.0;
        let mut prev = 0.0;
        for (i, &k) in self.knots.iter().enumerate() {
            let dt = if t <= k { t - prev } else { k - prev };
            accum += self.lambdas[i] * dt;
            prev = k;
            if t <= k {
                break;
            }
        }
        // If t beyond last knot, extend with last lambda
        if t > *self.knots.last().unwrap() {
            accum += self.lambdas.last().copied().unwrap() * (t - *self.knots.last().unwrap());
        }
        (-accum).exp()
    }

    /// Default probability between `t1` and `t2`.
    #[must_use]
    pub fn default_prob(&self, t1: F, t2: F) -> F {
        debug_assert!(t2 >= t1);
        let sp1 = self.sp(t1);
        let sp2 = self.sp(t2);
        sp1 - sp2
    }

    /// Accessors
    pub fn id(&self) -> &CurveId {
        &self.id
    }
    /// Curve valuation **base date**.
    pub fn base_date(&self) -> Date {
        self.base
    }
}

impl TermStructure for HazardCurve {
    fn id(&self) -> &CurveId {
        &self.id
    }
}

impl Survival for HazardCurve {
    fn sp(&self, t: F) -> F {
        HazardCurve::sp(self, t)
    }

    fn default_prob(&self, t1: F, t2: F) -> F {
        HazardCurve::default_prob(self, t1, t2)
    }
}

/// Fluent builder for [`HazardCurve`].
pub struct HazardCurveBuilder {
    id: &'static str,
    base: Date,
    points: Vec<(F, F)>, // (t, lambda)
}

impl HazardCurveBuilder {
    /// Set the **base date** for the curve.
    pub fn base_date(mut self, d: Date) -> Self {
        self.base = d;
        self
    }
    /// Supply knot points `(t, λ)` where λ is the hazard rate.
    pub fn knots<I>(mut self, pts: I) -> Self
    where
        I: IntoIterator<Item = (F, F)>,
    {
        self.points.extend(pts);
        self
    }

    /// Validate input and build the [`HazardCurve`].
    pub fn build(self) -> crate::Result<HazardCurve> {
        if self.points.is_empty() {
            return Err(InputError::TooFewPoints.into());
        }
        // Validate non-negative hazard rates
        if self.points.iter().any(|&(_, l)| l < 0.0) {
            return Err(InputError::NegativeValue.into());
        }
        let (kvec, lvec): (Vec<F>, Vec<F>) = self.points.into_iter().unzip();
        if kvec.len() > 1 {
            crate::market_data::utils::validate_knots(&kvec)?;
        }
        Ok(HazardCurve {
            id: CurveId::new(self.id),
            base: self.base,
            knots: kvec.into_boxed_slice(),
            lambdas: lvec.into_boxed_slice(),
        })
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn survival_monotone_decreasing() {
        let hc = HazardCurve::builder("USD-CREDIT")
            .knots([(0.0, 0.01), (5.0, 0.02)])
            .build()
            .unwrap();
        assert!(hc.sp(1.0) < 1.0);
        assert!(hc.sp(6.0) < hc.sp(1.0));
    }

    #[test]
    fn default_prob_positive() {
        let hc = HazardCurve::builder("USD")
            .knots([(0.0, 0.01), (10.0, 0.015)])
            .build()
            .unwrap();
        let dp = hc.default_prob(2.0, 4.0);
        assert!(dp >= 0.0);
    }
}
