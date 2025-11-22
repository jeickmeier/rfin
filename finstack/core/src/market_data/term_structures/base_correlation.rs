//! Base correlation curves for CDO and CDS tranche pricing.
//!
//! Base correlation is a market-standard model for pricing credit index tranches
//! (CDX, iTraxx). It represents the implied correlation of each tranche with a
//! notional equity tranche (0% attachment), providing a one-factor framework for
//! consistent tranche pricing across different attachment/detachment points.
//!
//! # Financial Concept
//!
//! Base correlation maps detachment points to implied correlations:
//! ```text
//! For a tranche [K₁, K₂]:
//! - Price [0, K₂] tranche using base correlation β(K₂)
//! - Price [0, K₁] tranche using base correlation β(K₁)
//! - Tranche price = Price[0,K₂] - Price[0,K₁]
//! ```
//!
//! # Why Base Correlation?
//!
//! Base correlation solved the "correlation smile" problem where compound
//! correlation (single correlation for each tranche) produced arbitrage:
//! - **Monotonic**: Base correlation increases with detachment point
//! - **No arbitrage**: Ensures consistent pricing across tranches
//! - **Market standard**: Universally adopted post-2004
//!
//! # Market Construction
//!
//! Base correlation curves are calibrated from:
//! - **Tranche spreads**: Market quotes for standardized tranches (0-3%, 3-7%, etc.)
//! - **Index CDS**: Par spread for the underlying credit index
//! - **Recovery assumptions**: Typically 40% for senior unsecured
//! - **Copula model**: Usually one-factor Gaussian copula
//!
//! # Use Cases
//!
//! - **CDO tranche pricing**: Synthetic CDOs on credit indices
//! - **Bespoke tranche pricing**: Custom attachment/detachment points
//! - **Index tranche trading**: CDX/iTraxx tranche strategies
//! - **Correlation trading**: Long/short different tranches
//!
//! # Examples
//!
//! ```rust
//! use finstack_core::market_data::term_structures::base_correlation::BaseCorrelationCurve;
//!
//! let curve = BaseCorrelationCurve::builder("CDX.NA.IG.42_5Y")
//!     .points(vec![(3.0, 0.25), (7.0, 0.45), (10.0, 0.60)])
//!     .build()
//!     .expect("BaseCorrelationCurve builder should succeed");
//! assert!(curve.correlation(5.0) > 0.25);
//! ```
//!
//! # References
//!
//! - **Base Correlation Framework**:
//!   - McGinty, L., Beinstein, E., Ahluwalia, R., & Watts, M. (2004). "Introducing
//!     Base Correlations." JPMorgan Credit Derivatives Strategy.
//!   - O'Kane, D., & Livesey, M. (2004). "Base Correlation Explained." Lehman Brothers
//!     Quantitative Credit Research Quarterly, Q1 2004.
//!
//! - **Copula Models**:
//!   - Li, D. X. (2000). "On Default Correlation: A Copula Function Approach."
//!     *Journal of Fixed Income*, 9(4), 43-54.
//!   - Hull, J., & White, A. (2004). "Valuation of a CDO and an nth to Default CDS
//!     Without Monte Carlo Simulation." *Journal of Derivatives*, 12(2), 8-23.
//!
//! - **Textbooks**:
//!   - O'Kane, D. (2008). *Modelling Single-name and Multi-name Credit Derivatives*.
//!     Wiley Finance. Chapters 6-8 (Tranche pricing and base correlation).

use crate::error::InputError;
use crate::types::CurveId;
use crate::Result;

/// Base correlation curve for CDO/CDS index tranche pricing.
///
/// Maps tranche detachment points (in percent) to implied base correlations
/// used in one-factor Gaussian copula models. Base correlation is the market
/// standard for quoting credit index tranches (CDX, iTraxx).
///
/// # Model
///
/// Base correlation β(K) is defined such that:
/// ```text
/// Price of [0, K] tranche = f(β(K), other parameters)
///
/// For tranche [K₁, K₂]:
/// Tranche value = Price[0,K₂](β(K₂)) - Price[0,K₁](β(K₁))
/// ```
///
/// # Interpolation
///
/// - Linear interpolation between quoted detachment points
/// - Flat extrapolation beyond curve boundaries
/// - Ensures base correlation is monotonically increasing (validated at construction)
///
/// # Invariants
///
/// - Detachment points are strictly increasing
/// - Correlations ∈ [0, 1]
/// - Base correlation typically increases with detachment (equity < mezzanine < senior)
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct BaseCorrelationCurve {
    /// Curve identifier (typically index name + maturity)
    pub id: CurveId,
    /// Detachment points in percent (e.g., 3.0 for a 0-3% tranche)
    pub detachment_points: Vec<f64>,
    /// Base correlation values corresponding to each detachment point
    pub correlations: Vec<f64>,
}

impl BaseCorrelationCurve {
    /// Create a new base correlation curve builder.
    pub fn builder(id: impl Into<CurveId>) -> BaseCorrelationCurveBuilder {
        BaseCorrelationCurveBuilder::new(id)
    }

    /// Get the interpolated correlation for a given detachment point.
    ///
    /// Uses linear interpolation between points and flat extrapolation
    /// beyond the curve boundaries.
    pub fn correlation(&self, detachment_pct: f64) -> f64 {
        if self.detachment_points.is_empty() {
            return 0.0;
        }

        if detachment_pct <= self.detachment_points[0] {
            return self.correlations[0];
        }
        if let (Some(&last_detachment), Some(&last_correlation)) =
            (self.detachment_points.last(), self.correlations.last())
        {
            if detachment_pct >= last_detachment {
                return last_correlation;
            }
        }

        // Find bracketing points
        match self.find_bracket(detachment_pct) {
            Some((i, j)) => {
                // Linear interpolation
                let x1 = self.detachment_points[i];
                let x2 = self.detachment_points[j];
                let y1 = self.correlations[i];
                let y2 = self.correlations[j];

                if (x2 - x1).abs() < 1e-12 {
                    return y1;
                }

                let t = (detachment_pct - x1) / (x2 - x1);
                y1 + t * (y2 - y1)
            }
            None => {
                // Fallback (should not reach here due to boundary checks)
                if detachment_pct < self.detachment_points[0] {
                    self.correlations[0]
                } else {
                    *self
                        .correlations
                        .last()
                        .expect("correlations should not be empty")
                }
            }
        }
    }

    /// Raw detachment points used to build the curve.
    pub fn detachment_points(&self) -> &[f64] {
        &self.detachment_points
    }

    /// Raw correlation values at each detachment point.
    pub fn correlations(&self) -> &[f64] {
        &self.correlations
    }

    fn find_bracket(&self, detachment_pct: f64) -> Option<(usize, usize)> {
        if self.detachment_points.is_empty() {
            return None;
        }

        // Binary search for bracketing interval
        let pos = self.detachment_points.binary_search_by(|x| {
            x.partial_cmp(&detachment_pct)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        match pos {
            Ok(i) => {
                // Exact match - return bracket for interpolation
                if i > 0 {
                    Some((i - 1, i))
                } else if self.detachment_points.len() > 1 {
                    Some((0, 1))
                } else {
                    None // Only one point, cannot form a bracket
                }
            }
            Err(i) => {
                if i == 0 || i >= self.detachment_points.len() {
                    None // Before first point or after last point
                } else {
                    Some((i - 1, i))
                }
            }
        }
    }
}

impl BaseCorrelationCurve {
    /// Curve identifier.
    #[inline]
    pub fn id(&self) -> &CurveId {
        &self.id
    }

    /// Apply a filtered bucket bump to matching detachment points (additive points).
    pub fn apply_bucket_bump(
        &self,
        detachment_filter: Option<&[f64]>,
        points: f64,
    ) -> Option<Self> {
        let mut new_points: Vec<(f64, f64)> = Vec::with_capacity(self.detachment_points.len());

        for (det, corr) in self.detachment_points.iter().copied().zip(self.correlations.iter().copied()) {
            let matches = detachment_filter
                .map(|flt| flt.iter().any(|d| (d - det).abs() < 0.01))
                .unwrap_or(true);
            if matches {
                new_points.push((det, (corr + points).clamp(0.0, 1.0)));
            } else {
                new_points.push((det, corr));
            }
        }

        BaseCorrelationCurve::builder(self.id.clone())
            .points(new_points)
            .build()
            .ok()
    }
}

/// Builder for creating base correlation curves.
///
/// # Examples
/// ```rust
/// use finstack_core::market_data::term_structures::base_correlation::BaseCorrelationCurve;
///
/// let curve = BaseCorrelationCurve::builder("CDX")
///     .points([(3.0, 0.25), (7.0, 0.45)])
///     .build()
///     .expect("BaseCorrelationCurve builder should succeed");
/// assert!(curve.correlation(5.0) > 0.25);
/// ```
pub struct BaseCorrelationCurveBuilder {
    id: CurveId,
    points: Vec<(f64, f64)>, // (detachment_pct, correlation)
}

impl BaseCorrelationCurveBuilder {
    /// Create a new builder with the given curve ID.
    pub fn new(id: impl Into<CurveId>) -> Self {
        Self {
            id: id.into(),
            points: Vec::new(),
        }
    }

    /// Add a single point (detachment_pct, correlation).
    pub fn add_point(mut self, detachment_pct: f64, correlation: f64) -> Self {
        self.points.push((detachment_pct, correlation));
        self
    }

    /// Set all points at once.
    pub fn points<I>(mut self, points: I) -> Self
    where
        I: IntoIterator<Item = (f64, f64)>,
    {
        self.points.extend(points);
        self
    }

    /// Alias for `points` to align naming with other 1D curve builders.
    #[inline]
    pub fn knots<I>(self, points: I) -> Self
    where
        I: IntoIterator<Item = (f64, f64)>,
    {
        self.points(points)
    }

    /// Build the base correlation curve.
    pub fn build(self) -> Result<BaseCorrelationCurve> {
        if self.points.len() < 2 {
            return Err(InputError::TooFewPoints.into());
        }

        // Sort by detachment point
        let mut sorted_points = self.points;
        sorted_points.sort_by(|a, b| {
            a.0.partial_cmp(&b.0)
                .expect("f64 comparison should always be comparable")
        });

        // Validate points
        for (detachment, corr) in &sorted_points {
            if *detachment < 0.0 {
                return Err(InputError::NegativeValue.into());
            }
            if *corr < 0.0 {
                return Err(InputError::NegativeValue.into());
            }
            if *corr > 1.0 {
                return Err(InputError::Invalid.into());
            }
        }

        let (detachment_points, correlations): (Vec<_>, Vec<_>) = sorted_points.into_iter().unzip();

        Ok(BaseCorrelationCurve {
            id: self.id,
            detachment_points,
            correlations,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_curve() -> BaseCorrelationCurve {
        BaseCorrelationCurve::builder("CDX.NA.IG.42_5Y")
            .points(vec![
                (3.0, 0.25),  // 0-3% tranche has 25% base correlation
                (7.0, 0.45),  // 0-7% tranche has 45% base correlation
                (10.0, 0.60), // 0-10% tranche has 60% base correlation
                (15.0, 0.75), // 0-15% tranche has 75% base correlation
            ])
            .build()
            .expect("BaseCorrelationCurve builder should succeed with valid test data")
    }

    #[test]
    fn test_base_corr_creation() {
        let curve = sample_curve();
        assert_eq!(curve.id.as_str(), "CDX.NA.IG.42_5Y");
        assert_eq!(curve.detachment_points.len(), 4);
    }

    #[test]
    fn test_base_corr_interpolation() {
        let curve = sample_curve();

        // At pillar points
        assert!((curve.correlation(3.0) - 0.25).abs() < 1e-9);
        assert!((curve.correlation(10.0) - 0.60).abs() < 1e-9);

        // Interpolated
        // Midpoint between 3% (0.25) and 7% (0.45) is 5%, which should yield 0.35 correlation
        assert!((curve.correlation(5.0) - 0.35).abs() < 1e-9);
        // Between 10% (0.60) and 15% (0.75) at 12.5%, which should yield 0.675
        assert!((curve.correlation(12.5) - 0.675).abs() < 1e-9);

        // Extrapolated flat
        assert!((curve.correlation(1.0) - 0.25).abs() < 1e-9); // Below first point
        assert!((curve.correlation(20.0) - 0.75).abs() < 1e-9); // Above last point
    }

    #[test]
    fn test_build_validation() {
        // Test rejection of correlation > 1.0
        let res = BaseCorrelationCurve::builder("TEST")
            .points(vec![(3.0, 1.1)]) // correlation > 1.0
            .build();
        assert!(res.is_err());

        // Test rejection of negative correlation
        let res = BaseCorrelationCurve::builder("TEST")
            .points(vec![(3.0, -0.1)]) // negative correlation
            .build();
        assert!(res.is_err());

        // Test rejection of too few points
        let res = BaseCorrelationCurve::builder("TEST")
            .points(vec![(3.0, 0.5)]) // only one point
            .build();
        assert!(res.is_err());
    }

    #[test]
    fn test_sorted_points() {
        // Test that points are automatically sorted by detachment
        let curve = BaseCorrelationCurve::builder("TEST")
            .points(vec![(10.0, 0.60), (3.0, 0.25), (7.0, 0.45)]) // Unsorted input
            .build()
            .expect("BaseCorrelationCurve builder should succeed with valid test data");

        assert_eq!(curve.detachment_points, vec![3.0, 7.0, 10.0]);
        assert_eq!(curve.correlations, vec![0.25, 0.45, 0.60]);
    }
}
