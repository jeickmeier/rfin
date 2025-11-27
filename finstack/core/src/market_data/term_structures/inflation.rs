//! Inflation curves for CPI/RPI modeling and inflation-linked securities.
//!
//! Represents expected future inflation as a term structure of CPI (Consumer
//! Price Index) levels. Used for pricing inflation-linked bonds (TIPS, linkers),
//! inflation swaps, and inflation caps/floors.
//!
//! # Financial Concept
//!
//! The inflation curve maps time to expected CPI index levels:
//! ```text
//! I(t) = CPI index level at time t
//! π(t₁, t₂) = [I(t₂) / I(t₁)]^(1/(t₂-t₁)) - 1  (annualized inflation rate)
//! ```
//!
//! # Market Construction
//!
//! Inflation curves are bootstrapped from:
//! - **Zero-coupon inflation swaps** (ZCIS): Market standard for breakeven inflation
//! - **Inflation-linked bonds**: TIPS (US), Linkers (UK), OATi (France)
//! - **Year-on-year swaps** (YoY): Annual inflation rate swaps
//! - **Seasonality adjustments**: Monthly patterns in published CPI
//!
//! # Curve Types
//!
//! - **Real inflation**: Market expectations from inflation swaps
//! - **Breakeven inflation**: Implied from TIPS vs nominal bond spreads
//! - **Seasonal inflation**: Incorporates month-to-month volatility
//!
//! # Interpolation
//!
//! LogLinear interpolation is standard (constant inflation rate between knots):
//! ```text
//! I(t) = I(t₁) * exp(π * (t - t₁))
//! ```
//!
//! # Use Cases
//!
//! - **TIPS pricing**: Inflation-adjusted principal and coupons
//! - **Inflation swap valuation**: Zero-coupon and year-on-year structures
//! - **Real rate extraction**: Separate nominal rates into real + inflation
//! - **Pension liability valuation**: Inflation-linked obligations
//!
//! # Examples
//!
//! ```rust
//! use finstack_core::market_data::term_structures::inflation::InflationCurve;
//! # use finstack_core::math::interp::InterpStyle;
//! let ic = InflationCurve::builder("US-CPI")
//!     .base_cpi(300.0)
//!     .knots([(0.0, 300.0), (5.0, 327.0)])
//!     .set_interp(InterpStyle::LogLinear)
//!     .build()
//!     .expect("InflationCurve builder should succeed");
//! assert!(ic.inflation_rate(0.0, 5.0) > 0.0);
//! ```
//!
//! # References
//!
//! - **Inflation Markets**:
//!   - Deacon, M., Derry, A., & Mirfendereski, D. (2004). *Inflation-Indexed Securities:
//!     Bonds, Swaps and Other Derivatives* (2nd ed.). Wiley Finance.
//!   - Kerkhof, J. (2005). "Inflation Derivatives Explained." *Journal of Derivatives
//!     Accounting*, 2(1), 1-19.
//!
//! - **Curve Construction**:
//!   - Hurd, M., & Relleen, J. (2006). "Estimating the Inflation Risk Premium."
//!     Bank of England Quarterly Bulletin, Q2 2006.
//!   - Fleckenstein, M., Longstaff, F. A., & Lustig, H. (2017). "Deflation Risk."
//!     *Review of Financial Studies*, 30(8), 2719-2760.

use super::common::{build_interp, split_points};
use crate::math::interp::{ExtrapolationPolicy, InterpStyle};
use crate::{
    error::InputError, market_data::traits::TermStructure, math::interp::types::Interp,
    types::CurveId,
};

/// Inflation curve representing CPI/RPI index levels over time.
///
/// Stores CPI index levels at knot times and interpolates between them using
/// the specified interpolation method. LogLinear interpolation (constant
/// inflation rate) is the market standard.
///
/// # Mathematical Representation
///
/// ```text
/// I(t) = CPI index level at time t
/// π(t₁, t₂) = annualized inflation rate from t₁ to t₂
///           = [I(t₂) / I(t₁)]^(1/(t₂-t₁)) - 1
/// ```
///
/// # Use Cases
///
/// - TIPS (Treasury Inflation-Protected Securities) pricing
/// - Inflation swap valuation (zero-coupon and year-on-year)
/// - Real rate curve construction (nominal - breakeven = real)
/// - Pension liability modeling with inflation indexation
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(try_from = "RawInflationCurve", into = "RawInflationCurve")
)]
pub struct InflationCurve {
    id: CurveId,
    base_cpi: f64,
    /// Knot times in **years**.
    knots: Box<[f64]>,
    /// CPI index levels at each knot.
    cpi_levels: Box<[f64]>,
    interp: Interp,
}

impl Clone for InflationCurve {
    fn clone(&self) -> Self {
        let interp = super::common::build_interp(
            self.interp.style(),
            self.knots.clone(),
            self.cpi_levels.clone(),
            self.interp.extrapolation(),
        )
        .expect("Clone should not fail for valid curve");

        Self {
            id: self.id.clone(),
            base_cpi: self.base_cpi,
            knots: self.knots.clone(),
            cpi_levels: self.cpi_levels.clone(),
            interp,
        }
    }
}

/// Raw serializable state of an InflationCurve
#[cfg(feature = "serde")]
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawInflationCurve {
    #[serde(flatten)]
    common_id: super::common::StateId,
    /// Base CPI level at t=0
    pub base_cpi: f64,
    #[serde(flatten)]
    points: super::common::StateKnotPoints,
    #[serde(flatten)]
    interp: super::common::StateInterp,
}

#[cfg(feature = "serde")]
impl From<InflationCurve> for RawInflationCurve {
    fn from(curve: InflationCurve) -> Self {
        let knot_points: Vec<(f64, f64)> = curve
            .knots
            .iter()
            .copied()
            .zip(curve.cpi_levels.iter().copied())
            .collect();

        RawInflationCurve {
            common_id: super::common::StateId {
                id: curve.id.to_string(),
            },
            base_cpi: curve.base_cpi,
            points: super::common::StateKnotPoints { knot_points },
            interp: super::common::StateInterp {
                interp_style: curve.interp.style(),
                extrapolation: curve.interp.extrapolation(),
            },
        }
    }
}

#[cfg(feature = "serde")]
impl TryFrom<RawInflationCurve> for InflationCurve {
    type Error = crate::Error;

    fn try_from(state: RawInflationCurve) -> crate::Result<Self> {
        InflationCurve::builder(state.common_id.id)
            .base_cpi(state.base_cpi)
            .knots(state.points.knot_points)
            .set_interp(state.interp.interp_style)
            .build()
    }
}

impl InflationCurve {
    /// Start building an inflation curve with identifier `id`.
    ///
    /// # Examples
    /// ```rust
    /// use finstack_core::market_data::term_structures::inflation::InflationCurve;
    /// use finstack_core::math::interp::InterpStyle;
    ///
    /// let curve = InflationCurve::builder("US-CPI")
    ///     .base_cpi(300.0)
    ///     .knots([(0.0, 300.0), (5.0, 325.0)])
    ///     .set_interp(InterpStyle::LogLinear)
    ///     .build()
    ///     .expect("InflationCurve builder should succeed");
    /// assert!(curve.inflation_rate(0.0, 5.0) > 0.0);
    /// ```
    pub fn builder(id: impl Into<CurveId>) -> InflationCurveBuilder {
        InflationCurveBuilder {
            id: id.into(),
            base_cpi: 100.0,
            points: Vec::new(),
            style: InterpStyle::LogLinear,
        }
    }

    /// CPI level at time `t` (years).
    pub fn cpi(&self, t: f64) -> f64 {
        if t <= 0.0 {
            return self.base_cpi;
        }
        self.interp.interp(t)
    }

    /// Simple annualised inflation rate between `t1` and `t2`.
    pub fn inflation_rate(&self, t1: f64, t2: f64) -> f64 {
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
    pub fn knots(&self) -> &[f64] {
        &self.knots
    }

    /// CPI levels provided at each knot.
    #[inline]
    pub fn cpi_levels(&self) -> &[f64] {
        &self.cpi_levels
    }

    /// Base CPI level at t = 0.
    #[inline]
    pub fn base_cpi(&self) -> f64 {
        self.base_cpi
    }

    /// Roll the curve forward by a specified number of days.
    ///
    /// This creates a new curve with:
    /// - Knot times shifted backwards (t' = t - dt_years)
    /// - Points with t' <= 0 are filtered out (expired)
    /// - CPI levels are preserved
    /// - Base CPI is updated to the interpolated value at the roll time
    ///
    /// # Arguments
    /// * `days` - Number of days to roll forward
    ///
    /// # Returns
    /// A new inflation curve with shifted knots and updated base CPI.
    ///
    /// # Errors
    /// Returns an error if no knot points remain after filtering expired points.
    pub fn roll_forward(&self, days: i64) -> crate::Result<Self> {
        let dt_years = days as f64 / 365.0;

        // Get the new base CPI by interpolating at the roll time
        let new_base_cpi = self.cpi(dt_years);

        // Shift knots and filter expired points
        let rolled_points: Vec<(f64, f64)> = self
            .knots
            .iter()
            .zip(self.cpi_levels.iter())
            .filter_map(|(&t, &cpi)| {
                let new_t = t - dt_years;
                if new_t > 0.0 {
                    Some((new_t, cpi))
                } else {
                    None
                }
            })
            .collect();

        if rolled_points.is_empty() {
            return Err(crate::error::InputError::TooFewPoints.into());
        }

        InflationCurve::builder(self.id.clone())
            .base_cpi(new_base_cpi)
            .knots(rolled_points)
            .build()
    }
}

// Minimal trait implementation for polymorphism where needed

impl TermStructure for InflationCurve {
    #[inline]
    fn id(&self) -> &CurveId {
        &self.id
    }
}

/// Fluent builder for [`InflationCurve`].
pub struct InflationCurveBuilder {
    id: CurveId,
    base_cpi: f64,
    points: Vec<(f64, f64)>, // (t, cpi)
    style: InterpStyle,
}

impl InflationCurveBuilder {
    /// Set the **base CPI** level at t = 0.
    pub fn base_cpi(mut self, cpi: f64) -> Self {
        self.base_cpi = cpi;
        self
    }
    /// Supply knot points `(t, cpi_level)`.
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
        let (kvec, cvec): (Vec<f64>, Vec<f64>) = split_points(self.points);
        crate::math::interp::utils::validate_knots(&kvec)?;
        let knots = kvec.into_boxed_slice();
        let cpi_levels = cvec.into_boxed_slice();
        let interp = build_interp(
            self.style,
            knots.clone(),
            cpi_levels.clone(),
            ExtrapolationPolicy::default(),
        )?;
        Ok(InflationCurve {
            id: self.id,
            base_cpi: self.base_cpi,
            knots,
            cpi_levels,
            interp,
        })
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

    fn sample_curve() -> InflationCurve {
        InflationCurve::builder("US-CPI")
            .base_cpi(300.0)
            .knots([(0.0, 300.0), (1.0, 306.0), (2.0, 312.0)])
            .build()
            .expect("InflationCurve builder should succeed in test")
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
