//! Discount factor curves for present value calculations.
//!
//! A discount curve represents the time value of money, mapping future dates to
//! present values. This is the fundamental building block for pricing all fixed
//! income securities and derivatives.
//!
//! # Financial Concept
//!
//! The discount factor DF(t) is the present value of $1 received at time t:
//! ```text
//! DF(t) = PV($1 at time t)
//!       = e^(-r(t) * t)
//!
//! where r(t) is the continuously compounded zero rate at maturity t
//! ```
//!
//! # Market Construction
//!
//! Discount curves are typically bootstrapped from liquid market instruments:
//! - **Money market**: Overnight rates (SOFR, €STR, SONIA)
//! - **Futures**: SOFR futures, Eurodollar futures
//! - **Swaps**: Fixed-float interest rate swaps (par rates)
//! - **Bonds**: Government bonds (when OIS not available)
//!
//! # Interpolation Methods
//!
//! The curve supports multiple interpolation schemes via [`crate::math::interp::InterpStyle`]:
//! - **Linear**: Simple, but may create arbitrage
//! - **LogLinear**: Constant zero rates between knots
//! - **FlatFwd**: Piecewise constant forward rates (no-arbitrage)
//! - **MonotoneConvex**: Smooth, no-arbitrage (Hagan-West algorithm)
//!
//! # Use Cases
//!
//! - **Bond pricing**: Discount future coupons and principal
//! - **Swap valuation**: Mark-to-market fixed and floating legs
//! - **Option pricing**: Discount expected payoffs
//! - **Risk metrics**: DV01, duration, convexity calculation
//!
//! ## Example
//! ```rust
//! use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
//! use finstack_core::dates::Date;
//! use time::Month;
//! # use finstack_core::math::interp::InterpStyle;
//!
//! let curve = DiscountCurve::builder("USD-OIS")
//!     .base_date(Date::from_calendar_date(2025, Month::January, 1).expect("Valid date"))
//!     .knots([(0.0, 1.0), (5.0, 0.9)])
//!     .set_interp(InterpStyle::MonotoneConvex)
//!     .build()
//!     .expect("DiscountCurve builder should succeed");
//! assert!(curve.df(3.0) < 1.0);
//! ```
//!
//! # References
//!
//! - **Curve Construction and Bootstrapping**:
//!   - Hull, J. C. (2018). *Options, Futures, and Other Derivatives* (10th ed.).
//!     Pearson. Chapters 4-7.
//!   - Andersen, L., & Piterbarg, V. (2010). *Interest Rate Modeling* (3 vols).
//!     Atlantic Financial Press. Volume 1, Chapters 2-3.
//!
//! - **Interpolation Methods**:
//!   - Hagan, P. S., & West, G. (2006). "Interpolation Methods for Curve Construction."
//!     *Applied Mathematical Finance*, 13(2), 89-129.
//!   - Hagan, P. S., & West, G. (2008). "Methods for Constructing a Yield Curve."
//!     *Wilmott Magazine*, May 2008.
//!
//! - **Industry Standards**:
//!   - OpenGamma (2013). "Interest Rate Instruments and Market Conventions Guide."
//!   - ISDA (2006). "2006 ISDA Definitions." Sections on discount factors and rates.

use super::common::{build_interp_input_error, split_points};
use crate::math::interp::{ExtrapolationPolicy, InterpStyle};
use crate::{
    dates::{Date, DayCount, DayCountCtx},
    market_data::traits::{Discounting, TermStructure},
    math::interp::types::Interp,
    types::CurveId,
};

/// Piece-wise discount factor curve supporting several interpolation styles.
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(try_from = "RawDiscountCurve", into = "RawDiscountCurve")
)]
pub struct DiscountCurve {
    id: CurveId,
    base: Date,
    /// Day-count basis used to convert dates → time for discounting.
    day_count: DayCount,
    /// Knot times in **years**.
    knots: Box<[f64]>,
    /// Discount factors (unitless).
    dfs: Box<[f64]>,
    interp: Interp,
    /// Interpolation style (stored for serialization and bumping)
    style: InterpStyle,
    /// Extrapolation policy (stored for serialization and bumping)
    extrapolation: ExtrapolationPolicy,
}

impl Clone for DiscountCurve {
    fn clone(&self) -> Self {
        // Rebuild the interpolator from raw data since Interp might not be Clone
        // This is expensive but necessary if we want Clone on the main struct
        // Alternatively, we could make Interp Clone, but for now we rebuild.
        // Actually, since we have style and extrapolation, we can rebuild.
        let interp = crate::market_data::term_structures::common::build_interp_input_error(
            self.style,
            self.knots.clone(),
            self.dfs.clone(),
            self.extrapolation,
        )
        .expect("Clone should not fail for valid curve");

        Self {
            id: self.id.clone(),
            base: self.base,
            day_count: self.day_count,
            knots: self.knots.clone(),
            dfs: self.dfs.clone(),
            interp,
            style: self.style,
            extrapolation: self.extrapolation,
        }
    }
}

/// Raw serializable state of DiscountCurve
#[cfg(feature = "serde")]
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawDiscountCurve {
    #[serde(flatten)]
    common_id: super::common::StateId,
    /// Base date
    pub base: Date,
    /// Day count convention for discount time basis
    #[serde(default = "default_day_count")]
    pub day_count: DayCount,
    #[serde(flatten)]
    points: super::common::StateKnotPoints,
    #[serde(flatten)]
    interp: super::common::StateInterp,
    /// Whether monotonic discount factors were required (deprecated, always true now)
    #[serde(default = "default_true")]
    pub require_monotonic: bool,
    /// Minimum forward rate floor (if set)
    #[serde(default)]
    pub min_forward_rate: Option<f64>,
    /// Whether non-monotonic DFs are allowed (dangerous override)
    #[serde(default)]
    pub allow_non_monotonic: bool,
}

#[cfg(feature = "serde")]
impl From<DiscountCurve> for RawDiscountCurve {
    fn from(curve: DiscountCurve) -> Self {
        let knot_points: Vec<(f64, f64)> = curve
            .knots
            .iter()
            .zip(curve.dfs.iter())
            .map(|(&t, &df)| (t, df))
            .collect();

        RawDiscountCurve {
            common_id: super::common::StateId {
                id: curve.id.to_string(),
            },
            base: curve.base,
            day_count: curve.day_count,
            points: super::common::StateKnotPoints { knot_points },
            interp: super::common::StateInterp {
                interp_style: curve.style,
                extrapolation: curve.extrapolation,
            },
            require_monotonic: true,
            min_forward_rate: None, // Can't recover from existing curves easily without storing it
            allow_non_monotonic: false,
        }
    }
}

#[cfg(feature = "serde")]
impl TryFrom<RawDiscountCurve> for DiscountCurve {
    type Error = crate::Error;

    fn try_from(state: RawDiscountCurve) -> crate::Result<Self> {
        let mut builder = DiscountCurve::builder(state.common_id.id)
            .base_date(state.base)
            .day_count(state.day_count)
            .knots(state.points.knot_points)
            .set_interp(state.interp.interp_style)
            .extrapolation(state.interp.extrapolation);

        // Handle legacy require_monotonic field (now always true by default)
        if !state.require_monotonic || state.allow_non_monotonic {
            builder = builder.allow_non_monotonic();
        }

        // Apply forward rate floor if specified
        if let Some(min_rate) = state.min_forward_rate {
            builder = builder.with_min_forward_rate(min_rate);
        }

        builder.build()
    }
}

#[cfg(feature = "serde")]
fn default_true() -> bool {
    true
}

#[cfg(feature = "serde")]
fn default_day_count() -> DayCount {
    DayCount::Act365F
}

impl DiscountCurve {
    /// Unique identifier of the curve.
    #[inline]
    pub fn id(&self) -> &CurveId {
        &self.id
    }

    /// Base (valuation) date of the curve.
    #[inline]
    pub fn base_date(&self) -> Date {
        self.base
    }

    /// Day-count basis used for discount time mapping.
    #[inline]
    pub fn day_count(&self) -> DayCount {
        self.day_count
    }

    /// Interpolation style used by this curve.
    #[inline]
    pub fn interp_style(&self) -> InterpStyle {
        self.style
    }

    /// Extrapolation policy used by this curve.
    #[inline]
    pub fn extrapolation(&self) -> ExtrapolationPolicy {
        self.extrapolation
    }

    /// Continuously-compounded zero rate.
    #[inline]
    pub fn zero(&self, t: f64) -> f64 {
        if t == 0.0 {
            return 0.0;
        }
        -self.df(t).ln() / t
    }

    /// Simple forward rate between `t1` and `t2`.
    ///
    /// The forward rate `f(t1, t2)` satisfies: `DF(t2) = DF(t1) * exp(-f * (t2-t1))`
    /// Therefore: `f = ln(DF(t1)/DF(t2)) / (t2-t1) = (z2*t2 - z1*t1) / (t2-t1)`
    /// where `z*t = -ln(DF)`.
    #[inline]
    pub fn forward(&self, t1: f64, t2: f64) -> f64 {
        debug_assert!(t2 > t1, "forward requires t2 > t1");
        let z1 = self.zero(t1) * t1;
        let z2 = self.zero(t2) * t2;
        (z2 - z1) / (t2 - t1)
    }

    /// Batch evaluation helper (parallel over `times` slice when compiled
    /// with the `parallel` feature).
    #[cfg_attr(docsrs, doc(cfg(feature = "parallel")))]
    pub fn df_batch(&self, times: &[f64]) -> Vec<f64> {
        #[cfg(feature = "parallel")]
        {
            use rayon::prelude::*;
            // Parallel iteration is required to be order-stable; results must be bit-identical
            // to the sequential path. We therefore only parallelize the map, preserving order.
            // ParallelIterator::collect already pre-allocates, so this is efficient.
            times.par_iter().map(|&t| self.df(t)).collect()
        }
        #[cfg(not(feature = "parallel"))]
        {
            let mut result = Vec::with_capacity(times.len());
            result.extend(times.iter().map(|&t| self.df(t)));
            result
        }
    }

    /// Convenience: discount factor on a specific date `date` given a curve and
    /// the curve base `base` and `day_count`.
    /// This is equivalent to `disc.df(t)` where `t` is the year fraction from `base` to `date`.
    #[inline]
    #[allow(clippy::manual_unwrap_or)]
    pub fn df_on_date(&self, date: Date, dc: crate::dates::DayCount) -> f64 {
        let t = if date == self.base {
            0.0
        } else {
            match dc.year_fraction(self.base, date, DayCountCtx::default()) {
                Ok(yf) => yf,
                Err(_) => 0.0,
            }
        };
        self.df(t)
    }

    /// Convenience: discount factor on a specific date using the curve's own day-count.
    #[inline]
    #[allow(clippy::manual_unwrap_or)]
    pub fn df_on_date_curve(&self, date: Date) -> f64 {
        let t = if date == self.base {
            0.0
        } else {
            match self
                .day_count
                .year_fraction(self.base, date, DayCountCtx::default())
            {
                Ok(yf) => yf,
                Err(_) => 0.0,
            }
        };
        self.df(t)
    }

    /// Static convenience: discount factor on a specific date given any discount curve.
    /// For backward compatibility with existing code.
    #[inline]
    #[allow(clippy::manual_unwrap_or)]
    pub fn df_on(
        disc: &dyn Discounting,
        base: Date,
        date: Date,
        dc: crate::dates::DayCount,
    ) -> f64 {
        let t = if date == base {
            0.0
        } else {
            match dc.year_fraction(base, date, DayCountCtx::default()) {
                Ok(yf) => yf,
                Err(_) => 0.0,
            }
        };
        disc.df(t)
    }

    /// Create a new curve with a parallel rate bump applied in basis points (fallible).
    ///
    /// Uses df_bumped(t) = df_original(t) * exp(-bump * t), where bump = bp / 10_000.
    ///
    /// Returns an error if the bumped curve violates validation constraints.
    pub fn try_with_parallel_bump(&self, bp: f64) -> crate::Result<Self> {
        let bump_rate = bp / 10_000.0;
        let bumped_points: Vec<(f64, f64)> = self
            .knots
            .iter()
            .zip(self.dfs.iter())
            .map(|(&t, &df)| (t, df * (-bump_rate * t).exp()))
            .collect();

        // Derive new ID with suffix
        let new_id = crate::market_data::bumps::id_bump_bp(self.id.as_str(), bp);

        // Rebuild preserving base date, interpolation, and extrapolation policies
        // Use allow_non_monotonic to handle negative rate environments
        DiscountCurve::builder(new_id)
            .base_date(self.base)
            .day_count(self.day_count)
            .knots(bumped_points)
            .set_interp(self.style)
            .extrapolation(self.extrapolation)
            .allow_non_monotonic() // Allow for negative rate environments
            .build()
    }

    /// Create a new curve with a triangular key-rate bump using explicit bucket neighbors.
    ///
    /// This is the market-standard key-rate DV01 implementation (per Tuckman/Fabozzi)
    /// where the triangular weight is defined by the **bucket grid**, not curve knots.
    /// This ensures that the sum of all bucketed DV01s equals the parallel DV01.
    ///
    /// # Mathematical Foundation
    ///
    /// For a zero rate bump δr applied with triangular weight w(t):
    /// ```text
    /// DF_bumped(t) = DF(t) × exp(-w(t) × δr × t)
    /// ```
    ///
    /// The triangular weight function for bucket at `target` with neighbors `prev` and `next`:
    /// - w(t) = 0                                    if t ≤ prev
    /// - w(t) = (t - prev) / (target - prev)        if prev < t ≤ target
    /// - w(t) = (next - t) / (next - target)        if target < t < next
    /// - w(t) = 0                                    if t ≥ next
    ///
    /// # Key Property: Unity Partition
    ///
    /// For any time t, the sum of all bucket weights equals 1.0:
    /// `Σᵢ wᵢ(t) = 1.0`
    ///
    /// This ensures: **sum of bucketed DV01 = parallel DV01**
    ///
    /// # Arguments
    /// * `prev_bucket` - Previous bucket time in years (use 0.0 for first bucket)
    /// * `target_bucket` - Target bucket time in years (peak of the triangle)
    /// * `next_bucket` - Next bucket time in years (use f64::INFINITY for last bucket)
    /// * `bp` - Bump size in basis points (100bp = 1%)
    ///
    /// # Returns
    /// A new discount curve with the triangular key-rate bump applied.
    ///
    /// # Errors
    /// Returns an error if the bumped curve violates validation constraints.
    ///
    /// # Examples
    /// ```rust,no_run
    /// use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    /// use time::{Date, Month};
    ///
    /// let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    /// let curve = DiscountCurve::builder("USD_OIS")
    ///     .base_date(base_date)
    ///     .knots(vec![(1.0, 0.98), (2.0, 0.96), (5.0, 0.90), (10.0, 0.80)])
    ///     .build()
    ///     .unwrap();
    ///
    /// // Apply 10bp bump at 5Y bucket with neighbors at 3Y and 7Y
    /// let bumped = curve.try_with_triangular_key_rate_bump_neighbors(3.0, 5.0, 7.0, 10.0).unwrap();
    /// ```
    pub fn try_with_triangular_key_rate_bump_neighbors(
        &self,
        prev_bucket: f64,
        target_bucket: f64,
        next_bucket: f64,
        bp: f64,
    ) -> crate::Result<Self> {
        if self.knots.len() < 2 {
            return self.try_with_parallel_bump(bp);
        }

        let bump_rate = bp / 10_000.0;
        let mut bumped_points: Vec<(f64, f64)> = Vec::with_capacity(self.knots.len());

        for (&knot_t, &df) in self.knots.iter().zip(self.dfs.iter()) {
            // Calculate triangular weight based on BUCKET grid (not curve knots!)
            // This is the key difference from the old implementation
            let weight = triangular_weight(knot_t, prev_bucket, target_bucket, next_bucket);

            // Apply weighted bump to zero rate:
            // r_bumped = r + w × δr
            // DF_bumped = exp(-r_bumped × t) = DF × exp(-w × δr × t)
            let new_df = df * (-bump_rate * weight * knot_t).exp();
            bumped_points.push((knot_t, new_df));
        }

        let new_id = crate::market_data::bumps::id_bump_bp(self.id.as_str(), bp);
        DiscountCurve::builder(new_id)
            .base_date(self.base)
            .day_count(self.day_count)
            .knots(bumped_points)
            .set_interp(self.style)
            .extrapolation(self.extrapolation)
            .allow_non_monotonic() // Allow for negative rate environments
            .build()
    }

    /// Roll the curve forward by a specified number of days.
    ///
    /// This creates a new curve with:
    /// - Base date advanced by `days`
    /// - Knot times shifted backwards (t' = t - dt_years)
    /// - Points with t' <= 0 are filtered out (expired)
    /// - Discount factors are preserved (no carry/theta adjustment)
    ///
    /// This is the "constant curves" or "pure roll-down" scenario where discount
    /// factors at each calendar date remain the same, but maturity times are
    /// re-measured from the new base date.
    ///
    /// # Arguments
    /// * `days` - Number of days to roll forward
    ///
    /// # Returns
    /// A new discount curve with updated base date and shifted knots.
    ///
    /// # Errors
    /// Returns an error if fewer than 2 knot points remain after filtering expired points.
    ///
    /// # Examples
    /// ```rust,no_run
    /// use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    /// use time::{Date, Month};
    ///
    /// let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    /// let curve = DiscountCurve::builder("USD_OIS")
    ///     .base_date(base_date)
    ///     .knots(vec![(0.5, 0.99), (1.0, 0.98), (2.0, 0.96), (5.0, 0.90)])
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
            .zip(self.dfs.iter())
            .filter_map(|(&t, &df)| {
                let new_t = t - dt_years;
                if new_t > 0.0 {
                    Some((new_t, df))
                } else {
                    None
                }
            })
            .collect();

        if rolled_points.len() < 2 {
            return Err(crate::error::InputError::TooFewPoints.into());
        }

        DiscountCurve::builder(self.id.clone())
            .base_date(new_base)
            .day_count(self.day_count)
            .knots(rolled_points)
            .set_interp(self.style)
            .extrapolation(self.extrapolation)
            .build()
    }

    /// Discount factor at time `t` (helper calling the underlying interpolator).
    #[inline]
    pub fn df(&self, t: f64) -> f64 {
        self.interp.interp(t)
    }

    /// Raw knot times (t) in **years** passed at construction.
    #[inline]
    pub fn knots(&self) -> &[f64] {
        &self.knots
    }

    /// Raw discount factors corresponding to each knot.
    #[inline]
    pub fn dfs(&self) -> &[f64] {
        &self.dfs
    }

    /// Builder entry-point.
    ///
    /// **Note:** Monotonic discount factor validation is enabled by default to ensure
    /// no-arbitrage conditions. Use `.allow_non_monotonic()` if you need to disable this
    /// validation (not recommended for production use).
    ///
    /// **Defaults:** MonotoneConvex interpolation with FlatForward extrapolation follow
    /// market-standard practices for no-arbitrage discount curves.
    pub fn builder(id: impl Into<CurveId>) -> DiscountCurveBuilder {
        DiscountCurveBuilder {
            id: id.into(),
            base: Date::from_calendar_date(1970, time::Month::January, 1)
                .expect("January 1, 1970 should always be valid"),
            day_count: DayCount::Act365F,
            points: Vec::new(),
            style: InterpStyle::MonotoneConvex,
            extrapolation: ExtrapolationPolicy::FlatForward,
            require_monotonic: true, // Enforced by default for no-arbitrage
            min_forward_rate: None,  // No floor by default
            allow_non_monotonic: false, // Strict validation by default
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
        forward_id: impl Into<CurveId>,
        tenor_years: f64,
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
                } else if t == 0.0 && dt > 0.0 && df == 1.0 && df_next > 0.0 {
                    // Special case: t=0 with DF(0)=1, use forward to next point
                    (-df_next.ln()) / t_next
                } else {
                    return Err(crate::error::InputError::Invalid.into());
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
                    return Err(crate::error::InputError::Invalid.into());
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
                    return Err(crate::error::InputError::Invalid.into());
                }
            };

            forward_rates.push((t, forward_rate));
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
        forward_id: impl Into<CurveId>,
        tenor_years: f64,
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
                } else if t == 0.0 && dt > 0.0 && df == 1.0 && df_next > 0.0 {
                    // Special case: t=0 with DF(0)=1, use forward to next point
                    (-df_next.ln()) / t_next
                } else {
                    return Err(crate::error::InputError::Invalid.into());
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
                    return Err(crate::error::InputError::Invalid.into());
                }
            } else {
                let t_prev = self.knots[i - 1];
                let df = self.dfs[i];
                let df_prev = self.dfs[i - 1];
                let dt = t - t_prev;

                if dt > 0.0 && df > 0.0 && df_prev > 0.0 {
                    (df_prev / df - 1.0) / dt
                } else {
                    return Err(crate::error::InputError::Invalid.into());
                }
            };

            forward_rates.push((t, forward_rate));
        }

        ForwardCurve::builder(forward_id, tenor_years)
            .base_date(self.base)
            .knots(forward_rates)
            .set_interp(interp_style)
            .build()
    }
}

/// Fluent builder for [`DiscountCurve`].
///
/// Typical usage chains `base_date`, `knots`, and `set_interp` (optional)
/// before calling [`DiscountCurveBuilder::build`].
///
/// # Examples
/// ```rust
/// use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
/// use finstack_core::math::interp::InterpStyle;
/// use finstack_core::dates::Date;
/// use time::Month;
///
/// let base = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");
/// let curve = DiscountCurve::builder("USD-OIS")
///     .base_date(base)
///     .knots([(0.0, 1.0), (5.0, 0.9)])
///     .set_interp(InterpStyle::Linear)
///     .build()
///     .expect("DiscountCurve builder should succeed");
/// assert!(curve.df(2.0) < 1.0);
/// ```
pub struct DiscountCurveBuilder {
    id: CurveId,
    base: Date,
    day_count: DayCount,
    points: Vec<(f64, f64)>, // (t, df)
    style: InterpStyle,
    extrapolation: ExtrapolationPolicy,
    require_monotonic: bool,       // Enforced by default for no-arbitrage
    min_forward_rate: Option<f64>, // Minimum allowed forward rate (e.g., -50bp = -0.005)
    allow_non_monotonic: bool,     // Override to disable monotonicity checks (use with caution)
}

impl DiscountCurveBuilder {
    /// Override the default **base date** (valuation date).
    pub fn base_date(mut self, d: Date) -> Self {
        self.base = d;
        self
    }
    /// Choose the day-count basis for discount time mapping.
    pub fn day_count(mut self, dc: DayCount) -> Self {
        self.day_count = dc;
        self
    }
    /// Supply knot points `(t, df)` where *t* is the year fraction and *df*
    /// the discount factor.
    pub fn knots<I>(mut self, pts: I) -> Self
    where
        I: IntoIterator<Item = (f64, f64)>,
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
    ///
    /// **Note:** Monotonicity is enforced by default. This method is kept for
    /// backward compatibility but has no effect since validation is now automatic.
    pub fn require_monotonic(mut self) -> Self {
        self.require_monotonic = true;
        self
    }

    /// Enforce comprehensive no-arbitrage checks on the discount curve.
    ///
    /// This enables:
    /// - Monotonic (strictly decreasing) discount factors
    /// - Forward rate floor at -50bp to prevent unrealistic negative rates
    ///
    /// # Example
    /// ```
    /// use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    /// use finstack_core::dates::Date;
    /// use time::Month;
    ///
    /// let base = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");
    /// let curve = DiscountCurve::builder("USD-OIS")
    ///     .base_date(base)
    ///     .knots([(0.0, 1.0), (1.0, 0.95), (5.0, 0.80)])
    ///     .enforce_no_arbitrage()
    ///     .build()
    ///     .expect("DiscountCurve builder should succeed");
    /// ```
    pub fn enforce_no_arbitrage(mut self) -> Self {
        self.require_monotonic = true;
        self.min_forward_rate = Some(-0.005); // -50bp floor
        self
    }

    /// Set a custom minimum forward rate (in decimal).
    ///
    /// Forward rates below this threshold will trigger a validation error.
    /// This prevents unrealistic negative rate scenarios that could indicate
    /// data errors or create arbitrage opportunities.
    ///
    /// # Example
    /// ```
    /// use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    /// use finstack_core::dates::Date;
    /// use time::Month;
    ///
    /// let base = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");
    /// let curve = DiscountCurve::builder("USD-OIS")
    ///     .base_date(base)
    ///     .knots([(0.0, 1.0), (1.0, 0.98), (5.0, 0.85)])
    ///     .with_min_forward_rate(-0.01)  // Floor at -100bp
    ///     .build()
    ///     .expect("DiscountCurve builder should succeed");
    /// ```
    pub fn with_min_forward_rate(mut self, min_rate: f64) -> Self {
        self.min_forward_rate = Some(min_rate);
        self
    }

    /// Allow non-monotonic discount factors (use with extreme caution).
    ///
    /// This disables the default monotonicity validation and should only be used
    /// in exceptional circumstances where you need to work with malformed market data.
    ///
    /// **Warning:** Non-monotonic discount factors create arbitrage opportunities
    /// and will produce incorrect pricing results. Only use this override if you
    /// understand the implications.
    pub fn allow_non_monotonic(mut self) -> Self {
        self.allow_non_monotonic = true;
        self.require_monotonic = false;
        self
    }

    /// Validate input and create the [`DiscountCurve`].
    pub fn build(self) -> crate::Result<DiscountCurve> {
        if self.points.len() < 2 {
            return Err(crate::error::InputError::TooFewPoints.into());
        }
        if self.points.iter().any(|&(_, df)| df <= 0.0) {
            return Err(crate::error::InputError::NonPositiveValue.into());
        }

        let (knots_vec, dfs_vec): (Vec<f64>, Vec<f64>) = split_points(self.points);
        crate::math::interp::utils::validate_knots(&knots_vec)
            .map_err(|_| crate::error::InputError::NonMonotonicKnots)?;

        // Enforce monotonicity by default (can be disabled via allow_non_monotonic)
        if self.require_monotonic && !self.allow_non_monotonic {
            validate_monotonic_df(&knots_vec, &dfs_vec)?;
        }

        // Validate forward rates if minimum is specified
        if let Some(min_fwd) = self.min_forward_rate {
            validate_forward_rates(&knots_vec, &dfs_vec, min_fwd)?;
        }

        let knots = knots_vec.into_boxed_slice();
        let dfs = dfs_vec.into_boxed_slice();

        let interp =
            build_interp_input_error(self.style, knots.clone(), dfs.clone(), self.extrapolation)?;

        Ok(DiscountCurve {
            id: self.id,
            base: self.base,
            day_count: self.day_count,
            knots,
            dfs,
            interp,
            style: self.style,
            extrapolation: self.extrapolation,
        })
    }
}

// -----------------------------------------------------------------------------
// Validation helper functions
// -----------------------------------------------------------------------------

/// Validate that discount factors are strictly decreasing (monotonic).
///
/// Non-monotonic discount factors violate no-arbitrage conditions and will
/// produce incorrect pricing results.
fn validate_monotonic_df(knots: &[f64], dfs: &[f64]) -> crate::Result<()> {
    for i in 1..dfs.len() {
        if dfs[i] >= dfs[i - 1] {
            return Err(crate::Error::Validation(format!(
                "Discount factors must be strictly decreasing: DF(t={:.4}) = {:.6} >= DF(t={:.4}) = {:.6}",
                knots[i], dfs[i], knots[i - 1], dfs[i - 1]
            )));
        }
    }
    Ok(())
}

/// Validate that implied forward rates are above a minimum threshold.
///
/// Forward rates are calculated as: f(t1, t2) = -ln(DF(t2)/DF(t1)) / (t2 - t1)
///
/// Excessively negative forward rates (below the specified floor) indicate
/// either data errors or unrealistic market conditions.
fn validate_forward_rates(knots: &[f64], dfs: &[f64], min_rate: f64) -> crate::Result<()> {
    for i in 1..knots.len() {
        let dt = knots[i] - knots[i - 1];
        if dt <= 0.0 {
            continue; // Skip degenerate intervals
        }

        // Calculate implied forward rate
        let fwd = -(dfs[i] / dfs[i - 1]).ln() / dt;

        if fwd < min_rate {
            return Err(crate::Error::Validation(format!(
                "Forward rate {:.4}% (decimal: {:.6}) between t={:.4} and t={:.4} is below minimum {:.4}% (decimal: {:.6}). \
                 This may indicate a data error or create arbitrage opportunities.",
                fwd * 100.0, fwd, knots[i - 1], knots[i], min_rate * 100.0, min_rate
            )));
        }
    }
    Ok(())
}

/// Calculate triangular weight for key-rate DV01.
///
/// Returns a weight in [0, 1] that peaks at `target` and linearly decays to 0
/// at `prev` and `next`. This function defines the weight based on the **bucket grid**,
/// ensuring that the sum of all bucket weights at any time t equals 1.0.
///
/// # Arguments
/// * `t` - The time at which to calculate the weight
/// * `prev` - Previous bucket time (0.0 for first bucket)
/// * `target` - Target bucket time (peak of the triangle)
/// * `next` - Next bucket time (f64::INFINITY for last bucket)
///
/// # Returns
/// Weight in [0, 1] representing the contribution of this bucket to the rate at time t.
#[inline]
fn triangular_weight(t: f64, prev: f64, target: f64, next: f64) -> f64 {
    if t <= prev {
        0.0
    } else if t <= target {
        // Rising edge: 0 at prev, 1 at target
        let denom = (target - prev).max(1e-10);
        (t - prev) / denom
    } else if next.is_infinite() {
        // Last bucket: flat weight of 1.0 beyond target
        1.0
    } else if t < next {
        // Falling edge: 1 at target, 0 at next
        let denom = (next - target).max(1e-10);
        (next - t) / denom
    } else {
        0.0
    }
}

// -----------------------------------------------------------------------------
// Minimal trait implementation for polymorphism where needed
// -----------------------------------------------------------------------------

impl Discounting for DiscountCurve {
    #[inline]
    fn base_date(&self) -> Date {
        self.base
    }

    #[inline]
    fn df(&self, t: f64) -> f64 {
        self.interp.interp(t)
    }
}

impl TermStructure for DiscountCurve {
    #[inline]
    fn id(&self) -> &CurveId {
        &self.id
    }
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

    fn sample_curve_linear() -> DiscountCurve {
        DiscountCurve::builder("USD-OIS")
            .base_date(
                Date::from_calendar_date(2025, time::Month::June, 30).expect("Valid test date"),
            )
            .knots([(0.0, 1.0), (1.0, 0.98), (2.0, 0.95)])
            .set_interp(InterpStyle::Linear)
            .build()
            .expect("DiscountCurve builder should succeed with valid test data")
    }

    fn sample_curve_log() -> DiscountCurve {
        DiscountCurve::builder("USD-OIS")
            .base_date(
                Date::from_calendar_date(2025, time::Month::June, 30).expect("Valid test date"),
            )
            .knots([(0.0, 1.0), (1.0, 0.98), (2.0, 0.95)])
            .set_interp(InterpStyle::LogLinear)
            .build()
            .expect("DiscountCurve builder should succeed with valid test data")
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

    #[test]
    fn tail_continuity_with_flatforward_extrapolation() {
        // Test that FlatForward extrapolation maintains continuous forward rates beyond last pillar
        let base =
            Date::from_calendar_date(2025, time::Month::January, 1).expect("Valid test date");
        let curve = DiscountCurve::builder("USD-OIS")
            .base_date(base)
            .knots([
                (0.0, 1.0),
                (0.25, 0.99),
                (1.0, 0.96),
                (5.0, 0.82),
                (10.0, 0.67),
            ])
            .set_interp(InterpStyle::MonotoneConvex)
            .extrapolation(ExtrapolationPolicy::FlatForward)
            .build()
            .expect("DiscountCurve builder should succeed with valid test data");

        // Check that tail extrapolation maintains reasonable forward behavior
        let df_at_10 = curve.df(10.0);
        let df_at_15 = curve.df(15.0);
        let df_at_20 = curve.df(20.0);

        // DFs should continue decreasing monotonically
        assert!(
            df_at_15 < df_at_10,
            "Tail DF should decrease: df(15)={:.6} >= df(10)={:.6}",
            df_at_15,
            df_at_10
        );
        assert!(
            df_at_20 < df_at_15,
            "Tail DF should decrease: df(20)={:.6} >= df(15)={:.6}",
            df_at_20,
            df_at_15
        );

        // Calculate forward rates in tail - should be stable with FlatForward
        let fwd_10_15 = curve.forward(10.0, 15.0);
        let fwd_15_20 = curve.forward(15.0, 20.0);

        // Forward rates should be continuous (within reasonable tolerance for finite differences)
        let fwd_diff = (fwd_15_20 - fwd_10_15).abs();
        assert!(
            fwd_diff < 0.01,
            "Forward rate should be stable in tail with FlatForward: fwd(10-15)={:.4}%, fwd(15-20)={:.4}%",
            fwd_10_15 * 100.0, fwd_15_20 * 100.0
        );
    }

    #[test]
    fn default_uses_monotone_convex_and_flatforward() {
        // Verify new market-standard defaults are in place
        let base =
            Date::from_calendar_date(2025, time::Month::January, 1).expect("Valid test date");
        let curve = DiscountCurve::builder("TEST")
            .base_date(base)
            .knots([(0.0, 1.0), (1.0, 0.98), (5.0, 0.90)])
            .build()
            .expect("DiscountCurve builder should succeed with valid test data");

        // Defaults should be MonotoneConvex + FlatForward (verified by checking tail DF behavior)
        // With FlatForward, the tail should extrapolate using the forward rate at the last segment
        let df_at_last = curve.df(5.0);
        let df_beyond = curve.df(10.0);

        // Discount factors should continue decreasing (or remain stable in extreme cases)
        // The key is that FlatForward doesn't produce zero or increasing DFs
        assert!(
            df_beyond <= df_at_last,
            "Tail DF should not increase: df(10)={:.6}, df(5)={:.6}",
            df_beyond,
            df_at_last
        );

        // Forward rate in tail should be non-negative for this curve
        let zero_at_last = curve.zero(5.0);
        assert!(
            zero_at_last > 0.0,
            "Zero rate should be positive for decreasing DF curve"
        );
    }

    #[test]
    fn df_to_fwd_preserves_low_forwards_no_clamp() {
        // Test that DF→FWD conversion works with very small forwards
        // (The old code would clamp to [0, 0.5])
        let base =
            Date::from_calendar_date(2025, time::Month::January, 1).expect("Valid test date");

        // Build a nearly-flat curve implying very low forwards
        // All DFs very close to 1.0 implies near-zero interest rates
        let curve = DiscountCurve::builder("EUR-OIS")
            .base_date(base)
            .knots([
                (0.0, 1.0),
                (1.0, 0.9998), // ~2bp zero rate
                (5.0, 0.9990), // ~2bp zero rate
            ])
            .set_interp(InterpStyle::Linear)
            .build()
            .expect("DiscountCurve builder should succeed with valid test data");

        // Convert to forward curve - should succeed
        let fwd_curve = curve.to_forward_curve("EUR-FWD", 0.25);

        // Should succeed
        assert!(fwd_curve.is_ok(), "DF→FWD should work with low forwards");
        let fwd = fwd_curve.expect("Forward curve conversion should succeed in test");

        // All forwards should be very small (< 1%)
        let rates: Vec<f64> = fwd.knots().iter().map(|&t| fwd.rate(t)).collect();

        for (i, &rate) in rates.iter().enumerate() {
            assert!(
                rate.abs() < 0.01,
                "Forward rate {} should be very small: {:.4}%",
                i,
                rate * 100.0
            );
        }

        // Verify no clamping occurred - rates should accurately reflect the DF curve
        // The first forward should be approximately (1.0/0.9998 - 1)/1 ≈ 0.0002 = 0.02%
        assert!(
            rates[0] >= 0.0 && rates[0] < 0.001,
            "First forward should be near 0.02%: actual {:.4}%",
            rates[0] * 100.0
        );
    }
}
