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
//! - **MonotoneConvex**: Smooth, no-arbitrage (Hagan-West algorithm)
//! - **CubicHermite**: Shape-preserving cubic (requires monotone input for no-arb)
//! - **PiecewiseQuadraticForward**: Smooth forward curve (C²), commonly used for display
//!
//! # Use Cases
//!
//! - **Bond pricing**: Discount future coupons and principal
//! - **Swap valuation**: Mark-to-market fixed and floating legs
//! - **Option pricing**: Discount expected payoffs
//! - **Risk metrics**: DV01, duration, convexity calculation
//!
//! # Extrapolation Behavior and Limitations
//!
//! The curve supports two extrapolation policies via [`ExtrapolationPolicy`]:
//!
//! - **`FlatZero`** (conservative): Returns the discount factor at the boundary knot.
//!   Beyond the last knot, this implies zero forward rates. Use for risk management
//!   where you want to avoid assumptions about unobserved rates.
//!
//! - **`FlatForward`** (default): Extends the curve using the forward rate at the
//!   boundary. This is the market standard for production curves.
//!
//! ## Warning: Ultra-Long Tenor Extrapolation
//!
//! When extrapolating significantly beyond the last curve knot (e.g., pricing a 50Y
//! instrument from a 10Y curve), be aware of the following limitations:
//!
//! 1. **Model uncertainty**: Extrapolated forward rates are not market-implied.
//!    For tenors 2× beyond the last knot, consider the extrapolation unreliable.
//!
//! 2. **Risk sensitivity**: Greeks computed in extrapolated regions may be
//!    misleading. The curve has no sensitivity to rates beyond its last pillar.
//!
//! 3. **Regulatory considerations**: Basel III/IV and Solvency II have specific
//!    requirements for ultra-long rate extrapolation (Smith-Wilson, UFR methods).
//!    This implementation does not include regulatory extrapolation methods.
//!
//! **Best practice**: If you frequently price instruments beyond your curve's last
//! pillar, either:
//! - Extend the curve with appropriate long-dated instruments (e.g., 30Y, 50Y swaps)
//! - Use a regulatory-compliant extrapolation method for insurance/pension valuations
//! - Apply explicit haircuts or uncertainty bands to extrapolated values
//!
//! ## Example
//! ```rust
//! use finstack_core::market_data::term_structures::DiscountCurve;
//! use finstack_core::dates::Date;
//! use time::Month;
//! # use finstack_core::math::interp::InterpStyle;
//!
//! let curve = DiscountCurve::builder("USD-OIS")
//!     .base_date(Date::from_calendar_date(2025, Month::January, 1).expect("Valid date"))
//!     .knots([(0.0, 1.0), (5.0, 0.9)])
//!     .interp(InterpStyle::MonotoneConvex)
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

use super::common::{build_interp_input_error, roll_knots, split_points, triangular_weight};
use crate::math::interp::{ExtrapolationPolicy, InterpStyle};
use crate::{
    dates::{Date, DayCount, DayCountCtx},
    market_data::traits::{Discounting, TermStructure},
    math::interp::types::Interp,
    types::CurveId,
};

/// Default minimum forward rate tenor in years (~30 seconds).
///
/// Very short tenors cause precision degradation in the formula (z2 - z1) / (t2 - t1)
/// due to catastrophic cancellation when z1*t1 ≈ z2*t2.
///
/// This constant can be overridden via [`DiscountCurveBuilder::min_forward_tenor`].
pub const DEFAULT_MIN_FORWARD_TENOR: f64 = 1e-6;

/// Piece-wise discount factor curve supporting several interpolation styles.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(try_from = "RawDiscountCurve", into = "RawDiscountCurve")]
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
    /// Minimum tenor for forward rate calculations (configurable)
    min_forward_tenor: f64,
}

/// Raw serializable state of DiscountCurve
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
    /// Minimum forward rate floor (if set)
    #[serde(default)]
    pub min_forward_rate: Option<f64>,
    /// Whether non-monotonic DFs are allowed (dangerous override)
    #[serde(default)]
    pub allow_non_monotonic: bool,
    /// Minimum tenor for forward rate calculations
    #[serde(default = "default_min_forward_tenor")]
    pub min_forward_tenor: f64,
}

fn default_min_forward_tenor() -> f64 {
    DEFAULT_MIN_FORWARD_TENOR
}

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
            min_forward_rate: None, // Can't recover from existing curves easily without storing it
            allow_non_monotonic: false,
            min_forward_tenor: curve.min_forward_tenor,
        }
    }
}

impl TryFrom<RawDiscountCurve> for DiscountCurve {
    type Error = crate::Error;

    fn try_from(state: RawDiscountCurve) -> crate::Result<Self> {
        let mut builder = DiscountCurve::builder(state.common_id.id)
            .base_date(state.base)
            .day_count(state.day_count)
            .knots(state.points.knot_points)
            .interp(state.interp.interp_style)
            .extrapolation(state.interp.extrapolation)
            .min_forward_tenor(state.min_forward_tenor);

        if state.allow_non_monotonic {
            builder = builder.allow_non_monotonic();
        }

        // Apply forward rate floor if specified
        if let Some(min_rate) = state.min_forward_rate {
            builder = builder.min_forward_rate(min_rate);
        }

        builder.build()
    }
}

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

    /// Number of knot points in the curve.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.knots.len()
    }

    /// Returns `true` if the curve has no knot points.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.knots.is_empty()
    }

    /// Continuously-compounded zero rate.
    ///
    /// Formula: `r_cc = -ln(DF) / t`
    #[must_use]
    #[inline]
    pub fn zero(&self, t: f64) -> f64 {
        if t == 0.0 {
            return 0.0;
        }
        -self.df(t).ln() / t
    }

    /// Annually-compounded zero rate (bond equivalent yield convention).
    ///
    /// This is the rate quoted for most bonds and is commonly used by
    /// Bloomberg for displaying zero rates.
    ///
    /// Formula: `r_annual = DF^(-1/t) - 1`
    ///
    /// # Example
    ///
    /// ```
    /// use finstack_core::market_data::term_structures::DiscountCurve;
    /// use finstack_core::dates::Date;
    /// use time::Month;
    ///
    /// let curve = DiscountCurve::builder("USD-OIS")
    ///     .base_date(Date::from_calendar_date(2025, Month::January, 1).expect("Valid date"))
    ///     .knots([(0.0, 1.0), (1.0, 0.95), (5.0, 0.80)])
    ///     .build()
    ///     .expect("DiscountCurve should build");
    ///
    /// // At 1Y, DF = 0.95, so annual rate = 0.95^(-1) - 1 ≈ 5.26%
    /// let annual_rate = curve.zero_annual(1.0);
    /// assert!((annual_rate - 0.0526).abs() < 0.001);
    /// ```
    #[inline]
    pub fn zero_annual(&self, t: f64) -> f64 {
        if t == 0.0 {
            return 0.0;
        }
        self.df(t).powf(-1.0 / t) - 1.0
    }

    /// Periodically-compounded zero rate with `n` compounding periods per year.
    ///
    /// Common values for `n`:
    /// - 1: Annual (same as `zero_annual`)
    /// - 2: Semi-annual (US Treasury convention)
    /// - 4: Quarterly
    /// - 12: Monthly
    ///
    /// Formula: `r_periodic = n * (DF^(-1/(n*t)) - 1)`
    ///
    /// # Example
    ///
    /// ```
    /// use finstack_core::market_data::term_structures::DiscountCurve;
    /// use finstack_core::dates::Date;
    /// use time::Month;
    ///
    /// let curve = DiscountCurve::builder("USD-OIS")
    ///     .base_date(Date::from_calendar_date(2025, Month::January, 1).expect("Valid date"))
    ///     .knots([(0.0, 1.0), (1.0, 0.95), (5.0, 0.80)])
    ///     .build()
    ///     .expect("DiscountCurve should build");
    ///
    /// // Semi-annual compounded rate at 1Y
    /// let semi_annual_rate = curve.zero_periodic(1.0, 2);
    /// // Annual rate should equal periodic with n=1
    /// let annual_via_periodic = curve.zero_periodic(1.0, 1);
    /// assert!((curve.zero_annual(1.0) - annual_via_periodic).abs() < 1e-12);
    /// ```
    #[inline]
    pub fn zero_periodic(&self, t: f64, n: u32) -> f64 {
        if t == 0.0 || n == 0 {
            return 0.0;
        }
        let n_f = n as f64;
        n_f * (self.df(t).powf(-1.0 / (n_f * t)) - 1.0)
    }

    /// Simple interest (money market) zero rate.
    ///
    /// Returns the simple interest rate (no compounding) implied by the discount factor.
    /// This is the standard convention for money market instruments with tenors under 1 year,
    /// including deposits, CDs, T-bills, and short-term rate fixings.
    ///
    /// # Compounding Convention
    ///
    /// **Simple interest means NO compounding.** Interest accrues linearly:
    /// - Future Value = Principal × (1 + rate × time)
    /// - This differs from annually compounded rates which compound once per year
    ///
    /// # Formula
    ///
    /// ```text
    /// r_simple = (1/DF - 1) / t
    /// ```
    ///
    /// Derived from the simple interest present value formula: `DF(t) = 1 / (1 + r × t)`
    ///
    /// # Market Standards
    ///
    /// Simple interest is the market convention for:
    /// - **USD**: SOFR, Fed Funds, T-bills, CDs, deposits (< 1Y tenor)
    /// - **EUR**: €STR, Euribor fixings
    /// - **GBP**: SONIA
    /// - **Most markets**: Interbank deposits, repo rates
    ///
    /// **Day count**: Typically paired with ACT/360 (USD, EUR) or ACT/365F (GBP).
    ///
    /// # Bloomberg Equivalent
    ///
    /// This matches Bloomberg's simple interest zero rate output when compounding
    /// is set to "Simple" in curve display screens (e.g., SWPM, SWCV).
    ///
    /// # Comparison with Other Rate Conventions
    ///
    /// For a given discount factor at time t:
    /// - `zero()` returns continuously compounded rate: `r_cc = -ln(DF) / t`
    /// - `zero_annual()` returns annually compounded: `r_annual = DF^(-1/t) - 1`
    /// - `zero_simple()` returns simple interest: `r_simple = (1/DF - 1) / t`
    ///
    /// For positive rates and t > 0: `r_simple > r_annual > r_cc`
    ///
    /// # Example
    ///
    /// ```
    /// use finstack_core::market_data::term_structures::DiscountCurve;
    /// use finstack_core::dates::Date;
    /// use time::Month;
    ///
    /// let curve = DiscountCurve::builder("USD-OIS")
    ///     .base_date(Date::from_calendar_date(2025, Month::January, 1).expect("Valid date"))
    ///     .knots([(0.0, 1.0), (0.25, 0.99), (1.0, 0.95)])
    ///     .build()
    ///     .expect("DiscountCurve should build");
    ///
    /// // At 3M (0.25Y), DF = 0.99, so simple rate = (1/0.99 - 1) / 0.25 ≈ 4.04%
    /// let simple_rate = curve.zero_simple(0.25);
    /// assert!((simple_rate - 0.0404).abs() < 0.001);
    /// ```
    #[inline]
    pub fn zero_simple(&self, t: f64) -> f64 {
        if t == 0.0 {
            return 0.0;
        }
        (1.0 / self.df(t) - 1.0) / t
    }

    /// Simple forward rate between `t1` and `t2`.
    /// Continuously-compounded forward rate between `t1` and `t2`.
    ///
    /// The forward rate `f(t1, t2)` satisfies: `DF(t2) = DF(t1) * exp(-f * (t2-t1))`
    /// Therefore: `f = ln(DF(t1)/DF(t2)) / (t2-t1) = (z2*t2 - z1*t1) / (t2-t1)`
    /// where `z*t = -ln(DF)`.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - `t1` or `t2` is non-finite
    /// - `t2 <= t1`
    /// - `(t2 - t1) < min_forward_tenor` (configurable, default ~30 seconds) to avoid
    ///   numerical precision issues from catastrophic cancellation
    ///
    /// # Configuring Minimum Tenor
    ///
    /// The minimum forward tenor can be customized when building the curve:
    /// ```rust,no_run
    /// use finstack_core::market_data::term_structures::DiscountCurve;
    /// # use time::macros::date;
    /// # fn main() -> finstack_core::Result<()> {
    /// let curve = DiscountCurve::builder("USD")
    ///     .base_date(date!(2025-01-01))
    ///     .knots([(0.0, 1.0), (1.0, 0.95)])
    ///     .min_forward_tenor(1e-8)  // Allow very short tenors
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    #[must_use = "computed forward rate should not be discarded"]
    pub fn forward(&self, t1: f64, t2: f64) -> crate::Result<f64> {
        if !t1.is_finite() || !t2.is_finite() || t2 <= t1 {
            return Err(crate::error::InputError::Invalid.into());
        }
        if (t2 - t1) < self.min_forward_tenor {
            return Err(crate::error::InputError::Invalid.into());
        }
        let z1 = self.zero(t1) * t1;
        let z2 = self.zero(t2) * t2;
        Ok((z2 - z1) / (t2 - t1))
    }

    /// Get the minimum forward tenor configured for this curve.
    #[inline]
    pub fn min_forward_tenor(&self) -> f64 {
        self.min_forward_tenor
    }

    /// Batch evaluation of discount factors for multiple times.
    #[inline]
    #[must_use]
    pub fn df_batch(&self, times: &[f64]) -> Vec<f64> {
        times.iter().map(|&t| self.df(t)).collect()
    }

    /// Fallible: discount factor on a specific date `date` using explicit day-count `dc`.
    #[inline]
    #[must_use = "computed discount factor should not be discarded"]
    pub fn df_on_date(&self, date: Date, dc: crate::dates::DayCount) -> crate::Result<f64> {
        let t = if date == self.base {
            0.0
        } else {
            dc.year_fraction(self.base, date, DayCountCtx::default())?
        };
        Ok(self.df(t))
    }

    /// Fallible: discount factor on a specific date `date` using the curve's day-count.
    #[inline]
    #[must_use = "computed discount factor should not be discarded"]
    pub fn df_on_date_curve(&self, date: Date) -> crate::Result<f64> {
        let t = self.year_fraction_to(date)?;
        Ok(self.df(t))
    }

    /// Fallible: discount factor from `from` to `to` using the curve's day-count.
    ///
    /// This is the canonical helper for the common "relative DF" pattern:
    /// `DF(from→to) = DF(0→to) / DF(0→from)`.
    ///
    /// Works for both forward and backward date order. Returns `1.0` when
    /// `from == to`.
    #[inline]
    #[must_use = "computed discount factor should not be discarded"]
    pub fn df_between_dates(&self, from: Date, to: Date) -> crate::Result<f64> {
        if from == to {
            return Ok(1.0);
        }

        let df_from = self.df_on_date_curve(from)?;
        if !df_from.is_finite() || df_from <= 0.0 {
            return Err(crate::Error::Validation(format!(
                "Invalid discount factor on 'from' date ({from}): {df_from}"
            )));
        }

        let df_to = self.df_on_date_curve(to)?;
        if !df_to.is_finite() || df_to <= 0.0 {
            return Err(crate::Error::Validation(format!(
                "Invalid discount factor on 'to' date ({to}): {df_to}"
            )));
        }
        Ok(df_to / df_from)
    }

    /// Returns the zero rate for a given date with specified compounding convention.
    ///
    /// This is the unified method for obtaining zero rates under any compounding convention.
    /// It replaces the individual `zero_on_date`, `zero_annual_on_date`, `zero_periodic_on_date`,
    /// and `zero_simple_on_date` methods.
    ///
    /// # Arguments
    /// * `date` - Target date for the zero rate
    /// * `compounding` - Compounding convention (Continuous, Annual, Periodic(n), Simple)
    ///
    /// # Mathematical Formulas
    ///
    /// For a discount factor `df` and time `t`:
    ///
    /// | Compounding | Formula | Use Case |
    /// |-------------|---------|----------|
    /// | Continuous | r = -ln(df) / t | Internal calculations, curve building |
    /// | Annual | r = df^(-1/t) - 1 | Bond markets (UK, Europe) |
    /// | Periodic(n) | r = n × (df^(-1/(n×t)) - 1) | US Treasuries (n=2), corporates |
    /// | Simple | r = (1/df - 1) / t | Money market (< 1Y) |
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_core::market_data::term_structures::DiscountCurve;
    /// use finstack_core::math::Compounding;
    /// use finstack_core::dates::Date;
    /// use time::Month;
    ///
    /// let anchor = Date::from_calendar_date(2024, Month::January, 2).unwrap();
    /// // Build a flat 5% curve (df at 1Y = exp(-0.05 * 1) ≈ 0.9512)
    /// let curve = DiscountCurve::builder("USD-OIS")
    ///     .base_date(anchor)
    ///     .knots([(0.0, 1.0), (1.0, (-0.05_f64).exp())])
    ///     .build()?;
    /// let target = Date::from_calendar_date(2025, Month::January, 2).unwrap();
    ///
    /// // Continuous rate (default for internal calculations)
    /// let r_cont = curve.zero_rate_on_date(target, Compounding::Continuous)?;
    ///
    /// // Annual rate (for European bonds)
    /// let r_ann = curve.zero_rate_on_date(target, Compounding::Annual)?;
    ///
    /// // Semi-annual rate (for US Treasuries)
    /// let r_semi = curve.zero_rate_on_date(target, Compounding::SEMI_ANNUAL)?;
    ///
    /// // Simple rate (for money market)
    /// let r_simple = curve.zero_rate_on_date(target, Compounding::Simple)?;
    /// # Ok::<(), finstack_core::Error>(())
    /// ```
    ///
    /// # Errors
    /// Returns an error if the date is before the anchor.
    #[inline]
    #[must_use = "computed zero rate should not be discarded"]
    pub fn zero_rate_on_date(
        &self,
        date: Date,
        compounding: crate::math::Compounding,
    ) -> crate::Result<f64> {
        let t = self.year_fraction_to(date)?;
        Ok(self.zero_rate(t, compounding))
    }

    /// Returns the zero rate for a given year fraction with specified compounding.
    ///
    /// This is the unified method for obtaining zero rates under any compounding convention.
    ///
    /// # Arguments
    /// * `t` - Year fraction from the anchor date
    /// * `compounding` - Compounding convention (Continuous, Annual, Periodic(n), Simple)
    ///
    /// # Edge Cases
    /// - For t = 0, all compounding conventions return 0.0 (instantaneous rate is undefined)
    #[inline]
    #[must_use]
    pub fn zero_rate(&self, t: f64, compounding: crate::math::Compounding) -> f64 {
        use crate::math::Compounding;
        match compounding {
            Compounding::Continuous => self.zero(t),
            Compounding::Annual => self.zero_annual(t),
            Compounding::Periodic(n) => self.zero_periodic(t, n.get()),
            Compounding::Simple => self.zero_simple(t),
        }
    }

    /// Simple forward rate between two dates using the curve's day-count.
    ///
    /// This is equivalent to `curve.forward(t1, t2)` where `t1` and `t2` are
    /// year fractions from base date using the curve's day-count convention.
    ///
    /// # Errors
    ///
    /// Returns an error if year fraction calculation fails or if the forward
    /// rate calculation fails.
    #[inline]
    #[must_use = "computed forward rate should not be discarded"]
    pub fn forward_on_dates(&self, d1: Date, d2: Date) -> crate::Result<f64> {
        let t1 = self.year_fraction_to(d1)?;
        let t2 = self.year_fraction_to(d2)?;
        self.forward(t1, t2)
    }

    /// Helper: compute year fraction from base date to target date using curve's day-count.
    #[inline]
    fn year_fraction_to(&self, date: Date) -> crate::Result<f64> {
        super::common::year_fraction_to(self.base, date, self.day_count)
    }

    /// Create a new curve with a parallel rate bump applied in basis points (fallible).
    ///
    /// Uses df_bumped(t) = df_original(t) * exp(-bump * t), where bump = bp / 10_000.
    ///
    /// Returns an error if the bumped curve violates validation constraints.
    pub fn with_parallel_bump(&self, bp: f64) -> crate::Result<Self> {
        let bump_rate = bp / 10_000.0;
        let bumped_points: Vec<(f64, f64)> = self
            .knots
            .iter()
            .zip(self.dfs.iter())
            .map(|(&t, &df)| (t, df * (-bump_rate * t).exp()))
            .collect();

        // Derive new ID with suffix
        let new_id = crate::market_data::bumps::id_bump_bp(self.id.as_str(), bp);

        // Rebuild preserving base date, interpolation, extrapolation, and forward tenor policies
        // Use allow_non_monotonic to handle negative rate environments
        DiscountCurve::builder(new_id)
            .base_date(self.base)
            .day_count(self.day_count)
            .knots(bumped_points)
            .interp(self.style)
            .extrapolation(self.extrapolation)
            .min_forward_tenor(self.min_forward_tenor)
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
    /// use finstack_core::market_data::term_structures::DiscountCurve;
    /// use time::macros::date;
    /// # fn main() -> finstack_core::Result<()> {
    ///
    /// let base_date = date!(2025 - 01 - 01);
    /// let curve = DiscountCurve::builder("USD_OIS")
    ///     .base_date(base_date)
    ///     .knots(vec![(1.0, 0.98), (2.0, 0.96), (5.0, 0.90), (10.0, 0.80)])
    ///     .build()
    ///     ?;
    ///
    /// // Apply 10bp bump at 5Y bucket with neighbors at 3Y and 7Y
    /// let bumped = curve.with_triangular_key_rate_bump_neighbors(3.0, 5.0, 7.0, 10.0)?;
    /// # let _ = bumped;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_triangular_key_rate_bump_neighbors(
        &self,
        prev_bucket: f64,
        target_bucket: f64,
        next_bucket: f64,
        bp: f64,
    ) -> crate::Result<Self> {
        if self.knots.len() < 2 {
            return self.with_parallel_bump(bp);
        }

        // Validate bucket grid ordering.
        // next_bucket may be +∞ for the last bucket.
        if !prev_bucket.is_finite()
            || !target_bucket.is_finite()
            || !(next_bucket.is_finite() || next_bucket.is_infinite())
            || prev_bucket >= target_bucket
            || (!next_bucket.is_infinite() && target_bucket >= next_bucket)
        {
            return Err(crate::error::InputError::Invalid.into());
        }

        let bump_rate = bp / 10_000.0;
        let bumped_points: Vec<(f64, f64)> = self
            .knots
            .iter()
            .zip(self.dfs.iter())
            .map(|(&knot_t, &df)| {
                // Triangular weight based on BUCKET grid (not curve knots!)
                let weight = triangular_weight(knot_t, prev_bucket, target_bucket, next_bucket);
                // r_bumped = r + w × δr
                // DF_bumped = exp(-r_bumped × t) = DF × exp(-w × δr × t)
                (knot_t, df * (-bump_rate * weight * knot_t).exp())
            })
            .collect();

        let new_id = crate::market_data::bumps::id_bump_bp(self.id.as_str(), bp);
        DiscountCurve::builder(new_id)
            .base_date(self.base)
            .day_count(self.day_count)
            .knots(bumped_points)
            .interp(self.style)
            .extrapolation(self.extrapolation)
            .min_forward_tenor(self.min_forward_tenor)
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
    /// use finstack_core::market_data::term_structures::DiscountCurve;
    /// use time::macros::date;
    /// # fn main() -> finstack_core::Result<()> {
    ///
    /// let base_date = date!(2025 - 01 - 01);
    /// let curve = DiscountCurve::builder("USD_OIS")
    ///     .base_date(base_date)
    ///     .knots(vec![(0.5, 0.99), (1.0, 0.98), (2.0, 0.96), (5.0, 0.90)])
    ///     .build()
    ///     ?;
    ///
    /// // Roll 6 months forward - the 0.5Y point expires
    /// let rolled = curve.roll_forward(182)?;
    /// assert!(rolled.knots().len() < curve.knots().len());
    /// # Ok(())
    /// # }
    /// ```
    pub fn roll_forward(&self, days: i64) -> crate::Result<Self> {
        let new_base = self.base + time::Duration::days(days);
        let dt_years = self
            .day_count
            .year_fraction(self.base, new_base, DayCountCtx::default())?;

        let rolled_points = roll_knots(&self.knots, &self.dfs, dt_years);

        if rolled_points.len() < 2 {
            return Err(crate::error::InputError::TooFewPoints.into());
        }

        DiscountCurve::builder(self.id.clone())
            .base_date(new_base)
            .day_count(self.day_count)
            .knots(rolled_points)
            .interp(self.style)
            .extrapolation(self.extrapolation)
            .min_forward_tenor(self.min_forward_tenor)
            .build()
    }

    /// Discount factor at time `t` (helper calling the underlying interpolator).
    #[must_use]
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
        // Epoch date - unwrap_or provides defensive fallback for infallible operation
        let base =
            Date::from_calendar_date(1970, time::Month::January, 1).unwrap_or(time::Date::MIN);
        DiscountCurveBuilder {
            id: id.into(),
            base,
            day_count: DayCount::Act365F,
            points: Vec::new(),
            style: InterpStyle::MonotoneConvex,
            extrapolation: ExtrapolationPolicy::FlatForward,
            min_forward_rate: None,     // No floor by default
            allow_non_monotonic: false, // Strict validation by default
            min_forward_tenor: DEFAULT_MIN_FORWARD_TENOR, // Default ~30 seconds
        }
    }

    /// Create a forward curve from this discount curve.
    ///
    /// For single-curve bootstrapping, this creates a forward curve from the
    /// discount factors using the formula:
    /// f(t) = -d/dt[ln(DF(t))] = -1/DF(t) * dDF/dt
    ///
    /// For discrete points, we use: f(t) ≈ (DF(t) - DF(t+dt)) / (dt * DF(t+dt))
    ///
    /// # Arguments
    ///
    /// * `forward_id` - Identifier for the resulting forward curve
    /// * `tenor_years` - Tenor of the forward rate in years
    /// * `interp_style` - Optional interpolation style; defaults to `Linear` if `None`
    pub fn to_forward_curve(
        &self,
        forward_id: impl Into<CurveId>,
        tenor_years: f64,
        interp_style: Option<InterpStyle>,
    ) -> crate::Result<super::forward_curve::ForwardCurve> {
        use super::forward_curve::ForwardCurve;

        let style = interp_style.unwrap_or(InterpStyle::Linear);

        // Calculate forward rates at each knot point
        let mut forward_rates = Vec::with_capacity(self.knots.len());

        // Ensure we have enough points
        if self.knots.len() < 2 {
            return Err(crate::error::InputError::TooFewPoints.into());
        }

        for i in 0..self.knots.len() {
            let t = self.knots[i];
            // Allow exact float comparison for well-known sentinel values (t=0, DF(0)=1).
            #[allow(clippy::float_cmp)]
            let forward_rate = if i == 0 {
                // First point: use next point for forward difference
                let t_next = self.knots[1];
                let df = self.dfs[0];
                let df_next = self.dfs[1];
                let dt = t_next - t;

                if dt > 0.0 && df_next > 0.0 && df > 0.0 {
                    (df / df_next).ln() / dt
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
                    (df_prev / df_next).ln() / dt
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
                    (df_prev / df).ln() / dt
                } else {
                    return Err(crate::error::InputError::Invalid.into());
                }
            };

            forward_rates.push((t, forward_rate));
        }

        // Build forward curve with the specified interpolation style
        ForwardCurve::builder(forward_id, tenor_years)
            .base_date(self.base)
            .knots(forward_rates)
            .interp(style)
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
/// use finstack_core::market_data::term_structures::DiscountCurve;
/// use finstack_core::math::interp::InterpStyle;
/// use finstack_core::dates::Date;
/// use time::Month;
///
/// let base = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");
/// let curve = DiscountCurve::builder("USD-OIS")
///     .base_date(base)
///     .knots([(0.0, 1.0), (5.0, 0.9)])
///     .interp(InterpStyle::Linear)
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
    min_forward_rate: Option<f64>, // Minimum allowed forward rate (e.g., -50bp = -0.005)
    allow_non_monotonic: bool,     // Override to disable monotonicity checks (use with caution)
    min_forward_tenor: f64,        // Minimum tenor for forward rate calculations
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
    pub fn interp(mut self, style: InterpStyle) -> Self {
        self.style = style;
        self
    }

    /// Set the extrapolation policy for out-of-bounds evaluation.
    pub fn extrapolation(mut self, policy: ExtrapolationPolicy) -> Self {
        self.extrapolation = policy;
        self
    }

    /// Enforce comprehensive no-arbitrage checks on the discount curve.
    ///
    /// This enables:
    /// - Monotonic (non-increasing) discount factors
    /// - Forward rate floor at -50bp to prevent unrealistic negative rates
    ///
    /// # Example
    /// ```
    /// use finstack_core::market_data::term_structures::DiscountCurve;
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
    /// use finstack_core::market_data::term_structures::DiscountCurve;
    /// use finstack_core::dates::Date;
    /// use time::Month;
    ///
    /// let base = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");
    /// let curve = DiscountCurve::builder("USD-OIS")
    ///     .base_date(base)
    ///     .knots([(0.0, 1.0), (1.0, 0.98), (5.0, 0.85)])
    ///     .min_forward_rate(-0.01)  // Floor at -100bp
    ///     .build()
    ///     .expect("DiscountCurve builder should succeed");
    /// ```
    pub fn min_forward_rate(mut self, min_rate: f64) -> Self {
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
    ///
    /// For negative rate environments, prefer [`allow_non_monotonic_with_floor`](Self::allow_non_monotonic_with_floor)
    /// which adds a -5% safety floor on implied forward rates.
    pub fn allow_non_monotonic(mut self) -> Self {
        self.allow_non_monotonic = true;
        self
    }

    /// Allow non-monotonic discount factors with a safety floor on forward rates.
    ///
    /// This is the recommended way to handle negative rate environments.
    /// Disables monotonicity validation but sets a -5% floor on implied forward
    /// rates to catch data errors.
    ///
    /// The -5% floor is a conservative bound that accommodates historical negative
    /// rate regimes (e.g., ECB deposit facility at -0.50%) while catching obviously
    /// erroneous data.
    ///
    /// For full override without any floor, use [`allow_non_monotonic`](Self::allow_non_monotonic)
    /// or chain with `.min_forward_rate(f64::NEG_INFINITY)`.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use finstack_core::market_data::term_structures::DiscountCurve;
    /// # use time::macros::date;
    /// # fn main() -> finstack_core::Result<()> {
    /// let curve = DiscountCurve::builder("EUR-OIS")
    ///     .base_date(date!(2025-01-01))
    ///     .knots([(0.0, 1.0), (1.0, 1.002), (5.0, 0.99)])
    ///     .allow_non_monotonic_with_floor()
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn allow_non_monotonic_with_floor(mut self) -> Self {
        self.allow_non_monotonic = true;
        // Set a default forward rate floor when monotonicity is relaxed,
        // unless the user has already set one explicitly.
        // -5% is a reasonable floor that allows negative rate environments
        // (ECB deposit facility) while catching data errors.
        if self.min_forward_rate.is_none() {
            self.min_forward_rate = Some(-0.05);
        }
        self
    }

    /// Set a custom minimum tenor for forward rate calculations.
    ///
    /// The forward rate calculation `f(t1, t2) = (z2*t2 - z1*t1) / (t2 - t1)` suffers
    /// from catastrophic cancellation when `(t2 - t1)` is very small. This threshold
    /// prevents such precision issues.
    ///
    /// # Default
    ///
    /// The default value is [`DEFAULT_MIN_FORWARD_TENOR`] (~30 seconds or 1e-6 years).
    ///
    /// # Use Cases
    ///
    /// - Set to a smaller value (e.g., `1e-8`) for high-frequency intraday operations
    /// - Set to a larger value (e.g., `1e-4`) for daily curve operations with coarse data
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use finstack_core::market_data::term_structures::DiscountCurve;
    /// # use time::macros::date;
    /// # fn main() -> finstack_core::Result<()> {
    /// let curve = DiscountCurve::builder("USD")
    ///     .base_date(date!(2025-01-01))
    ///     .knots([(0.0, 1.0), (1.0, 0.95)])
    ///     .min_forward_tenor(1e-8)  // Allow sub-second tenors
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn min_forward_tenor(mut self, tenor: f64) -> Self {
        self.min_forward_tenor = tenor;
        self
    }

    /// Build the curve with minimal validation for solver use.
    ///
    /// This method skips monotonicity validation and forward rate checks, providing
    /// faster curve construction for iterative solving where the curve is temporary.
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    /// - At least 2 knot points are provided
    /// - All discount factors are positive
    /// - Knots are sorted in ascending order
    ///
    /// This is an internal optimization for calibration solvers.
    /// For general use, prefer [`build`] which includes full validation.
    #[doc(hidden)]
    pub fn build_for_solver(self) -> crate::Result<DiscountCurve> {
        if self.points.len() < 2 {
            return Err(crate::error::InputError::TooFewPoints.into());
        }

        // Minimal validation: check for positive DFs only
        if self.points.iter().any(|&(_, df)| df <= 0.0) {
            return Err(crate::error::InputError::NonPositiveValue.into());
        }

        let (knots_vec, dfs_vec): (Vec<f64>, Vec<f64>) = split_points(self.points);

        // Skip knot sorting validation - assume caller provides sorted input
        // Skip monotonicity validation - solver may explore non-monotonic states
        // Skip forward rate validation - temporary curves during solving

        let knots = knots_vec.into_boxed_slice();
        let dfs = dfs_vec.into_boxed_slice();

        let interp = build_interp_input_error(
            self.style,
            knots.clone(),
            dfs.clone(),
            self.extrapolation,
            true, // Discount curves defer DF validation to domain-specific helpers
        )?;

        Ok(DiscountCurve {
            id: self.id,
            base: self.base,
            day_count: self.day_count,
            knots,
            dfs,
            interp,
            style: self.style,
            extrapolation: self.extrapolation,
            min_forward_tenor: self.min_forward_tenor,
        })
    }

    /// Validate input and create the [`DiscountCurve`].
    ///
    /// If the first knot time is `> 0.0`, automatically prepends `(0.0, 1.0)` to
    /// ensure the round-trip invariant `DF(0) = 1.0` (ISDA/QuantLib standard).
    pub fn build(mut self) -> crate::Result<DiscountCurve> {
        // Auto-enforce DF(0) = 1.0: if no knot at t=0, prepend one.
        // This matches QuantLib/Bloomberg convention where DF(0) = 1.0 always.
        if !self.points.is_empty() {
            self.points
                .sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
            let first_t = self.points[0].0;
            if first_t > 1e-14 {
                // First knot is after t=0, auto-prepend DF(0)=1.0
                self.points.insert(0, (0.0, 1.0));
            }
        }

        if self.points.len() < 2 {
            return Err(crate::error::InputError::TooFewPoints.into());
        }
        if self.points.iter().any(|&(_, df)| df <= 0.0) {
            return Err(crate::error::InputError::NonPositiveValue.into());
        }

        let (knots_vec, dfs_vec): (Vec<f64>, Vec<f64>) = split_points(self.points);
        crate::math::interp::utils::validate_knots(&knots_vec)?;

        // Enforce monotonicity by default (can be disabled via allow_non_monotonic)
        if !self.allow_non_monotonic {
            validate_monotonic_df(&knots_vec, &dfs_vec)?;
        } else if self.style == InterpStyle::MonotoneConvex {
            // MonotoneConvex interpolation requires arbitrage-free DF input
            // (positive and non-increasing). If the caller disables monotonic validation,
            // we still need to fail loudly here instead of producing a misleading error
            // from the interpolation layer.
            validate_monotone_convex_compatible_df(&knots_vec, &dfs_vec)?;
        }

        // Validate forward rates if minimum is specified
        if let Some(min_fwd) = self.min_forward_rate {
            validate_forward_rates(&knots_vec, &dfs_vec, min_fwd)?;
        }

        let knots = knots_vec.into_boxed_slice();
        let dfs = dfs_vec.into_boxed_slice();

        let interp = build_interp_input_error(
            self.style,
            knots.clone(),
            dfs.clone(),
            self.extrapolation,
            true, // Discount curves defer DF validation to domain-specific helpers
        )?;

        Ok(DiscountCurve {
            id: self.id,
            base: self.base,
            day_count: self.day_count,
            knots,
            dfs,
            interp,
            style: self.style,
            extrapolation: self.extrapolation,
            min_forward_tenor: self.min_forward_tenor,
        })
    }
}

// -----------------------------------------------------------------------------
// Validation helper functions
// -----------------------------------------------------------------------------

/// Validate that discount factors are monotone (non-increasing) within tolerance.
///
/// Non-monotonic discount factors violate no-arbitrage conditions and will
/// produce incorrect pricing results.
fn validate_monotonic_df(knots: &[f64], dfs: &[f64]) -> crate::Result<()> {
    for (knot_pair, df_pair) in knots.windows(2).zip(dfs.windows(2)) {
        let (prev, curr) = (df_pair[0], df_pair[1]);
        let tol = 1e-14 * prev.abs().max(1.0);
        if curr > prev + tol {
            return Err(crate::Error::Validation(format!(
                "Discount factors must be non-increasing: DF(t={:.4}) = {:.12} > DF(t={:.4}) = {:.12}",
                knot_pair[1], curr, knot_pair[0], prev
            )));
        }
    }
    Ok(())
}

/// Validate DF input compatibility with MonotoneConvex interpolation.
///
/// MonotoneConvex (Hagan–West) requires a positive, non-increasing DF term structure.
fn validate_monotone_convex_compatible_df(knots: &[f64], dfs: &[f64]) -> crate::Result<()> {
    for (knot_pair, df_pair) in knots.windows(2).zip(dfs.windows(2)) {
        let (prev, curr) = (df_pair[0], df_pair[1]);
        let tol = 1e-14 * prev.abs().max(1.0);
        if curr > prev + tol {
            return Err(crate::Error::Validation(format!(
                "InterpStyle::MonotoneConvex requires non-increasing discount factors. \
                 Found DF(t={:.4}) = {:.12} > DF(t={:.4}) = {:.12}. \
                 Use LogLinear/Linear (and allow_non_monotonic) for negative-rate / increasing-DF inputs, \
                 or fix the input curve.",
                knot_pair[1], curr, knot_pair[0], prev
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
    for (knot_pair, df_pair) in knots.windows(2).zip(dfs.windows(2)) {
        let dt = knot_pair[1] - knot_pair[0];
        if dt <= 0.0 {
            continue; // Skip degenerate intervals
        }

        // Calculate implied forward rate
        let fwd = -(df_pair[1] / df_pair[0]).ln() / dt;

        if fwd < min_rate {
            return Err(crate::Error::Validation(format!(
                "Forward rate {:.4}% (decimal: {:.6}) between t={:.4} and t={:.4} is below minimum {:.4}% (decimal: {:.6}). \
                 This may indicate a data error or create arbitrage opportunities.",
                fwd * 100.0, fwd, knot_pair[0], knot_pair[1], min_rate * 100.0, min_rate
            )));
        }
    }
    Ok(())
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

    #[inline]
    fn day_count(&self) -> DayCount {
        self.day_count
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
#[allow(clippy::expect_used, clippy::panic, clippy::indexing_slicing)]
mod tests {
    use super::*;

    fn sample_curve_linear() -> DiscountCurve {
        DiscountCurve::builder("USD-OIS")
            .base_date(
                Date::from_calendar_date(2025, time::Month::June, 30).expect("Valid test date"),
            )
            .knots([(0.0, 1.0), (1.0, 0.98), (2.0, 0.95)])
            .interp(InterpStyle::Linear)
            .build()
            .expect("DiscountCurve builder should succeed with valid test data")
    }

    fn sample_curve_log() -> DiscountCurve {
        DiscountCurve::builder("USD-OIS")
            .base_date(
                Date::from_calendar_date(2025, time::Month::June, 30).expect("Valid test date"),
            )
            .knots([(0.0, 1.0), (1.0, 0.98), (2.0, 0.95)])
            .interp(InterpStyle::LogLinear)
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
    fn auto_sorts_and_prepends_df0() {
        // Previously this rejected unsorted knots; now the builder sorts and
        // auto-prepends (0, 1.0) so the curve builds successfully.
        let res = DiscountCurve::builder("USD")
            .knots([(1.0, 0.99), (0.5, 0.995)])
            .build();
        assert!(
            res.is_ok(),
            "Builder should auto-sort and prepend DF(0)=1.0"
        );
        let curve = res.expect("curve should build");
        // Should have 3 knots: (0.0, 1.0), (0.5, 0.995), (1.0, 0.99)
        assert_eq!(curve.knots().len(), 3);
        assert!((curve.df(0.0) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn logdf_interpolator_behaves() {
        let yc = sample_curve_log();
        let mid = yc.df(0.5);
        assert!(mid < 1.0 && mid > 0.98);
    }

    #[test]
    fn df_between_dates_returns_one_when_equal() {
        let yc = sample_curve_linear();
        let d = yc.base_date();
        let df = yc
            .df_between_dates(d, d)
            .expect("df_between_dates should be defined for equal dates");
        assert_eq!(df, 1.0);
    }

    #[test]
    fn df_between_dates_matches_on_date_ratio() {
        let yc = sample_curve_linear();
        let base = yc.base_date();
        let from = base + time::Duration::days(180);
        let to = base + time::Duration::days(540);

        let df_from = yc
            .df_on_date_curve(from)
            .expect("df(from) should be defined");
        let df_to = yc.df_on_date_curve(to).expect("df(to) should be defined");
        let expected = df_to / df_from;

        let actual = yc
            .df_between_dates(from, to)
            .expect("df_between_dates should be defined");
        assert!(
            (actual - expected).abs() < 1e-12,
            "Expected df_between_dates({from}->{to})={expected}, got {actual}"
        );
    }

    #[test]
    fn df_between_dates_validates_non_finite_and_non_positive() {
        let base = Date::from_calendar_date(2025, time::Month::June, 30).expect("Valid test date");
        // Builders should reject non-finite or non-positive discount factors at construction time.
        let bad_nan = DiscountCurve::builder("USD-OIS")
            .base_date(base)
            .knots([(0.0, 1.0), (1.0, f64::NAN), (2.0, 0.95)])
            .interp(InterpStyle::Linear)
            .build();
        assert!(bad_nan.is_err());

        let bad_zero = DiscountCurve::builder("USD-OIS")
            .base_date(base)
            .knots([(0.0, 1.0), (1.0, 0.0), (2.0, 0.95)])
            .interp(InterpStyle::Linear)
            .build();
        assert!(bad_zero.is_err());

        let bad_neg = DiscountCurve::builder("USD-OIS")
            .base_date(base)
            .knots([(0.0, 1.0), (1.0, -0.01), (2.0, 0.95)])
            .interp(InterpStyle::Linear)
            .build();
        assert!(bad_neg.is_err());
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
            .interp(InterpStyle::MonotoneConvex)
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
        let fwd_10_15 = curve
            .forward(10.0, 15.0)
            .expect("forward(10,15) should succeed");
        let fwd_15_20 = curve
            .forward(15.0, 20.0)
            .expect("forward(15,20) should succeed");

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
            .interp(InterpStyle::Linear)
            .build()
            .expect("DiscountCurve builder should succeed with valid test data");

        // Convert to forward curve - should succeed
        let fwd_curve = curve.to_forward_curve("EUR-FWD", 0.25, None);

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

    #[test]
    fn zero_simple_matches_bloomberg_formula() {
        // Test the simple interest zero rate formula: r = (1/DF - 1) / t
        let base =
            Date::from_calendar_date(2025, time::Month::January, 1).expect("Valid test date");
        let curve = DiscountCurve::builder("USD-MM")
            .base_date(base)
            .knots([
                (0.0, 1.0),
                (0.25, 0.99), // 3M: DF = 0.99
                (0.5, 0.975), // 6M: DF = 0.975
                (1.0, 0.95),  // 1Y: DF = 0.95
            ])
            .interp(InterpStyle::Linear)
            .build()
            .expect("DiscountCurve builder should succeed with valid test data");

        // At 3M (0.25Y), DF = 0.99
        // Simple rate = (1/0.99 - 1) / 0.25 = 0.01010101... / 0.25 ≈ 4.04%
        let simple_3m = curve.zero_simple(0.25);
        let expected_3m = (1.0 / 0.99 - 1.0) / 0.25;
        assert!(
            (simple_3m - expected_3m).abs() < 1e-12,
            "3M simple rate mismatch: {} vs {}",
            simple_3m,
            expected_3m
        );

        // At 6M (0.5Y), DF = 0.975
        // Simple rate = (1/0.975 - 1) / 0.5 ≈ 5.13%
        let simple_6m = curve.zero_simple(0.5);
        let expected_6m = (1.0 / 0.975 - 1.0) / 0.5;
        assert!(
            (simple_6m - expected_6m).abs() < 1e-12,
            "6M simple rate mismatch: {} vs {}",
            simple_6m,
            expected_6m
        );

        // At 1Y, DF = 0.95
        // Simple rate = (1/0.95 - 1) / 1.0 ≈ 5.26%
        let simple_1y = curve.zero_simple(1.0);
        let expected_1y = (1.0 / 0.95 - 1.0) / 1.0;
        assert!(
            (simple_1y - expected_1y).abs() < 1e-12,
            "1Y simple rate mismatch: {} vs {}",
            simple_1y,
            expected_1y
        );

        // Test edge case: t = 0 should return 0
        assert!(
            curve.zero_simple(0.0).abs() < 1e-15,
            "Simple rate at t=0 should be 0"
        );
    }

    #[test]
    fn zero_simple_vs_other_compounding_conventions() {
        // For typical market rates, simple > annual > continuous for positive rates
        // This is because more frequent compounding yields more interest
        //
        // Note: At t=1, simple and annual rates are mathematically identical:
        //   simple:  r = (1/DF - 1) / t  →  at t=1: r = 1/DF - 1
        //   annual:  r = DF^(-1/t) - 1   →  at t=1: r = 1/DF - 1
        // So we test at t=2 where they differ.
        let base =
            Date::from_calendar_date(2025, time::Month::January, 1).expect("Valid test date");
        let curve = DiscountCurve::builder("USD-TEST")
            .base_date(base)
            .knots([
                (0.0, 1.0),
                (1.0, 0.95),
                (2.0, 0.90), // DF at 2Y
                (5.0, 0.75),
            ])
            .interp(InterpStyle::Linear)
            .build()
            .expect("DiscountCurve builder should succeed with valid test data");

        // At 2Y maturity (where simple ≠ annual)
        let t = 2.0;
        let simple = curve.zero_simple(t);
        let annual = curve.zero_annual(t);
        let continuous = curve.zero(t);

        // For positive rates and t > 1: simple > annual > continuous
        // (less compounding means higher quoted rate for same DF)
        assert!(
            simple > annual,
            "Simple rate ({:.6}) should be > annual rate ({:.6}) at t={}",
            simple,
            annual,
            t
        );
        assert!(
            annual > continuous,
            "Annual rate ({:.6}) should be > continuous rate ({:.6})",
            annual,
            continuous
        );

        // Verify round-trip: all rates should produce the same DF
        let df = curve.df(t);
        let df_from_simple = 1.0 / (1.0 + simple * t);
        let df_from_annual = (1.0 + annual).powf(-t);
        let df_from_continuous = (-continuous * t).exp();

        assert!(
            (df - df_from_simple).abs() < 1e-12,
            "Simple rate should reproduce DF"
        );
        assert!(
            (df - df_from_annual).abs() < 1e-12,
            "Annual rate should reproduce DF"
        );
        assert!(
            (df - df_from_continuous).abs() < 1e-12,
            "Continuous rate should reproduce DF"
        );

        // Also verify at t=1 that simple equals annual (mathematical identity)
        let simple_1y = curve.zero_simple(1.0);
        let annual_1y = curve.zero_annual(1.0);
        assert!(
            (simple_1y - annual_1y).abs() < 1e-12,
            "At t=1, simple ({:.6}) should equal annual ({:.6})",
            simple_1y,
            annual_1y
        );
    }

    #[test]
    fn monotone_validation_allows_flat_segments() {
        let base =
            Date::from_calendar_date(2025, time::Month::January, 1).expect("Valid test date");
        let curve = DiscountCurve::builder("FLAT-SEGMENT")
            .base_date(base)
            .knots([
                (0.0, 1.0),
                (1.0, 1.0),  // flat (zero rate) segment
                (2.0, 0.99), // then decreasing
            ])
            .interp(InterpStyle::LogLinear)
            .build();
        assert!(curve.is_ok(), "Flat DF segments should be allowed");
    }

    #[test]
    fn allow_non_monotonic_with_floor_sets_default_floor() {
        let base =
            Date::from_calendar_date(2025, time::Month::January, 1).expect("Valid test date");
        // Slightly increasing DF (negative rates) but within -5% floor
        let curve = DiscountCurve::builder("EUR-OIS")
            .base_date(base)
            .knots([(0.0, 1.0), (1.0, 1.002), (5.0, 0.99)])
            .interp(InterpStyle::Linear)
            .allow_non_monotonic_with_floor()
            .build();
        assert!(
            curve.is_ok(),
            "Mild negative rates should pass with -5% floor"
        );
    }

    #[test]
    fn allow_non_monotonic_with_floor_rejects_wildly_negative_forwards() {
        let base =
            Date::from_calendar_date(2025, time::Month::January, 1).expect("Valid test date");
        // Drastically increasing DFs implying > -5% forward rates
        let curve = DiscountCurve::builder("BAD-DF")
            .base_date(base)
            .knots([(0.0, 1.0), (1.0, 1.10), (5.0, 1.50)])
            .interp(InterpStyle::Linear)
            .allow_non_monotonic_with_floor()
            .build();
        assert!(
            curve.is_err(),
            "Wildly negative forward rates should be rejected by -5% floor"
        );
    }

    #[test]
    fn allow_non_monotonic_with_floor_respects_explicit_floor() {
        let base =
            Date::from_calendar_date(2025, time::Month::January, 1).expect("Valid test date");
        // Set a custom floor first, then call allow_non_monotonic_with_floor
        // The custom floor should be preserved (not overwritten by -5%)
        let curve = DiscountCurve::builder("EUR-OIS")
            .base_date(base)
            .knots([(0.0, 1.0), (1.0, 1.002), (5.0, 0.99)])
            .interp(InterpStyle::Linear)
            .min_forward_rate(-0.01) // Stricter floor: -1%
            .allow_non_monotonic_with_floor()
            .build();
        // The -1% floor should be preserved since it was set before
        assert!(
            curve.is_ok(),
            "Custom floor should be preserved by allow_non_monotonic_with_floor"
        );
    }

    #[test]
    fn allow_non_monotonic_with_monotone_convex_fails_loudly_on_increasing_df() {
        let base =
            Date::from_calendar_date(2025, time::Month::January, 1).expect("Valid test date");
        let res = DiscountCurve::builder("BAD-DF")
            .base_date(base)
            .knots([(0.0, 1.0), (1.0, 1.01), (2.0, 1.02)])
            .interp(InterpStyle::MonotoneConvex)
            .allow_non_monotonic()
            .build();
        assert!(res.is_err());
        let msg = res.expect_err("expected err").to_string();
        assert!(
            msg.contains("InterpStyle::MonotoneConvex requires non-increasing discount factors"),
            "Unexpected error message: {msg}"
        );
    }

    #[test]
    fn roll_forward_uses_curve_day_count() {
        let base =
            Date::from_calendar_date(2025, time::Month::January, 1).expect("Valid test date");
        // Include t=0 knot explicitly so auto-prepend doesn't change the count
        let curve = DiscountCurve::builder("ROLL")
            .base_date(base)
            .day_count(DayCount::Act360)
            .knots([(0.0, 1.0), (0.05, 0.999), (0.15, 0.998), (0.30, 0.995)])
            .interp(InterpStyle::Linear)
            .build()
            .expect("DiscountCurve builder should succeed with valid test data");

        // Roll 36 days => Act/360 year fraction = 0.1
        let rolled = curve.roll_forward(36).expect("roll_forward should succeed");
        let ks = rolled.knots();
        // After rolling 0.1: t=0.0 and t=0.05 expire, leaving t=0.15-0.1=0.05 and t=0.30-0.1=0.20
        // The rolled curve then auto-prepends (0, DF(0)=1.0), giving 3 knots
        assert_eq!(
            ks.len(),
            3,
            "Should have auto-prepended (0,1) + 2 surviving knots"
        );
        assert!((ks[1] - 0.05).abs() < 1e-12, "Expected 0.15-0.10=0.05");
        assert!((ks[2] - 0.20).abs() < 1e-12, "Expected 0.30-0.10=0.20");
    }
}
