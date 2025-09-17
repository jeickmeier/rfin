//! Base correlation curve for CDS tranche pricing.
//!
//! Maps tranche detachment points to the implied base correlation used
//! in the standardized base correlation pricing model. Base correlation
//! represents the implied correlation of a tranche with attachment point
//! of zero (equity tranche) and is used to maintain consistent pricing
//! across different tranche structures.
//!
//! ## Example
//! ```rust
//! use finstack_core::market_data::term_structures::base_correlation::BaseCorrelationCurve;
//!
//! let curve = BaseCorrelationCurve::builder("CDX.NA.IG.42_5Y")
//!     .points(vec![(3.0, 0.25), (7.0, 0.45), (10.0, 0.60)])
//!     .build()
//!     .unwrap();
//! assert!(curve.correlation(5.0) > 0.25);
//! ```

use crate::error::InputError;
use crate::types::CurveId;
use crate::Result;
use crate::F;

/// A curve representing the base correlation for a credit index.
///
/// This curve maps detachment points (in percent) to the corresponding
/// base correlation value. Uses linear interpolation between points and
/// flat extrapolation beyond the boundaries.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct BaseCorrelationCurve {
    /// Curve identifier (typically index name + maturity)
    pub id: CurveId,
    /// Detachment points in percent (e.g., 3.0 for a 0-3% tranche)
    pub detachment_points: Vec<F>,
    /// Base correlation values corresponding to each detachment point
    pub correlations: Vec<F>,
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
    pub fn correlation(&self, detachment_pct: F) -> F {
        if self.detachment_points.is_empty() {
            return 0.0;
        }

        if detachment_pct <= self.detachment_points[0] {
            return self.correlations[0];
        }
        if detachment_pct >= *self.detachment_points.last().unwrap() {
            return *self.correlations.last().unwrap();
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
                    *self.correlations.last().unwrap()
                }
            }
        }
    }

    /// Raw detachment points used to build the curve.
    pub fn detachment_points(&self) -> &[F] {
        &self.detachment_points
    }

    /// Raw correlation values at each detachment point.
    pub fn correlations(&self) -> &[F] {
        &self.correlations
    }

    fn find_bracket(&self, detachment_pct: F) -> Option<(usize, usize)> {
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
///     .unwrap();
/// assert!(curve.correlation(5.0) > 0.25);
/// ```
pub struct BaseCorrelationCurveBuilder {
    id: CurveId,
    points: Vec<(F, F)>, // (detachment_pct, correlation)
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
    pub fn add_point(mut self, detachment_pct: F, correlation: F) -> Self {
        self.points.push((detachment_pct, correlation));
        self
    }

    /// Set all points at once.
    pub fn points<I>(mut self, points: I) -> Self
    where
        I: IntoIterator<Item = (F, F)>,
    {
        self.points.extend(points);
        self
    }

    /// Alias for `points` to align naming with other 1D curve builders.
    #[inline]
    pub fn knots<I>(self, points: I) -> Self
    where
        I: IntoIterator<Item = (F, F)>,
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
        sorted_points.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

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

// Implement unified builder trait for BaseCorrelationCurveBuilder
impl super::common::CurveBuilder for BaseCorrelationCurveBuilder {
    type Output = BaseCorrelationCurve;

    fn knots<I>(self, pts: I) -> Self
    where
        I: IntoIterator<Item = (F, F)>,
    {
        BaseCorrelationCurveBuilder::points(self, pts)
    }

    fn build(self) -> crate::Result<Self::Output> {
        BaseCorrelationCurveBuilder::build(self)
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
            .unwrap()
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
            .unwrap();

        assert_eq!(curve.detachment_points, vec![3.0, 7.0, 10.0]);
        assert_eq!(curve.correlations, vec![0.25, 0.45, 0.60]);
    }
}
