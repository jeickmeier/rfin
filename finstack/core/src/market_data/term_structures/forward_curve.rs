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
//! let base = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");
//! let fc = ForwardCurve::builder("USD-SOFR3M", 0.25)
//!     .base_date(base)
//!     .knots([(0.0, 0.03), (5.0, 0.04)])
//!     .set_interp(InterpStyle::Linear)
//!     .build()
//!     .expect("ForwardCurve builder should succeed");
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(try_from = "RawForwardCurve", into = "RawForwardCurve")
)]
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

impl Clone for ForwardCurve {
    fn clone(&self) -> Self {
        let interp = super::common::build_interp(
            self.interp.style(),
            self.knots.clone(),
            self.forwards.clone(),
            self.interp.extrapolation(),
        )
        .expect("Clone should not fail for valid curve");

        Self {
            id: self.id.clone(),
            base: self.base,
            reset_lag: self.reset_lag,
            day_count: self.day_count,
            tenor: self.tenor,
            knots: self.knots.clone(),
            forwards: self.forwards.clone(),
            interp,
        }
    }
}

/// Raw serializable state of ForwardCurve
#[cfg(feature = "serde")]
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawForwardCurve {
    #[serde(flatten)]
    common_id: super::common::StateId,
    /// Base date
    pub base: Date,
    /// Reset lag in calendar days
    pub reset_lag: i32,
    /// Day count convention
    pub day_count: DayCount,
    /// Index tenor in years
    pub tenor: f64,
    #[serde(flatten)]
    points: super::common::StateKnotPoints,
    #[serde(flatten)]
    interp: super::common::StateInterp,
}

#[cfg(feature = "serde")]
impl From<ForwardCurve> for RawForwardCurve {
    fn from(curve: ForwardCurve) -> Self {
        let knot_points: Vec<(f64, f64)> = curve
            .knots
            .iter()
            .zip(curve.forwards.iter())
            .map(|(&t, &fwd)| (t, fwd))
            .collect();

        RawForwardCurve {
            common_id: super::common::StateId {
                id: curve.id.to_string(),
            },
            base: curve.base,
            reset_lag: curve.reset_lag,
            day_count: curve.day_count,
            tenor: curve.tenor,
            points: super::common::StateKnotPoints { knot_points },
            interp: super::common::StateInterp {
                interp_style: curve.interp.style(),
                extrapolation: curve.interp.extrapolation(),
            },
        }
    }
}

#[cfg(feature = "serde")]
impl TryFrom<RawForwardCurve> for ForwardCurve {
    type Error = crate::Error;

    fn try_from(state: RawForwardCurve) -> crate::Result<Self> {
        ForwardCurve::builder(state.common_id.id, state.tenor)
            .base_date(state.base)
            .reset_lag(state.reset_lag)
            .day_count(state.day_count)
            .knots(state.points.knot_points)
            .set_interp(state.interp.interp_style)
            .extrapolation(state.interp.extrapolation)
            .build()
    }
}

impl ForwardCurve {
    /// Start building a forward curve for `id` with tenor `tenor_years`.
    ///
    /// **Defaults:** Linear interpolation with FlatForward extrapolation maintains
    /// stable tail forward rates consistent with market practice.
    pub fn builder(id: impl Into<CurveId>, tenor_years: f64) -> ForwardCurveBuilder {
        ForwardCurveBuilder {
            id: id.into(),
            base: Date::from_calendar_date(1970, time::Month::January, 1)
                .expect("January 1, 1970 should always be valid"),
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
    ///
    /// # Note
    /// For industry-standard key-rate DV01 with triangular weighting that localizes
    /// impact around the target tenor, use [`try_with_triangular_key_rate_bump`](Self::try_with_triangular_key_rate_bump).
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

    /// Create a new curve with a triangular key-rate bump (industry standard).
    ///
    /// This implements the market-standard key-rate DV01 bump using triangular
    /// weighting (per Tuckman/Fabozzi). The shock peaks at the target tenor `t`
    /// and linearly decays to zero at adjacent curve knots, localizing the rate
    /// impact to a specific maturity bucket.
    ///
    /// # Algorithm
    ///
    /// For each knot point `k`:
    /// - If `k < t_prev`: weight = 0 (unaffected)
    /// - If `t_prev <= k <= t`: weight increases linearly from 0 to 1
    /// - If `t <= k <= t_next`: weight decreases linearly from 1 to 0
    /// - If `k > t_next`: weight = 0 (unaffected)
    ///
    /// The forward rate is then bumped: `rate_bumped = rate + bump * weight`
    ///
    /// # Arguments
    /// * `t` - Target time in years at which to apply the key-rate bump
    /// * `bp` - Bump size in basis points (100bp = 1%)
    ///
    /// # Returns
    /// A new forward curve with the triangular key-rate bump applied.
    ///
    /// # Errors
    /// Returns an error if the bumped curve violates validation constraints.
    ///
    /// # Examples
    /// ```rust,no_run
    /// use finstack_core::market_data::term_structures::forward_curve::ForwardCurve;
    /// use time::{Date, Month};
    ///
    /// let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    /// let curve = ForwardCurve::builder("USD_SOFR_3M", 0.25)
    ///     .base_date(base_date)
    ///     .knots(vec![(1.0, 0.045), (2.0, 0.048), (5.0, 0.050), (10.0, 0.052)])
    ///     .build()
    ///     .unwrap();
    ///
    /// // Apply 10bp bump at 5Y - effect peaks at 5Y, decays to 2Y and 10Y
    /// let bumped = curve.try_with_triangular_key_rate_bump(5.0, 10.0).unwrap();
    /// ```
    pub fn try_with_triangular_key_rate_bump(&self, t: f64, bp: f64) -> crate::Result<Self> {
        if self.knots.len() < 2 {
            return self.try_with_parallel_bump(bp);
        }

        let times = &self.knots;
        let bump_rate = bp / 10_000.0;

        // Find the indices bracketing the target time
        let (i_prev, i_next) = find_bracket_indices(times, t);

        // Get the actual times at the bracket indices
        let t_prev = if i_prev > 0 {
            times[i_prev - 1]
        } else {
            0.0 // Use 0 as the left boundary if target is at or before first knot
        };
        let t_next = times.get(i_next).copied().unwrap_or(times[times.len() - 1]);

        let mut bumped_rates: Vec<(f64, f64)> = Vec::with_capacity(self.knots.len());

        for (&knot_t, &rate) in self.knots.iter().zip(self.forwards.iter()) {
            // Calculate triangular weight: peaks at t, decays to 0 at neighbors
            let weight = if knot_t <= t_prev {
                0.0
            } else if knot_t <= t {
                // Rising edge: 0 at t_prev, 1 at t
                let denom = (t - t_prev).max(1e-10);
                (knot_t - t_prev) / denom
            } else if knot_t <= t_next {
                // Falling edge: 1 at t, 0 at t_next
                let denom = (t_next - t).max(1e-10);
                (t_next - knot_t) / denom
            } else {
                0.0
            };

            // Apply weighted bump: rate_bumped = rate + bump * weight
            let new_rate = rate + bump_rate * weight;
            bumped_rates.push((knot_t, new_rate));
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

    /// Roll the curve forward by a specified number of days.
    ///
    /// This creates a new curve with:
    /// - Base date advanced by `days`
    /// - Knot times shifted backwards (t' = t - dt_years)
    /// - Points with t' <= 0 are filtered out (expired)
    /// - Forward rates are preserved (no carry/theta adjustment)
    ///
    /// This is the "constant curves" or "pure roll-down" scenario where forward
    /// rates at each calendar date remain the same, but maturity times are
    /// re-measured from the new base date.
    ///
    /// # Arguments
    /// * `days` - Number of days to roll forward
    ///
    /// # Returns
    /// A new forward curve with updated base date and shifted knots.
    ///
    /// # Errors
    /// Returns an error if fewer than 2 knot points remain after filtering expired points.
    ///
    /// # Examples
    /// ```rust,no_run
    /// use finstack_core::market_data::term_structures::forward_curve::ForwardCurve;
    /// use time::{Date, Month};
    ///
    /// let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    /// let curve = ForwardCurve::builder("USD_SOFR_3M", 0.25)
    ///     .base_date(base_date)
    ///     .knots(vec![(0.5, 0.045), (1.0, 0.048), (2.0, 0.050), (5.0, 0.052)])
    ///     .build()
    ///     .unwrap();
    ///
    /// // Roll 6 months forward - the 0.5Y point expires
    /// let rolled = curve.roll_forward(182).unwrap();
    /// assert!(rolled.knots().len() < curve.knots().len());
    /// ```
    pub fn roll_forward(&self, days: i64) -> crate::Result<Self> {
        let dt_years = days as f64 / 365.0;
        let new_base = self.base + time::Duration::days(days);

        // Shift knots and filter expired points
        let rolled_points: Vec<(f64, f64)> = self
            .knots
            .iter()
            .zip(self.forwards.iter())
            .filter_map(|(&t, &rate)| {
                let new_t = t - dt_years;
                if new_t > 0.0 {
                    Some((new_t, rate))
                } else {
                    None
                }
            })
            .collect();

        if rolled_points.len() < 2 {
            return Err(crate::error::InputError::TooFewPoints.into());
        }

        ForwardCurve::builder(self.id.clone(), self.tenor)
            .base_date(new_base)
            .reset_lag(self.reset_lag)
            .day_count(self.day_count)
            .knots(rolled_points)
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
// Private helper functions
// -----------------------------------------------------------------------------

/// Find bracket indices for triangular key-rate bumps.
///
/// Returns (i_prev, i_next) where:
/// - i_prev: index of the first knot >= t (or 0 if t is before all knots)
/// - i_next: index of the first knot > t (or last index if t is at or after all knots)
fn find_bracket_indices(times: &[f64], t: f64) -> (usize, usize) {
    // Find first index where knot >= t
    let i_prev = times
        .iter()
        .position(|&knot| knot >= t)
        .unwrap_or(times.len() - 1);

    // Find first index where knot > t
    let i_next = times
        .iter()
        .position(|&knot| knot > t)
        .unwrap_or(times.len() - 1)
        .max(i_prev);

    // Ensure i_next is at least i_prev + 1 when possible
    let i_next = if i_next == i_prev && i_next < times.len() - 1 {
        i_next + 1
    } else {
        i_next
    };

    (i_prev, i_next)
}

// -----------------------------------------------------------------------------
// Serialization support
// -----------------------------------------------------------------------------

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
            .expect("ForwardCurve builder should succeed with valid test data")
    }

    #[test]
    fn interpolates_rate() {
        let fc = sample_forward();
        assert!((fc.rate(0.5) - 0.035).abs() < 1e-12);
    }

    #[test]
    fn tail_continuity_with_flatforward_extrapolation() {
        // Test that FlatForward extrapolation maintains stable tail forwards
        let base =
            Date::from_calendar_date(2025, time::Month::January, 1).expect("Valid test date");
        let fc = ForwardCurve::builder("USD-SOFR-3M", 0.25)
            .base_date(base)
            .knots([(0.0, 0.03), (1.0, 0.035), (5.0, 0.04)])
            .set_interp(InterpStyle::Linear)
            .extrapolation(ExtrapolationPolicy::FlatForward)
            .build()
            .expect("ForwardCurve builder should succeed with valid test data");

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
        let base =
            Date::from_calendar_date(2025, time::Month::January, 1).expect("Valid test date");
        let fc = ForwardCurve::builder("TEST", 0.25)
            .base_date(base)
            .knots([(0.0, 0.03), (1.0, 0.04)])
            .build()
            .expect("ForwardCurve builder should succeed with valid test data");

        // With FlatForward, tail rate should be stable (not zero)
        let rate_tail = fc.rate(5.0);
        assert!(
            rate_tail > 0.02,
            "Tail forward should remain positive with FlatForward: {:.6}",
            rate_tail
        );
    }
}
