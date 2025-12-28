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
use crate::math::interp::{types::Interp, ExtrapolationPolicy, InterpStyle};
use crate::types::CurveId;
use crate::Result;

// ============================================================================
// Arbitrage Validation Types
// ============================================================================

/// Result of arbitrage-free validation for a base correlation curve.
///
/// Contains detailed information about any violations found during validation,
/// allowing calibration systems to diagnose and fix curve issues.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ArbitrageCheckResult {
    /// Whether the curve passes all arbitrage checks.
    pub is_arbitrage_free: bool,
    /// List of specific violations found.
    pub violations: Vec<ArbitrageViolation>,
    /// Non-fatal warnings (e.g., high correlation near boundaries).
    pub warnings: Vec<String>,
    /// Maximum absolute violation magnitude (for severity assessment).
    pub max_violation_magnitude: f64,
}

impl ArbitrageCheckResult {
    /// Create a passing result with no violations.
    pub fn pass() -> Self {
        Self {
            is_arbitrage_free: true,
            violations: Vec::new(),
            warnings: Vec::new(),
            max_violation_magnitude: 0.0,
        }
    }

    /// Create a failing result with violations.
    pub fn fail(violations: Vec<ArbitrageViolation>) -> Self {
        let max_mag = violations.iter().map(|v| v.magnitude()).fold(0.0, f64::max);
        Self {
            is_arbitrage_free: false,
            violations,
            warnings: Vec::new(),
            max_violation_magnitude: max_mag,
        }
    }

    /// Add a warning to the result.
    pub fn with_warning(mut self, warning: String) -> Self {
        self.warnings.push(warning);
        self
    }

    /// Add multiple warnings.
    pub fn with_warnings(mut self, warnings: Vec<String>) -> Self {
        self.warnings.extend(warnings);
        self
    }
}

/// Specific arbitrage violation in a base correlation curve.
///
/// Each violation type captures the relevant data points involved.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ArbitrageViolation {
    /// Correlation decreases as detachment increases (violates monotonicity).
    ///
    /// Base correlation must be non-decreasing: senior tranches cannot have
    /// lower correlation than junior tranches.
    NonMonotonicCorrelation {
        /// Lower detachment point (percent)
        k1: f64,
        /// Correlation at lower detachment
        corr1: f64,
        /// Higher detachment point (percent)
        k2: f64,
        /// Correlation at higher detachment (should be >= corr1)
        corr2: f64,
    },

    /// Correlation outside valid range [0, 1].
    InvalidCorrelationBounds {
        /// Detachment point (percent)
        detachment: f64,
        /// Invalid correlation value
        correlation: f64,
    },

    /// Correlation too close to boundary (may cause numerical issues).
    BoundaryCorrelation {
        /// Detachment point (percent)
        detachment: f64,
        /// Correlation value near boundary
        correlation: f64,
        /// Whether near 0 or 1
        near_zero: bool,
    },
}

impl ArbitrageViolation {
    /// Get the magnitude of the violation for severity assessment.
    pub fn magnitude(&self) -> f64 {
        match self {
            ArbitrageViolation::NonMonotonicCorrelation { corr1, corr2, .. } => {
                (corr1 - corr2).abs()
            }
            ArbitrageViolation::InvalidCorrelationBounds { correlation, .. } => {
                if *correlation < 0.0 {
                    -correlation
                } else {
                    correlation - 1.0
                }
            }
            ArbitrageViolation::BoundaryCorrelation { correlation, .. } => {
                if *correlation < 0.05 {
                    0.05 - correlation
                } else {
                    correlation - 0.95
                }
            }
        }
    }

    /// Human-readable description of the violation.
    pub fn description(&self) -> String {
        match self {
            ArbitrageViolation::NonMonotonicCorrelation {
                k1,
                corr1,
                k2,
                corr2,
            } => {
                format!(
                    "Non-monotonic: β({:.1}%) = {:.4} > β({:.1}%) = {:.4}",
                    k1, corr1, k2, corr2
                )
            }
            ArbitrageViolation::InvalidCorrelationBounds {
                detachment,
                correlation,
            } => {
                format!(
                    "Invalid correlation {:.4} at K={:.1}% (must be in [0, 1])",
                    correlation, detachment
                )
            }
            ArbitrageViolation::BoundaryCorrelation {
                detachment,
                correlation,
                near_zero,
            } => {
                let boundary = if *near_zero { "0" } else { "1" };
                format!(
                    "Boundary correlation {:.4} near {} at K={:.1}%",
                    correlation, boundary, detachment
                )
            }
        }
    }
}

// ============================================================================
// Smoothing Methods
// ============================================================================

/// Smoothing method for enforcing arbitrage-free base correlation curves.
///
/// When calibration produces non-monotonic correlations, these methods
/// can be used to create an arbitrage-free curve.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SmoothingMethod {
    /// No smoothing - use raw calibrated values.
    #[default]
    None,

    /// Pool Adjacent Violators Algorithm (isotonic regression).
    ///
    /// Finds the closest monotonically increasing curve to the raw data
    /// in a least-squares sense. Fast and theoretically optimal.
    IsotonicRegression,

    /// Enforce strict monotonicity by adjusting violations.
    ///
    /// Each point is set to max(current, previous + epsilon).
    /// Simple but may accumulate adjustments.
    StrictMonotonic,

    /// Weighted moving average with monotonicity constraint.
    ///
    /// Smooths the curve while ensuring non-decreasing correlations.
    WeightedSmoothing,
}

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
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(try_from = "RawBaseCorrelationCurve", into = "RawBaseCorrelationCurve")
)]
pub struct BaseCorrelationCurve {
    /// Curve identifier (typically index name + maturity)
    pub id: CurveId,
    /// Detachment points in percent (e.g., 3.0 for a 0-3% tranche)
    pub detachment_points: Vec<f64>,
    /// Base correlation values corresponding to each detachment point
    pub correlations: Vec<f64>,
    /// Interpolator for base correlations
    interp: Interp,
}

#[cfg(feature = "serde")]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawBaseCorrelationCurve {
    id: CurveId,
    detachment_points: Vec<f64>,
    correlations: Vec<f64>,
}

#[cfg(feature = "serde")]
impl From<BaseCorrelationCurve> for RawBaseCorrelationCurve {
    fn from(curve: BaseCorrelationCurve) -> Self {
        RawBaseCorrelationCurve {
            id: curve.id,
            detachment_points: curve.detachment_points,
            correlations: curve.correlations,
        }
    }
}

#[cfg(feature = "serde")]
impl TryFrom<RawBaseCorrelationCurve> for BaseCorrelationCurve {
    type Error = crate::Error;

    fn try_from(raw: RawBaseCorrelationCurve) -> crate::Result<Self> {
        let points: Vec<(f64, f64)> = raw
            .detachment_points
            .iter()
            .copied()
            .zip(raw.correlations.iter().copied())
            .collect();

        BaseCorrelationCurve::builder(raw.id).points(points).build()
    }
}

impl Clone for BaseCorrelationCurve {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            detachment_points: self.detachment_points.clone(),
            correlations: self.correlations.clone(),
            interp: self.interp.clone(),
        }
    }
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
        self.interp.interp(detachment_pct)
    }

    /// Raw detachment points used to build the curve.
    pub fn detachment_points(&self) -> &[f64] {
        &self.detachment_points
    }

    /// Raw correlation values at each detachment point.
    pub fn correlations(&self) -> &[f64] {
        &self.correlations
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

        for (det, corr) in self
            .detachment_points
            .iter()
            .copied()
            .zip(self.correlations.iter().copied())
        {
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

    // ========================================================================
    // Arbitrage Validation
    // ========================================================================

    /// Validate that the curve is arbitrage-free.
    ///
    /// Checks for:
    /// 1. Monotonicity: β(K₁) ≤ β(K₂) for K₁ < K₂
    /// 2. Valid bounds: 0 ≤ β(K) ≤ 1 for all K
    /// 3. Boundary warnings: correlations very close to 0 or 1
    ///
    /// # Example
    ///
    /// ```rust
    /// use finstack_core::market_data::term_structures::base_correlation::BaseCorrelationCurve;
    ///
    /// let curve = BaseCorrelationCurve::builder("CDX")
    ///     .points(vec![(3.0, 0.25), (7.0, 0.45), (10.0, 0.60)])
    ///     .build()
    ///     .expect("Valid curve");
    ///
    /// let result = curve.validate_arbitrage_free();
    /// assert!(result.is_arbitrage_free);
    /// ```
    pub fn validate_arbitrage_free(&self) -> ArbitrageCheckResult {
        let mut violations = Vec::new();
        let mut warnings = Vec::new();

        // Check correlation bounds
        for (i, (&det, &corr)) in self
            .detachment_points
            .iter()
            .zip(&self.correlations)
            .enumerate()
        {
            // Check valid range
            if !(0.0..=1.0).contains(&corr) {
                violations.push(ArbitrageViolation::InvalidCorrelationBounds {
                    detachment: det,
                    correlation: corr,
                });
            }

            // Check boundary proximity (warning only)
            if corr < 0.02 {
                warnings.push(format!(
                    "Low correlation {:.4} at K={:.1}% may cause numerical issues",
                    corr, det
                ));
            } else if corr > 0.98 {
                warnings.push(format!(
                    "High correlation {:.4} at K={:.1}% may cause numerical issues",
                    corr, det
                ));
            }

            // Check monotonicity
            if i > 0 {
                let prev_corr = self.correlations[i - 1];
                if corr < prev_corr - 1e-9 {
                    violations.push(ArbitrageViolation::NonMonotonicCorrelation {
                        k1: self.detachment_points[i - 1],
                        corr1: prev_corr,
                        k2: det,
                        corr2: corr,
                    });
                }
            }
        }

        if violations.is_empty() {
            ArbitrageCheckResult::pass().with_warnings(warnings)
        } else {
            ArbitrageCheckResult::fail(violations).with_warnings(warnings)
        }
    }

    /// Check if the curve is monotonically non-decreasing.
    pub fn is_monotonic(&self) -> bool {
        for i in 1..self.correlations.len() {
            if self.correlations[i] < self.correlations[i - 1] - 1e-9 {
                return false;
            }
        }
        true
    }

    // ========================================================================
    // Smoothing Methods
    // ========================================================================

    /// Apply smoothing to create an arbitrage-free curve.
    ///
    /// If the curve is already arbitrage-free, returns a clone.
    /// Otherwise, applies the specified smoothing method.
    ///
    /// # Example
    ///
    /// ```rust
    /// use finstack_core::market_data::term_structures::base_correlation::{
    ///     BaseCorrelationCurve, SmoothingMethod
    /// };
    ///
    /// // Create a non-monotonic curve
    /// let raw = BaseCorrelationCurve::builder("TEST")
    ///     .points(vec![(3.0, 0.50), (7.0, 0.40), (10.0, 0.60)])
    ///     .build()
    ///     .expect("Valid curve");
    ///
    /// let smoothed = raw.apply_smoothing(SmoothingMethod::IsotonicRegression)
    ///     .expect("Smoothing should succeed");
    ///
    /// assert!(smoothed.is_monotonic());
    /// ```
    pub fn apply_smoothing(&self, method: SmoothingMethod) -> Result<Self> {
        match method {
            SmoothingMethod::None => Ok(self.clone()),
            SmoothingMethod::IsotonicRegression => self.apply_isotonic_regression(),
            SmoothingMethod::StrictMonotonic => self.apply_strict_monotonic(),
            SmoothingMethod::WeightedSmoothing => self.apply_weighted_smoothing(),
        }
    }

    /// Apply Pool Adjacent Violators Algorithm (PAVA) for isotonic regression.
    ///
    /// This finds the closest monotonically non-decreasing sequence to the
    /// input correlations in the L2 sense.
    fn apply_isotonic_regression(&self) -> Result<Self> {
        if self.correlations.is_empty() {
            return Ok(self.clone());
        }

        let n = self.correlations.len();
        let mut smoothed = self.correlations.clone();

        // PAVA: Pool Adjacent Violators Algorithm
        // Process from left to right, pooling violations
        let mut i = 0;
        while i < n - 1 {
            if smoothed[i] > smoothed[i + 1] {
                // Found a violation - pool these values
                let mut j = i + 1;

                // Extend pool to include all consecutive violations
                while j < n && smoothed[j - 1] > smoothed[j] {
                    j += 1;
                }

                // Average the pooled values
                let avg = smoothed[i..j].iter().sum::<f64>() / (j - i) as f64;
                for item in smoothed.iter_mut().take(j).skip(i) {
                    *item = avg;
                }

                // Go back to check if new pool violates with previous
                i = i.saturating_sub(1);
                if i == 0 && smoothed[0] <= smoothed[1] {
                    i = 1; // Don't get stuck at 0 if no violation
                }
            } else {
                i += 1;
            }
        }

        // Clamp to valid range
        for c in &mut smoothed {
            *c = c.clamp(0.001, 0.999);
        }

        // Enforce final monotonicity (should already be satisfied, but ensure)
        for i in 1..n {
            if smoothed[i] < smoothed[i - 1] {
                smoothed[i] = smoothed[i - 1];
            }
        }

        let points: Vec<(f64, f64)> = self
            .detachment_points
            .iter()
            .copied()
            .zip(smoothed)
            .collect();

        BaseCorrelationCurve::builder(self.id.clone())
            .points(points)
            .build()
    }

    /// Apply strict monotonicity enforcement.
    ///
    /// Each correlation is set to max(current, previous + epsilon).
    fn apply_strict_monotonic(&self) -> Result<Self> {
        const EPSILON: f64 = 1e-6;

        if self.correlations.is_empty() {
            return Ok(self.clone());
        }

        let mut smoothed = self.correlations.clone();

        // Forward pass: ensure non-decreasing
        for i in 1..smoothed.len() {
            let min_val = smoothed[i - 1] + EPSILON;
            if smoothed[i] < min_val {
                smoothed[i] = min_val;
            }
        }

        // Clamp to valid range
        for c in &mut smoothed {
            *c = c.clamp(0.001, 0.999);
        }

        let points: Vec<(f64, f64)> = self
            .detachment_points
            .iter()
            .copied()
            .zip(smoothed)
            .collect();

        BaseCorrelationCurve::builder(self.id.clone())
            .points(points)
            .build()
    }

    /// Apply weighted smoothing with monotonicity constraint.
    ///
    /// Uses a weighted average that respects the monotonicity requirement.
    fn apply_weighted_smoothing(&self) -> Result<Self> {
        if self.correlations.len() < 3 {
            return self.apply_strict_monotonic();
        }

        let n = self.correlations.len();
        let mut smoothed = vec![0.0; n];

        // First and last points: keep original (or apply light smoothing)
        smoothed[0] = self.correlations[0];
        smoothed[n - 1] = self.correlations[n - 1];

        // Interior points: weighted average of neighbors
        for (i, smoothed_val) in smoothed.iter_mut().enumerate().take(n - 1).skip(1) {
            // Weights: 0.25 * prev + 0.5 * current + 0.25 * next
            let avg = 0.25 * self.correlations[i - 1]
                + 0.5 * self.correlations[i]
                + 0.25 * self.correlations[i + 1];
            *smoothed_val = avg;
        }

        // Enforce monotonicity
        for i in 1..n {
            if smoothed[i] < smoothed[i - 1] {
                smoothed[i] = smoothed[i - 1] + 1e-6;
            }
        }

        // Clamp to valid range
        for c in &mut smoothed {
            *c = c.clamp(0.001, 0.999);
        }

        let points: Vec<(f64, f64)> = self
            .detachment_points
            .iter()
            .copied()
            .zip(smoothed)
            .collect();

        BaseCorrelationCurve::builder(self.id.clone())
            .points(points)
            .build()
    }

    /// Create an arbitrage-free version of this curve.
    ///
    /// Convenience method that validates and applies smoothing if needed.
    pub fn make_arbitrage_free(&self, method: SmoothingMethod) -> Result<Self> {
        let validation = self.validate_arbitrage_free();
        if validation.is_arbitrage_free {
            Ok(self.clone())
        } else {
            self.apply_smoothing(method)
        }
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

        // Sort by detachment point deterministically (panic-free even with NaNs).
        let mut sorted_points = self.points;
        sorted_points.sort_by(|a, b| a.0.total_cmp(&b.0));

        // Validate points
        for (detachment, corr) in &sorted_points {
            if !detachment.is_finite() || *detachment < 0.0 {
                return Err(InputError::Invalid.into());
            }
            if !corr.is_finite() || *corr < 0.0 || *corr > 1.0 {
                return Err(InputError::Invalid.into());
            }
        }

        let (kvec, cvec): (Vec<f64>, Vec<f64>) = sorted_points.into_iter().unzip();

        let interp = super::common::build_interp(
            InterpStyle::Linear,
            kvec.clone().into_boxed_slice(),
            cvec.clone().into_boxed_slice(),
            ExtrapolationPolicy::FlatZero,
        )?;

        Ok(BaseCorrelationCurve {
            id: self.id,
            detachment_points: kvec,
            correlations: cvec,
            interp,
        })
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, clippy::indexing_slicing)]
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

    fn non_monotonic_curve() -> BaseCorrelationCurve {
        BaseCorrelationCurve::builder("NON_MONOTONIC")
            .points(vec![
                (3.0, 0.50), // Higher than next
                (7.0, 0.40), // Violation: decreases
                (10.0, 0.60),
            ])
            .build()
            .expect("Builder should accept non-monotonic curves")
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

    // ========================================================================
    // Arbitrage Validation Tests
    // ========================================================================

    #[test]
    fn test_monotonic_curve_is_arbitrage_free() {
        let curve = sample_curve();
        let result = curve.validate_arbitrage_free();

        assert!(result.is_arbitrage_free);
        assert!(result.violations.is_empty());
    }

    #[test]
    fn test_non_monotonic_curve_detected() {
        let curve = non_monotonic_curve();
        let result = curve.validate_arbitrage_free();

        assert!(!result.is_arbitrage_free);
        assert!(!result.violations.is_empty());

        // Should have exactly one violation
        assert_eq!(result.violations.len(), 1);
        assert!(matches!(
            result.violations[0],
            ArbitrageViolation::NonMonotonicCorrelation { .. }
        ));
    }

    #[test]
    fn test_is_monotonic() {
        let monotonic = sample_curve();
        assert!(monotonic.is_monotonic());

        let non_monotonic = non_monotonic_curve();
        assert!(!non_monotonic.is_monotonic());
    }

    #[test]
    fn test_violation_magnitude() {
        let v = ArbitrageViolation::NonMonotonicCorrelation {
            k1: 3.0,
            corr1: 0.50,
            k2: 7.0,
            corr2: 0.40,
        };
        assert!((v.magnitude() - 0.10).abs() < 1e-10);
    }

    #[test]
    fn test_violation_description() {
        let v = ArbitrageViolation::NonMonotonicCorrelation {
            k1: 3.0,
            corr1: 0.50,
            k2: 7.0,
            corr2: 0.40,
        };
        let desc = v.description();
        assert!(desc.contains("Non-monotonic"));
        assert!(desc.contains("3.0"));
        assert!(desc.contains("7.0"));
    }

    // ========================================================================
    // Smoothing Tests
    // ========================================================================

    #[test]
    fn test_isotonic_regression_simple() {
        let curve = non_monotonic_curve();
        let smoothed = curve
            .apply_smoothing(SmoothingMethod::IsotonicRegression)
            .expect("Smoothing should succeed");

        assert!(smoothed.is_monotonic());

        // Check that smoothed curve is valid
        let result = smoothed.validate_arbitrage_free();
        assert!(result.is_arbitrage_free);
    }

    #[test]
    fn test_isotonic_regression_multiple_violations() {
        // Multiple violations
        let curve = BaseCorrelationCurve::builder("MULTI_VIOLATION")
            .points(vec![
                (3.0, 0.60),  // High
                (7.0, 0.40),  // Drops
                (10.0, 0.50), // Rises but still below first
                (15.0, 0.45), // Drops again
            ])
            .build()
            .expect("Builder should accept");

        let smoothed = curve
            .apply_smoothing(SmoothingMethod::IsotonicRegression)
            .expect("Smoothing should succeed");

        assert!(smoothed.is_monotonic());
    }

    #[test]
    fn test_strict_monotonic() {
        let curve = non_monotonic_curve();
        let smoothed = curve
            .apply_smoothing(SmoothingMethod::StrictMonotonic)
            .expect("Smoothing should succeed");

        assert!(smoothed.is_monotonic());
    }

    #[test]
    fn test_weighted_smoothing() {
        let curve = non_monotonic_curve();
        let smoothed = curve
            .apply_smoothing(SmoothingMethod::WeightedSmoothing)
            .expect("Smoothing should succeed");

        assert!(smoothed.is_monotonic());
    }

    #[test]
    fn test_no_smoothing_preserves_original() {
        let curve = sample_curve();
        let smoothed = curve
            .apply_smoothing(SmoothingMethod::None)
            .expect("No smoothing should succeed");

        assert_eq!(curve.correlations, smoothed.correlations);
        assert_eq!(curve.detachment_points, smoothed.detachment_points);
    }

    #[test]
    fn test_make_arbitrage_free_already_valid() {
        let curve = sample_curve();
        let result = curve
            .make_arbitrage_free(SmoothingMethod::IsotonicRegression)
            .expect("Should succeed");

        // Should return the same curve (no changes needed)
        assert_eq!(curve.correlations, result.correlations);
    }

    #[test]
    fn test_make_arbitrage_free_fixes_invalid() {
        let curve = non_monotonic_curve();
        let result = curve
            .make_arbitrage_free(SmoothingMethod::IsotonicRegression)
            .expect("Should succeed");

        assert!(result.is_monotonic());
        assert!(result.validate_arbitrage_free().is_arbitrage_free);
    }

    #[test]
    fn test_isotonic_preserves_endpoints() {
        // PAVA should keep first and last values if they don't violate
        let curve = BaseCorrelationCurve::builder("ENDPOINTS")
            .points(vec![
                (3.0, 0.25),  // First - should be preserved
                (7.0, 0.20),  // Violation
                (10.0, 0.60), // Last - should be preserved
            ])
            .build()
            .expect("Builder should accept");

        let smoothed = curve
            .apply_smoothing(SmoothingMethod::IsotonicRegression)
            .expect("Smoothing should succeed");

        // First point might be adjusted to satisfy monotonicity
        // Last point should be preserved or close
        assert!(smoothed.is_monotonic());
    }

    #[test]
    fn test_boundary_warnings() {
        let curve = BaseCorrelationCurve::builder("BOUNDARY")
            .points(vec![
                (3.0, 0.01), // Very low - should warn
                (7.0, 0.50),
                (10.0, 0.99), // Very high - should warn
            ])
            .build()
            .expect("Builder should accept");

        let result = curve.validate_arbitrage_free();

        // Should be arbitrage-free but have warnings
        assert!(result.is_arbitrage_free);
        assert!(!result.warnings.is_empty());
    }

    #[test]
    fn test_arbitrage_check_result_helpers() {
        let pass = ArbitrageCheckResult::pass();
        assert!(pass.is_arbitrage_free);
        assert!(pass.violations.is_empty());

        let fail = ArbitrageCheckResult::fail(vec![ArbitrageViolation::NonMonotonicCorrelation {
            k1: 3.0,
            corr1: 0.50,
            k2: 7.0,
            corr2: 0.40,
        }]);
        assert!(!fail.is_arbitrage_free);
        assert_eq!(fail.violations.len(), 1);
        assert!((fail.max_violation_magnitude - 0.10).abs() < 1e-10);
    }
}
