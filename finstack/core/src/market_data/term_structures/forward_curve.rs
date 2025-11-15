//! Forward rate curves for floating-rate indices.
//!
//! A forward curve represents expected future interest rates for a specific
//! index (e.g., 3-month SOFR, 6-month EURIBOR). These curves are essential
//! for pricing floating-rate instruments and calculating forward-looking
//! cash flows in swaps and floating-rate notes.
//!
//! # Financial Concept
//!
//! The forward rate f(t₁, t₂) is the rate agreed today for borrowing/lending
//! from time t₁ to t₂:
//! ```text
//! f(t₁, t₂) = [DF(t₁) / DF(t₂) - 1] / (t₂ - t₁)
//!
//! For a fixed-tenor index (e.g., 3M):
//! f(t) = forward rate resetting at time t for the index tenor
//! ```
//!
//! # Market Construction
//!
//! Forward curves are typically bootstrapped from:
//! - **Futures**: SOFR futures, Eurodollar futures (liquid up to ~5 years)
//! - **FRA** (Forward Rate Agreements): OTC quotes for forward rates
//! - **Swaps**: Float leg expectations from swap rates
//! - **Basis spreads**: Tenor basis between different index tenors
//!
//! # Index Conventions
//!
//! Each index has specific conventions:
//! - **SOFR**: Daily compounded in arrears, Act/360
//! - **EURIBOR**: Simple rate, Act/360, 2-day spot lag
//! - **SONIA**: Daily compounded in arrears, Act/365F
//! - **TIBOR**: Simple rate, Act/365F
//!
//! # Use Cases
//!
//! - **Floating-rate note pricing**: Project future coupon payments
//! - **Interest rate swap valuation**: Mark-to-market floating leg
//! - **Cap/floor pricing**: Forward rates determine intrinsic value
//! - **Basis swap pricing**: Spread between different index tenors
//!
//! # Examples
//!
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
//!
//! # References
//!
//! - Hull, J. C. (2018). *Options, Futures, and Other Derivatives* (10th ed.).
//!   Chapters 4-6 (Forward rates and curve construction).
//! - Andersen, L., & Piterbarg, V. (2010). *Interest Rate Modeling*.
//!   Volume 1, Chapter 3 (Multi-curve framework).
//! - Ametrano, F. M., & Bianchetti, M. (2013). "Everything You Always Wanted to
//!   Know About Multiple Interest Rate Curve Bootstrapping but Were Afraid to Ask."
//!   SSRN Working Paper.

use super::common::{build_interp, split_points};
use crate::math::interp::{ExtrapolationPolicy, InterpStyle};
use crate::{
    dates::{Date, DayCount},
    error::InputError,
    market_data::traits::{Forward, TermStructure},
    math::interp::types::Interp,
    types::CurveId,
};

/// Forward rate curve for a floating-rate index with fixed tenor.
///
/// Represents expected future interest rates for a specific index (e.g., 3-month
/// SOFR, 6-month EURIBOR). Stores simple forward rates at knot times and
/// interpolates between them.
///
/// # Index Components
///
/// - **Tenor**: Index accrual period (e.g., 0.25 years = 3 months)
/// - **Reset lag**: Days from fixing date to effective date
/// - **Day count**: Convention for accrual (usually Act/360 or Act/365F)
///
/// # Thread Safety
///
/// Immutable after construction; safe to share via `Arc<ForwardCurve>`.
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
    ///
    /// **Defaults:** Linear interpolation with FlatForward extrapolation maintains
    /// stable tail forward rates consistent with market practice.
    pub fn builder(id: impl Into<CurveId>, tenor_years: f64) -> ForwardCurveBuilder {
        ForwardCurveBuilder {
            id: id.into(),
            base: Date::from_calendar_date(1970, time::Month::January, 1).unwrap(),
            reset_lag: 2,
            day_count: DayCount::Act360,
            tenor: tenor_years,
            points: Vec::new(),
            style: InterpStyle::Linear,
            min_forward_rate: None,
            extrapolation: ExtrapolationPolicy::FlatForward,
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

    /// Create a new curve with a key-rate bump applied at a target time `t` (in years) (fallible).
    ///
    /// This approximates a classic key-rate DV01 bump by applying an additive rate
    /// shift only to the forward segment that contains `t`. The effect is localized
    /// to the bracketed interval [t_i, t_{i+1}] that contains `t` by shifting forward
    /// rates at and beyond `t_{i+1}` by the bump amount.
    ///
    /// If the curve has fewer than 2 knots, falls back to a parallel bump.
    ///
    /// Returns an error if the bumped curve violates validation constraints.
    pub fn try_with_key_rate_bump_years(&self, t: f64, bp: f64) -> crate::Result<Self> {
        if self.knots.len() < 2 {
            // Fallback to parallel bump for degenerate curves
            return self.try_with_parallel_bump(bp);
        }

        // Find segment i such that knots[i] < t <= knots[i+1]
        let times = &self.knots;
        let mut i = 0usize;
        if t <= times[0] {
            i = 0;
        } else if t >= times[times.len() - 1] {
            i = times.len() - 2;
        } else {
            for idx in 0..times.len() - 1 {
                if t > times[idx] && t <= times[idx + 1] {
                    i = idx;
                    break;
                }
            }
        }

        let bump_rate = bp / 10_000.0;

        let mut bumped_rates: Vec<(f64, f64)> = Vec::with_capacity(self.knots.len());
        for (idx, (&tt, &rate)) in self.knots.iter().zip(self.forwards.iter()).enumerate() {
            let new_rate = if idx > i { rate + bump_rate } else { rate };
            bumped_rates.push((tt, new_rate));
        }

        let new_id = crate::market_data::bumps::id_bump_bp(self.id.as_str(), bp);
        ForwardCurve::builder(new_id, self.tenor)
            .base_date(self.base)
            .reset_lag(self.reset_lag)
            .day_count(self.day_count)
            .knots(bumped_rates)
            .set_interp(self.interp.style())
            .extrapolation(self.interp.extrapolation())
            .build()
    }

    /// Create a new curve with a parallel rate bump applied in basis points (fallible).
    ///
    /// Adds the bump amount (converted from bp) to all forward rates uniformly.
    ///
    /// Returns an error if the bumped curve violates validation constraints.
    pub fn try_with_parallel_bump(&self, bp: f64) -> crate::Result<Self> {
        let bump_rate = bp / 10_000.0;
        let bumped_points: Vec<(f64, f64)> = self
            .knots
            .iter()
            .zip(self.forwards.iter())
            .map(|(&t, &rate)| (t, rate + bump_rate))
            .collect();

        // Derive new ID with suffix
        let new_id = crate::market_data::bumps::id_bump_bp(self.id.as_str(), bp);

        // Rebuild preserving base date, interpolation, and extrapolation policies
        ForwardCurve::builder(new_id, self.tenor)
            .base_date(self.base)
            .reset_lag(self.reset_lag)
            .day_count(self.day_count)
            .knots(bumped_points)
            .set_interp(self.interp.style())
            .extrapolation(self.interp.extrapolation())
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
    min_forward_rate: Option<f64>,
    extrapolation: ExtrapolationPolicy,
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

    /// Set the extrapolation policy for out-of-bounds evaluation.
    pub fn extrapolation(mut self, policy: ExtrapolationPolicy) -> Self {
        self.extrapolation = policy;
        self
    }

    /// Enforce a minimum forward rate across the provided knot points.
    pub fn with_min_forward_rate(mut self, min_rate: f64) -> Self {
        self.min_forward_rate = Some(min_rate);
        self
    }

    /// Convenience for requiring non-negative forwards.
    pub fn require_non_negative_forwards(self) -> Self {
        self.with_min_forward_rate(0.0)
    }

    /// Validate input and build the [`ForwardCurve`].
    pub fn build(self) -> crate::Result<ForwardCurve> {
        if self.points.len() < 2 {
            return Err(InputError::TooFewPoints.into());
        }
        let (kvec, fvec): (Vec<f64>, Vec<f64>) = split_points(self.points);
        crate::math::interp::utils::validate_knots(&kvec)?;
        if let Some(min_fwd) = self.min_forward_rate {
            for (i, &f) in fvec.iter().enumerate() {
                if f < min_fwd {
                    return Err(crate::Error::Validation(format!(
                        "Forward rate below minimum at t={:.6}: fwd={:.8} < min={:.8} (index {})",
                        kvec[i], f, min_fwd, i
                    )));
                }
            }
        }
        let knots = kvec.into_boxed_slice();
        let forwards = fvec.into_boxed_slice();
        let interp = build_interp(
            self.style,
            knots.clone(),
            forwards.clone(),
            self.extrapolation,
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

    #[test]
    fn tail_continuity_with_flatforward_extrapolation() {
        // Test that FlatForward extrapolation maintains stable tail forwards
        let base = Date::from_calendar_date(2025, time::Month::January, 1).unwrap();
        let fc = ForwardCurve::builder("USD-SOFR-3M", 0.25)
            .base_date(base)
            .knots([(0.0, 0.03), (1.0, 0.035), (5.0, 0.04)])
            .set_interp(InterpStyle::Linear)
            .extrapolation(ExtrapolationPolicy::FlatForward)
            .build()
            .unwrap();

        // Rate at last knot and beyond should be continuous
        let rate_at_last = fc.rate(5.0);
        let rate_beyond = fc.rate(10.0);

        // FlatForward should maintain the rate (or slope)
        let abs_diff = (rate_beyond - rate_at_last).abs();
        assert!(
            abs_diff < 0.01,
            "Forward rate tail discontinuity: rate_at_last={:.6}, rate_beyond={:.6}",
            rate_at_last,
            rate_beyond
        );
    }

    #[test]
    fn default_uses_flatforward_extrapolation() {
        // Verify new market-standard default extrapolation
        let base = Date::from_calendar_date(2025, time::Month::January, 1).unwrap();
        let fc = ForwardCurve::builder("TEST", 0.25)
            .base_date(base)
            .knots([(0.0, 0.03), (1.0, 0.04)])
            .build()
            .unwrap();

        // With FlatForward, tail rate should be stable (not zero)
        let rate_tail = fc.rate(5.0);
        assert!(
            rate_tail > 0.02,
            "Tail forward should remain positive with FlatForward: {:.6}",
            rate_tail
        );
    }
}
