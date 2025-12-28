//! Market-correlated stochastic recovery model (Andersen-Sidenius).
//!
//! Models recovery as inversely correlated with the systematic factor:
//! - In good markets (high Z): recovery is higher
//! - In stressed markets (low Z): recovery is lower
//!
//! This captures the "double hit" effect where defaults cluster AND
//! recovery rates fall simultaneously in stressed environments.
//!
//! # Mathematical Model
//!
//! ```text
//! R(Z) = μ_R + ρ_R · σ_R · Z
//! ```
//!
//! where:
//! - μ_R is mean recovery
//! - σ_R is recovery volatility
//! - ρ_R is correlation with systematic factor (typically negative)
//! - Z is the systematic market factor from the copula
//!
//! The result is clamped to [0, 1] to ensure valid recovery rates.
//!
//! # Impact on Tranches
//!
//! - **Equity tranches**: Increased losses (first hit by low recovery)
//! - **Mezzanine tranches**: Losses compound faster
//! - **Senior tranches**: Significant impact when losses reach them
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
//! - Andersen, L., & Sidenius, J. (2005). "Extensions to the Gaussian Copula:
//!   Random Recovery and Random Factor Loadings." *Journal of Credit Risk*.
//! - Krekel, M., & Stumpp, P. (2006). "Pricing Correlation Products: CDOs."

use super::RecoveryModel;

/// Market-correlated stochastic recovery model.
///
/// Recovery varies with the systematic market factor, capturing
/// the empirical negative correlation between defaults and recovery.
#[derive(Clone, Debug)]
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
    #[must_use]
    pub fn conservative() -> Self {
        Self::new(0.40, 0.30, -0.50)
    }

    /// Get the mean recovery rate.
    #[must_use]
    pub fn mean(&self) -> f64 {
        self.mean_recovery
    }

    /// Get the recovery volatility.
    #[must_use]
    pub fn volatility(&self) -> f64 {
        self.recovery_volatility
    }

    /// Get the factor correlation.
    #[must_use]
    pub fn correlation(&self) -> f64 {
        self.factor_correlation
    }
}

impl RecoveryModel for CorrelatedRecovery {
    fn expected_recovery(&self) -> f64 {
        self.mean_recovery
    }

    fn conditional_recovery(&self, market_factor: f64) -> f64 {
        // R(Z) = μ + ρ · σ · Z
        let recovery =
            self.mean_recovery + self.factor_correlation * self.recovery_volatility * market_factor;

        // Clamp to valid range
        recovery.clamp(self.min_recovery, self.max_recovery)
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

        // Mathematical note on sign convention:
        // R(Z) = μ_R + ρ_R * σ_R * Z
        // With ρ_R = -0.40 (negative correlation) and Z = -2 (stress):
        // R = 0.40 + (-0.40) * 0.25 * (-2) = 0.40 + 0.20 = 0.60

        let stress_recovery = model.conditional_recovery(-2.0);
        let expected = 0.40 + (-0.40) * 0.25 * (-2.0); // = 0.60

        assert!(
            (stress_recovery - expected).abs() < 1e-10,
            "Recovery {} should equal expected {}",
            stress_recovery,
            expected
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
