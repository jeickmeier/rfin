//! Piece-wise constant credit *hazard curve* λ(t).
//!
//! The hazard rate is assumed constant between successive knot times making
//! survival probabilities analytical and **fast** to compute.  The curve
//! implements [`crate::market_data::traits::Survival`].
//!
//! ## Example
//! ```rust
//! use finstack_core::market_data::term_structures::hazard_curve::HazardCurve;
//! use finstack_core::dates::Date;
//! use time::Month;
//!
//! let base = Date::from_calendar_date(2025, Month::January, 1).unwrap();
//! let hc = HazardCurve::builder("USD-CRED")
//!     .base_date(base)
//!     .knots([(0.0, 0.01), (10.0, 0.015)])
//!     .build()
//!     .unwrap();
//! assert!(hc.sp(5.0) < 1.0);
//! ```

use crate::{
    currency::Currency,
    dates::{Date, DayCount},
    error::InputError,
    market_data::traits::{Survival, TermStructure},
    types::CurveId,
    F,
};

/// Piecewise‐constant credit hazard curve.
///
/// λ(t) is assumed constant in each interval between knots. The survival
/// probability is therefore
/// `S(t) = exp(-∫_0^t λ(u) du)` which for piecewise‐constant λ simplifies
/// to `exp(-∑ λ_i * Δt_i)`.
///
/// Use `to_state()` and `from_state()` for serialization.
#[derive(Debug)]
pub struct HazardCurve {
    id: CurveId,
    base: Date,
    /// Time grid in years from base date; strictly increasing (first may be 0.0)
    knots: Box<[F]>,
    /// Piecewise-constant hazard rates λ ≥ 0; same length as `knots`.
    lambdas: Box<[F]>,
    /// Recovery rate used during calibration/reporting (metadata)
    recovery_rate: F,
    /// Optional issuer metadata
    issuer: Option<String>,
    /// Debt seniority
    pub seniority: Option<Seniority>,
    /// Currency of protection leg (metadata)
    currency: Option<Currency>,
    /// Day count convention for converting dates→times (metadata)
    day_count: DayCount,
    /// Stored market par spreads used to bootstrap this curve (for reporting)
    par_tenors: Box<[F]>,
    /// Par spreads in basis points at `par_tenors`
    par_spreads_bp: Box<[F]>,
}

/// Serializable state of a HazardCurve
#[cfg(feature = "serde")]
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HazardCurveState {
    #[cfg_attr(feature = "serde", serde(flatten))]
    common_id: super::common::StateId,
    /// Base date
    pub base: Date,
    #[cfg_attr(feature = "serde", serde(flatten))]
    points: super::common::StateKnotPoints,
    /// Recovery rate
    pub recovery_rate: F,
    /// Optional issuer
    pub issuer: Option<String>,
    /// Seniority
    pub seniority: Option<Seniority>,
    /// Currency
    pub currency: Option<Currency>,
    /// Day count convention
    pub day_count: DayCount,
    /// Par spread points for reporting
    pub par_points: Vec<(F, F)>,
}

impl HazardCurve {
    /// Start building a hazard curve with identifier `id`.
    pub fn builder(id: impl Into<CurveId>) -> HazardCurveBuilder {
        HazardCurveBuilder {
            id: id.into(),
            base: Date::from_calendar_date(1970, time::Month::January, 1).unwrap(),
            points: Vec::new(),
            recovery_rate: 0.4,
            issuer: None,
            seniority: None,
            currency: None,
            day_count: DayCount::Act365F,
            par_points: Vec::new(),
        }
    }

    /// Survival probability S(t) up to time `t` (in **years**).
    #[must_use]
    pub fn sp(&self, t: F) -> F {
        if t <= 0.0 {
            return 1.0;
        }
        let mut accum: F = 0.0;
        let mut prev: F = 0.0;
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

    /// Recovery rate metadata used when mapping spreads↔hazards during bootstrap.
    pub fn recovery_rate(&self) -> F {
        self.recovery_rate
    }

    /// Day count convention associated with this curve's time axis.
    pub fn day_count(&self) -> DayCount {
        self.day_count
    }

    /// Access the knot points (time, lambda) for inspection or modification.
    pub fn knot_points(&self) -> impl Iterator<Item = (F, F)> + '_ {
        self.knots
            .iter()
            .zip(self.lambdas.iter())
            .map(|(&t, &lambda)| (t, lambda))
    }

    /// Access the par spread points for inspection.
    pub fn par_spread_points(&self) -> impl Iterator<Item = (F, F)> + '_ {
        self.par_tenors
            .iter()
            .zip(self.par_spreads_bp.iter())
            .map(|(&t, &spread)| (t, spread))
    }

    /// Create a builder with this curve's parameters, using a new ID.
    /// Useful for creating modified versions of the curve.
    pub fn to_builder_with_id(&self, new_id: impl Into<CurveId>) -> HazardCurveBuilder {
        let mut builder = HazardCurve::builder(new_id)
            .base_date(self.base)
            .recovery_rate(self.recovery_rate)
            .day_count(self.day_count);

        if let Some(ref issuer) = self.issuer {
            builder = builder.issuer(issuer.clone());
        }
        if let Some(seniority) = self.seniority {
            builder = builder.seniority(seniority);
        }
        if let Some(currency) = self.currency {
            builder = builder.currency(currency);
        }

        // Add existing knot points
        builder = builder.knots(self.knot_points());

        // Add existing par spread points
        builder = builder.par_spreads(self.par_spread_points());

        builder
    }

    /// Create a new curve with hazard rates shifted by a constant amount.
    /// Uses the same ID with a "_BUMPED" suffix.
    /// Negative shifts are clamped to zero to ensure non-negative hazard rates.
    pub fn with_hazard_shift(&self, shift: F) -> crate::Result<HazardCurve> {
        let shifted_points: Vec<(F, F)> = self
            .knot_points()
            .map(|(t, lambda)| (t, (lambda + shift).max(0.0)))
            .collect();

        // Create a temporary ID for the bumped curve
        // In practice, the caller will manage IDs when building market contexts
        let temp_id = "TEMP_BUMPED_HAZARD";

        let mut builder = HazardCurve::builder(temp_id)
            .base_date(self.base)
            .recovery_rate(self.recovery_rate)
            .day_count(self.day_count)
            .knots(shifted_points);

        if let Some(ref issuer) = self.issuer {
            builder = builder.issuer(issuer.clone());
        }
        if let Some(seniority) = self.seniority {
            builder = builder.seniority(seniority);
        }
        if let Some(currency) = self.currency {
            builder = builder.currency(currency);
        }

        // Add existing par spread points
        builder = builder.par_spreads(self.par_spread_points());

        builder.build()
    }

    /// Return an interpolated par spread in basis points for reporting.
    /// Linear interpolation in spread, with log-linear fallback when values are positive and requested.
    pub fn quoted_spread_bp(&self, t: F, method: ParInterp) -> F {
        let n = self.par_tenors.len();
        if n == 0 {
            return 0.0;
        }
        if t <= self.par_tenors[0] {
            return self.par_spreads_bp[0];
        }
        if t >= self.par_tenors[n - 1] {
            return self.par_spreads_bp[n - 1];
        }
        // Find bracket
        let mut i = 1;
        while i < n && t > self.par_tenors[i] {
            i += 1;
        }
        let i1 = i - 1;
        let (x1, x2) = (self.par_tenors[i1], self.par_tenors[i]);
        let (y1, y2) = (self.par_spreads_bp[i1], self.par_spreads_bp[i]);
        let w = (t - x1) / (x2 - x1);
        match method {
            ParInterp::Linear => y1 + w * (y2 - y1),
            ParInterp::LogLinear => {
                if y1 > 0.0 && y2 > 0.0 {
                    let a = y1.ln();
                    let b = y2.ln();
                    (a + w * (b - a)).exp()
                } else {
                    y1 + w * (y2 - y1)
                }
            }
        }
    }

    #[cfg(feature = "serde")]
    /// Extract serializable state
    pub fn to_state(&self) -> HazardCurveState {
        let knot_points: Vec<(F, F)> = self
            .knots
            .iter()
            .zip(self.lambdas.iter())
            .map(|(&t, &lambda)| (t, lambda))
            .collect();
        let par_points: Vec<(F, F)> = self
            .par_tenors
            .iter()
            .zip(self.par_spreads_bp.iter())
            .map(|(&t, &spread)| (t, spread))
            .collect();

        HazardCurveState {
            common_id: super::common::StateId {
                id: self.id.to_string(),
            },
            base: self.base,
            points: super::common::StateKnotPoints { knot_points },
            recovery_rate: self.recovery_rate,
            issuer: self.issuer.clone(),
            seniority: self.seniority,
            currency: self.currency,
            day_count: self.day_count,
            par_points,
        }
    }

    #[cfg(feature = "serde")]
    /// Create from serialized state
    pub fn from_state(state: HazardCurveState) -> crate::Result<Self> {
        let mut builder = HazardCurve::builder(state.common_id.id)
            .base_date(state.base)
            .recovery_rate(state.recovery_rate)
            .day_count(state.day_count)
            .knots(state.points.knot_points)
            .par_spreads(state.par_points);

        if let Some(issuer) = state.issuer {
            builder = builder.issuer(issuer);
        }
        if let Some(seniority) = state.seniority {
            builder = builder.seniority(seniority);
        }
        if let Some(currency) = state.currency {
            builder = builder.currency(currency);
        }

        builder.build()
    }
}

// Minimal trait implementations for polymorphism where needed

impl Survival for HazardCurve {
    #[inline]
    fn sp(&self, t: F) -> F {
        self.sp(t)
    }
}

impl TermStructure for HazardCurve {
    #[inline]
    fn id(&self) -> &CurveId {
        &self.id
    }
}

/// Fluent builder for [`HazardCurve`].
pub struct HazardCurveBuilder {
    id: CurveId,
    base: Date,
    points: Vec<(F, F)>, // (t, lambda)
    recovery_rate: F,
    issuer: Option<String>,
    seniority: Option<Seniority>,
    currency: Option<Currency>,
    day_count: DayCount,
    par_points: Vec<(F, F)>, // (t, spread_bp)
}

impl HazardCurveBuilder {
    /// Set the **base date** for the curve.
    pub fn base_date(mut self, d: Date) -> Self {
        self.base = d;
        self
    }
    /// Set issuer metadata.
    pub fn issuer(mut self, name: impl Into<String>) -> Self {
        self.issuer = Some(name.into());
        self
    }
    /// Set seniority metadata.
    pub fn seniority(mut self, s: Seniority) -> Self {
        self.seniority = Some(s);
        self
    }
    /// Set currency metadata.
    pub fn currency(mut self, ccy: Currency) -> Self {
        self.currency = Some(ccy);
        self
    }
    /// Set day-count convention for the curve time axis.
    pub fn day_count(mut self, dc: DayCount) -> Self {
        self.day_count = dc;
        self
    }
    /// Set recovery rate metadata.
    pub fn recovery_rate(mut self, r: F) -> Self {
        self.recovery_rate = r;
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
    /// Store the market par spreads used for bootstrap for reporting.
    pub fn par_spreads<I>(mut self, pts: I) -> Self
    where
        I: IntoIterator<Item = (F, F)>,
    {
        self.par_points.extend(pts);
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
        let mut points = self.points;
        points.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        let (kvec, lvec): (Vec<F>, Vec<F>) = points.into_iter().unzip();
        if kvec.len() > 1 {
            crate::math::interp::utils::validate_knots(&kvec)?;
        }
        let mut par_pts = self.par_points;
        par_pts.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        let (p_ten, p_spd): (Vec<F>, Vec<F>) = par_pts.into_iter().unzip();
        Ok(HazardCurve {
            id: self.id,
            base: self.base,
            knots: kvec.into_boxed_slice(),
            lambdas: lvec.into_boxed_slice(),
            recovery_rate: self.recovery_rate,
            issuer: self.issuer,
            seniority: self.seniority,
            currency: self.currency,
            day_count: self.day_count,
            par_tenors: p_ten.into_boxed_slice(),
            par_spreads_bp: p_spd.into_boxed_slice(),
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

    #[test]
    fn quoted_spread_interpolation_linear() {
        let hc = HazardCurve::builder("TEST")
            .knots([(1.0, 0.02)])
            .par_spreads([(1.0, 100.0), (3.0, 200.0)])
            .build()
            .unwrap();
        assert!((hc.quoted_spread_bp(2.0, ParInterp::Linear) - 150.0).abs() < 1e-9);
    }
}

// -----------------------------------------------------------------------------
// Types
// -----------------------------------------------------------------------------

/// Seniority level for credit exposures.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Seniority {
    /// Senior secured debt
    SeniorSecured,
    /// Senior unsecured debt
    Senior,
    /// Subordinated debt
    Subordinated,
    /// Junior/mezzanine debt
    Junior,
}

impl core::fmt::Display for Seniority {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Seniority::SeniorSecured => write!(f, "SeniorSecured"),
            Seniority::Senior => write!(f, "Senior"),
            Seniority::Subordinated => write!(f, "Subordinated"),
            Seniority::Junior => write!(f, "Junior"),
        }
    }
}

/// Interpolation method for reporting par spreads stored on the curve.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParInterp {
    /// Linear interpolation in spread space
    Linear,
    /// Log-linear interpolation when spreads are strictly positive
    LogLinear,
}
