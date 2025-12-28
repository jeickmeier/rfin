//! Stochastic recovery models for credit portfolio pricing.
//!
//! Recovery rates empirically decrease in stressed markets (negative correlation
//! with default intensity). This is critical for senior tranches which are
//! sensitive to realized recovery at the time of defaults.
//!
//! # Constant vs Stochastic Recovery
//!
//! - **Constant**: Simplest model, uses fixed recovery (e.g., 40%)
//! - **Stochastic**: Recovery varies with market conditions
//!
//! # Stochastic Recovery Models
//!
//! ## Market-Correlated (Andersen-Sidenius)
//!
//! Recovery negatively correlated with the systematic factor:
//! ```text
//! R(Z) = μ_R + ρ_R · σ_R · Z
//! ```
//! where ρ_R < 0 (typically -0.3 to -0.5).
//!
//! This captures the "double hit" effect: defaults cluster AND recovery
//! falls simultaneously in stressed environments.
//!
//! ## Beta-Distributed
//!
//! Recovery bounded to [0, 1] with specified mean and variance:
//! ```text
//! R ~ Beta(α, β) with E[R] = μ, Var(R) = σ²
//! ```
//!
//! ## Frye Model (LGD-Default Correlation)
//!
//! LGD as function of portfolio default rate:
//! ```text
//! LGD(DR) = α + β · DR
//! ```
//!
//! # References
//!
//! - Altman, E., et al. (2005). "The Link between Default and Recovery Rates."
//!   *Journal of Business*, 78(6).
//! - Andersen, L., & Sidenius, J. (2005). "Extensions to the Gaussian Copula."
//! - Amraoui, S., & Hitier, S. (2008). "Optimal Stochastic Recovery for Base
//!   Correlation."
//! - Frye, J. (2000). "Depressing Recoveries." *Risk*, November 2000.

mod constant;
mod correlated;

pub use constant::ConstantRecovery;
pub use correlated::CorrelatedRecovery;

/// Recovery rate model for credit portfolio pricing.
///
/// Implementations provide both unconditional expected recovery
/// and recovery conditional on market state.
pub trait RecoveryModel: Send + Sync + std::fmt::Debug {
    /// Expected (unconditional) recovery rate.
    ///
    /// This is the average recovery used for simple calculations
    /// and as the baseline for stochastic models.
    fn expected_recovery(&self) -> f64;

    /// Recovery rate conditional on the systematic market factor.
    ///
    /// For stochastic models, recovery varies with market state:
    /// - Low Z (stressed market): lower recovery
    /// - High Z (good market): higher recovery
    ///
    /// For constant models, this equals expected_recovery().
    fn conditional_recovery(&self, market_factor: f64) -> f64;

    /// Loss given default = 1 - recovery.
    fn lgd(&self) -> f64 {
        1.0 - self.expected_recovery()
    }

    /// Conditional LGD given market factor.
    fn conditional_lgd(&self, market_factor: f64) -> f64 {
        1.0 - self.conditional_recovery(market_factor)
    }

    /// Recovery rate standard deviation (volatility).
    ///
    /// Returns 0 for constant models.
    fn recovery_volatility(&self) -> f64;

    /// Model name for diagnostics.
    fn model_name(&self) -> &'static str;

    /// Whether this model is stochastic (varies with market factor).
    fn is_stochastic(&self) -> bool {
        self.recovery_volatility() > 0.0
    }
}

/// Recovery model specification for configuration and serialization.
///
/// Allows recovery model selection without constructing the full model.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "type", deny_unknown_fields))]
pub enum RecoverySpec {
    /// Constant recovery rate (current default behavior).
    Constant {
        /// Recovery rate ∈ [0, 1]
        rate: f64,
    },

    /// Recovery correlated with market factor (Andersen-Sidenius model).
    ///
    /// R(Z) = mean + correlation * volatility * Z
    MarketCorrelated {
        /// Mean recovery rate
        mean_recovery: f64,
        /// Recovery volatility (standard deviation)
        recovery_volatility: f64,
        /// Correlation with systematic factor (typically negative)
        factor_correlation: f64,
    },

    /// Beta-distributed recovery (bounded [0, 1]).
    Beta {
        /// Mean recovery
        mean: f64,
        /// Standard deviation
        std_dev: f64,
    },

    /// Frye model: LGD = α + β * DefaultRate
    Frye {
        /// Base LGD when default rate is zero
        base_lgd: f64,
        /// LGD sensitivity to default rate
        lgd_sensitivity: f64,
    },
}

impl Default for RecoverySpec {
    fn default() -> Self {
        RecoverySpec::Constant { rate: 0.40 }
    }
}

impl RecoverySpec {
    /// Create constant recovery specification.
    ///
    /// # Arguments
    /// * `rate` - Recovery rate, clamped to [0.0, 1.0]
    #[must_use]
    pub fn constant(rate: f64) -> Self {
        RecoverySpec::Constant {
            rate: rate.clamp(0.0, 1.0),
        }
    }

    /// Create market-correlated recovery specification.
    ///
    /// # Arguments
    /// * `mean` - Mean recovery rate, clamped to [0.0, 1.0]. Typical: 0.40
    /// * `vol` - Recovery volatility, clamped to [0.0, 0.5]. Typical: 0.20-0.30
    /// * `corr` - Correlation with factor, clamped to [-1.0, 1.0]. Typical: -0.30 to -0.50
    #[must_use]
    pub fn market_correlated(mean: f64, vol: f64, corr: f64) -> Self {
        RecoverySpec::MarketCorrelated {
            mean_recovery: mean.clamp(0.0, 1.0),
            recovery_volatility: vol.clamp(0.0, 0.5),
            factor_correlation: corr.clamp(-1.0, 1.0),
        }
    }

    /// Create market-standard stochastic recovery.
    ///
    /// Uses typical calibration from CDX equity tranche:
    /// - Mean: 40%
    /// - Vol: 25%
    /// - Correlation: -40%
    #[must_use]
    pub fn market_standard_stochastic() -> Self {
        RecoverySpec::market_correlated(0.40, 0.25, -0.40)
    }

    /// Create beta-distributed recovery specification.
    ///
    /// Note: Currently approximated via correlated model.
    ///
    /// # Arguments
    /// * `mean` - Mean recovery, clamped to [0.05, 0.95]
    /// * `std_dev` - Standard deviation, clamped to [0.01, 0.30]
    #[must_use]
    pub fn beta(mean: f64, std_dev: f64) -> Self {
        RecoverySpec::Beta {
            mean: mean.clamp(0.05, 0.95),
            std_dev: std_dev.clamp(0.01, 0.30),
        }
    }

    /// Create Frye model specification.
    ///
    /// LGD(DR) = base_lgd + sensitivity * DefaultRate
    ///
    /// # Arguments
    /// * `base_lgd` - Base LGD when default rate is zero, clamped to [0.0, 1.0]
    /// * `sensitivity` - LGD sensitivity to default rate, clamped to [0.0, 5.0]
    #[must_use]
    pub fn frye(base_lgd: f64, sensitivity: f64) -> Self {
        RecoverySpec::Frye {
            base_lgd: base_lgd.clamp(0.0, 1.0),
            lgd_sensitivity: sensitivity.clamp(0.0, 5.0),
        }
    }

    /// Build the recovery model instance from this specification.
    #[must_use]
    pub fn build(&self) -> Box<dyn RecoveryModel> {
        match self {
            RecoverySpec::Constant { rate } => Box::new(ConstantRecovery::new(*rate)),
            RecoverySpec::MarketCorrelated {
                mean_recovery,
                recovery_volatility,
                factor_correlation,
            } => Box::new(CorrelatedRecovery::new(
                *mean_recovery,
                *recovery_volatility,
                *factor_correlation,
            )),
            RecoverySpec::Beta { mean, std_dev } => {
                // Beta approximated via correlated model
                // (full beta would require different integration)
                Box::new(CorrelatedRecovery::new(*mean, *std_dev, 0.0))
            }
            RecoverySpec::Frye {
                base_lgd,
                lgd_sensitivity,
            } => {
                // Frye model: LGD(DR) = base + sens * DR
                // Approximate via correlated recovery with negative correlation
                let mean_recovery = 1.0 - base_lgd;
                // Sensitivity translates to correlation in stressed scenarios
                let implied_vol = lgd_sensitivity * 0.1;
                Box::new(CorrelatedRecovery::new(mean_recovery, implied_vol, -0.5))
            }
        }
    }

    /// Get expected recovery rate from specification.
    #[must_use]
    pub fn expected_recovery(&self) -> f64 {
        match self {
            RecoverySpec::Constant { rate } => *rate,
            RecoverySpec::MarketCorrelated { mean_recovery, .. } => *mean_recovery,
            RecoverySpec::Beta { mean, .. } => *mean,
            RecoverySpec::Frye { base_lgd, .. } => 1.0 - base_lgd,
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_recovery_spec_default() {
        let spec = RecoverySpec::default();
        assert!(matches!(spec, RecoverySpec::Constant { rate } if (rate - 0.40).abs() < 1e-10));
    }

    #[test]
    fn test_recovery_spec_builders() {
        let constant = RecoverySpec::constant(0.35);
        assert!((constant.expected_recovery() - 0.35).abs() < 1e-10);

        let correlated = RecoverySpec::market_correlated(0.40, 0.25, -0.40);
        assert!((correlated.expected_recovery() - 0.40).abs() < 1e-10);

        let market_std = RecoverySpec::market_standard_stochastic();
        assert!((market_std.expected_recovery() - 0.40).abs() < 1e-10);
    }

    #[test]
    fn test_recovery_spec_clamping() {
        // Rate should be clamped to [0, 1]
        let high = RecoverySpec::constant(1.5);
        assert!(matches!(high, RecoverySpec::Constant { rate } if rate <= 1.0));

        let low = RecoverySpec::constant(-0.1);
        assert!(matches!(low, RecoverySpec::Constant { rate } if rate >= 0.0));
    }

    #[test]
    fn test_recovery_spec_build() {
        let specs = vec![
            RecoverySpec::constant(0.40),
            RecoverySpec::market_correlated(0.40, 0.25, -0.40),
            RecoverySpec::beta(0.40, 0.15),
            RecoverySpec::frye(0.60, 1.5),
        ];

        for spec in specs {
            let model = spec.build();
            assert!(model.expected_recovery() >= 0.0);
            assert!(model.expected_recovery() <= 1.0);
            assert!(!model.model_name().is_empty());
        }
    }

    #[test]
    fn test_lgd_calculation() {
        let model = ConstantRecovery::new(0.40);
        assert!((model.lgd() - 0.60).abs() < 1e-10);
    }
}
