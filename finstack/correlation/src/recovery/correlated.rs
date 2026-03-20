//! Market-correlated stochastic recovery model (Andersen-Sidenius).
//!
//! Models recovery through a latent-factor shock plus a smooth bounding
//! transform. The public parameters are quoted in decimals, so `0.40` means a
//! 40% recovery rate and `0.25` means 25% recovery volatility.
//!
//! # Mathematical Model
//!
//! ```text
//! shock(Z) = ρ_R · σ_R · Z
//! R(Z) = min_R + (max_R - min_R) * logistic(center(μ_R) + shock(Z) / local_slope)
//! ```
//!
//! where:
//! - `μ_R` is the mean recovery at `Z = 0`
//! - `σ_R` is the recovery-volatility scale
//! - `ρ_R` is the factor sensitivity
//! - `Z` is the supplied latent market factor
//!
//! The implementation does **not** hard-clamp an affine recovery rule. Instead,
//! it uses a logistic transform so recovery stays inside the configured bounds
//! smoothly while preserving the target mean exactly at `Z = 0`.
//!
//! # Sign Convention
//!
//! The sign of `Z` is caller-defined. With the crate's preset calibrations
//! (`ρ_R < 0`), negative factor realizations increase recovery and positive
//! realizations decrease it. Callers that want the opposite mapping should
//! either negate the factor they pass in or choose a positive `ρ_R`.
//!
//! # Calibration
//!
//! Typical market calibration from CDX equity tranche:
//! - Mean recovery: 40%
//! - Recovery volatility: 20-30%
//! - Factor correlation: -30% to -50%
//!
//! # References
//!
//! - Stochastic recovery and random loading context:
//!   `docs/REFERENCES.md#andersen-sidenius-2005-rfl`
//! - Tranche calibration background:
//!   `docs/REFERENCES.md#krekel-stumpp-2006-correlation-products`

use super::RecoveryModel;

/// Market-correlated stochastic recovery model.
///
/// Recovery varies with the systematic market factor, capturing
/// the empirical negative correlation between defaults and recovery.
///
/// # References
///
/// - `docs/REFERENCES.md#andersen-sidenius-2005-rfl`
/// - `docs/REFERENCES.md#altman-et-al-2005-recovery`
#[derive(Debug, Clone)]
pub struct CorrelatedRecovery {
    /// Mean recovery rate
    mean_recovery: f64,
    /// Recovery volatility (standard deviation)
    recovery_volatility: f64,
    /// Correlation with systematic factor (typically negative)
    factor_correlation: f64,
    /// Minimum recovery (floor)
    min_recovery: f64,
    /// Maximum recovery (ceiling)
    max_recovery: f64,
}

impl CorrelatedRecovery {
    /// Create a correlated recovery model.
    ///
    /// # Arguments
    /// * `mean` - Mean recovery rate, clamped to [0.05, 0.95]. Typical: 0.40
    /// * `vol` - Recovery volatility, clamped to [0.0, 0.50]. Typical: 0.20-0.30
    /// * `corr` - Correlation with market factor, clamped to [-1.0, 1.0]. Typical: -0.30 to -0.50
    ///
    /// # Returns
    ///
    /// A bounded stochastic recovery model with default bounds `[0.0, 1.0]`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_correlation::{CorrelatedRecovery, RecoveryModel};
    ///
    /// let model = CorrelatedRecovery::new(0.40, 0.25, -0.40);
    /// let mean = model.conditional_recovery(0.0);
    ///
    /// assert!((mean - 0.40).abs() < 1e-12);
    /// ```
    #[must_use]
    pub fn new(mean: f64, vol: f64, corr: f64) -> Self {
        Self {
            mean_recovery: mean.clamp(0.05, 0.95),
            recovery_volatility: vol.clamp(0.0, 0.50),
            factor_correlation: corr.clamp(-1.0, 1.0),
            min_recovery: 0.0,
            max_recovery: 1.0,
        }
    }

    /// Create with custom bounds.
    ///
    /// # Arguments
    /// * `mean` - Mean recovery rate
    /// * `vol` - Recovery volatility
    /// * `corr` - Correlation with market factor
    /// * `min` - Minimum recovery (floor), clamped to [0.0, 0.5]
    /// * `max` - Maximum recovery (ceiling), clamped to [0.5, 1.0]
    ///
    /// # Returns
    ///
    /// A bounded stochastic recovery model with caller-specified recovery bounds.
    #[must_use]
    pub fn with_bounds(mean: f64, vol: f64, corr: f64, min: f64, max: f64) -> Self {
        let mut model = Self::new(mean, vol, corr);
        model.min_recovery = min.clamp(0.0, 0.5);
        model.max_recovery = max.clamp(0.5, 1.0);
        model
    }

    /// Market-standard calibration from CDX equity tranche.
    ///
    /// Parameters:
    /// - Mean: 40%
    /// - Vol: 25%
    /// - Correlation: -40%
    ///
    /// # Returns
    ///
    /// The default stochastic-recovery calibration used by this crate.
    #[must_use]
    pub fn market_standard() -> Self {
        Self::new(0.40, 0.25, -0.40)
    }

    /// Conservative calibration with higher vol and correlation.
    ///
    /// Parameters:
    /// - Mean: 40%
    /// - Vol: 30%
    /// - Correlation: -50%
    ///
    /// # Returns
    ///
    /// A higher-volatility, more factor-sensitive stochastic-recovery calibration.
    #[must_use]
    pub fn conservative() -> Self {
        Self::new(0.40, 0.30, -0.50)
    }

    /// Get the mean recovery rate.
    ///
    /// # Returns
    ///
    /// The target mean recovery in decimal form.
    #[must_use]
    pub fn mean(&self) -> f64 {
        self.mean_recovery
    }

    /// Get the recovery volatility.
    ///
    /// # Returns
    ///
    /// The recovery-volatility scale in decimal form.
    #[must_use]
    pub fn volatility(&self) -> f64 {
        self.recovery_volatility
    }

    /// Get the factor correlation.
    ///
    /// # Returns
    ///
    /// The signed factor-sensitivity parameter.
    #[must_use]
    pub fn correlation(&self) -> f64 {
        self.factor_correlation
    }

    fn logistic_bounded_recovery(&self, shock: f64) -> f64 {
        let width = (self.max_recovery - self.min_recovery).max(f64::EPSILON);
        let mean = self
            .mean_recovery
            .clamp(self.min_recovery + 1e-9, self.max_recovery - 1e-9);
        let p = ((mean - self.min_recovery) / width).clamp(1e-9, 1.0 - 1e-9);
        let center = (p / (1.0 - p)).ln();
        let local_slope = (width * p * (1.0 - p)).max(1e-9);
        let squashed = 1.0 / (1.0 + (-(center + shock / local_slope)).exp());
        self.min_recovery + width * squashed
    }
}

impl RecoveryModel for CorrelatedRecovery {
    fn expected_recovery(&self) -> f64 {
        self.mean_recovery
    }

    fn conditional_recovery(&self, market_factor: f64) -> f64 {
        let shock = self.factor_correlation * self.recovery_volatility * market_factor;
        self.logistic_bounded_recovery(shock)
    }

    fn recovery_volatility(&self) -> f64 {
        self.recovery_volatility
    }

    fn model_name(&self) -> &'static str {
        "Market-Correlated Stochastic Recovery"
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_correlated_recovery_creation() {
        let model = CorrelatedRecovery::new(0.40, 0.25, -0.40);
        assert!((model.mean() - 0.40).abs() < 1e-10);
        assert!((model.volatility() - 0.25).abs() < 1e-10);
        assert!((model.correlation() - (-0.40)).abs() < 1e-10);
    }

    #[test]
    fn test_conditional_recovery_in_stress() {
        let model = CorrelatedRecovery::market_standard();

        let stress_recovery = model.conditional_recovery(-2.0);

        assert!(
            stress_recovery > model.expected_recovery(),
            "Stress with negative recovery correlation should raise recovery in this sign convention"
        );
    }

    #[test]
    fn test_conditional_recovery_varies() {
        let model = CorrelatedRecovery::market_standard();

        let r_neg = model.conditional_recovery(-2.0);
        let r_zero = model.conditional_recovery(0.0);
        let r_pos = model.conditional_recovery(2.0);

        // With negative correlation:
        // - Negative Z gives higher recovery
        // - Positive Z gives lower recovery
        assert!(r_neg > r_zero, "Neg Z should give higher recovery");
        assert!(r_pos < r_zero, "Pos Z should give lower recovery");
    }

    #[test]
    fn test_mean_recovery_at_zero_factor() {
        let model = CorrelatedRecovery::market_standard();

        // At Z=0, conditional should equal mean
        let r_at_zero = model.conditional_recovery(0.0);
        assert!(
            (r_at_zero - model.expected_recovery()).abs() < 1e-10,
            "Recovery at Z=0 should equal mean"
        );
    }

    #[test]
    fn test_recovery_bounded() {
        let model = CorrelatedRecovery::new(0.40, 0.30, -0.50);

        // Even with extreme factors, recovery should be bounded
        let extreme_neg = model.conditional_recovery(-10.0);
        let extreme_pos = model.conditional_recovery(10.0);

        assert!(
            (0.0..=1.0).contains(&extreme_neg),
            "Recovery {} should be in [0, 1]",
            extreme_neg
        );
        assert!(
            (0.0..=1.0).contains(&extreme_pos),
            "Recovery {} should be in [0, 1]",
            extreme_pos
        );
        assert!(
            extreme_neg < 1.0,
            "smooth bounding should avoid hard ceiling clamp"
        );
        assert!(
            extreme_pos > 0.0,
            "smooth bounding should avoid hard floor clamp"
        );
    }

    #[test]
    fn test_is_stochastic() {
        let model = CorrelatedRecovery::market_standard();
        assert!(model.is_stochastic());
        assert!(model.recovery_volatility() > 0.0);
    }

    #[test]
    fn test_zero_volatility_is_constant() {
        let model = CorrelatedRecovery::new(0.40, 0.0, -0.40);

        // With zero volatility, should behave like constant
        let r_neg = model.conditional_recovery(-2.0);
        let r_pos = model.conditional_recovery(2.0);

        assert!(
            (r_neg - r_pos).abs() < 1e-10,
            "Zero vol should give constant recovery"
        );
    }

    #[test]
    fn test_zero_correlation_is_constant() {
        let model = CorrelatedRecovery::new(0.40, 0.25, 0.0);

        // With zero correlation, should behave like constant
        let r_neg = model.conditional_recovery(-2.0);
        let r_pos = model.conditional_recovery(2.0);

        assert!(
            (r_neg - r_pos).abs() < 1e-10,
            "Zero correlation should give constant recovery"
        );
    }

    #[test]
    fn test_lgd_calculation() {
        let model = CorrelatedRecovery::market_standard();

        assert!((model.lgd() - 0.60).abs() < 1e-10);

        // Conditional LGD at Z=0 should equal expected LGD
        assert!((model.conditional_lgd(0.0) - 0.60).abs() < 1e-10);
    }

    #[test]
    fn test_market_standard_and_conservative() {
        let standard = CorrelatedRecovery::market_standard();
        let conservative = CorrelatedRecovery::conservative();

        // Conservative should have higher vol
        assert!(conservative.volatility() > standard.volatility());

        // Conservative should have stronger negative correlation
        assert!(conservative.correlation() < standard.correlation());
    }
}
