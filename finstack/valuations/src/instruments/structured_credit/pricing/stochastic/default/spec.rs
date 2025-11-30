//! Stochastic default specification.
//!
//! Provides a serializable specification enum for stochastic default models,
//! enabling configuration and deferred construction.

use super::{CopulaBasedDefault, IntensityProcessDefault, StochasticDefault};
use crate::cashflow::builder::specs::DefaultModelSpec;
use crate::instruments::common::models::correlation::copula::CopulaSpec;

/// Stochastic default model specification.
///
/// Allows default model selection and configuration without
/// constructing the full model.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "model", deny_unknown_fields))]
pub enum StochasticDefaultSpec {
    /// Use deterministic default model (no stochastic component).
    Deterministic(DefaultModelSpec),

    /// Copula-based default correlation model.
    ///
    /// Uses Li (2000) framework with specified copula.
    Copula {
        /// Base annual CDR
        base_cdr: f64,
        /// Copula specification
        copula_spec: CopulaSpec,
        /// Asset correlation
        correlation: f64,
    },

    /// Intensity process (Cox model) for default.
    ///
    /// Mean-reverting intensity with factor sensitivity.
    IntensityProcess {
        /// Base annual hazard rate
        base_hazard: f64,
        /// Sensitivity to systematic factor
        factor_sensitivity: f64,
        /// Mean reversion speed
        mean_reversion: f64,
        /// Intensity volatility
        volatility: f64,
        /// Asset correlation
        #[cfg_attr(feature = "serde", serde(default = "default_correlation"))]
        correlation: f64,
    },

    /// Factor-correlated CDR model.
    ///
    /// Simple model that shocks base CDR by systematic factor.
    FactorCorrelated {
        /// Base deterministic default specification
        base_spec: DefaultModelSpec,
        /// Factor loading
        factor_loading: f64,
        /// CDR volatility
        cdr_volatility: f64,
    },
}

#[cfg(feature = "serde")]
fn default_correlation() -> f64 {
    0.20
}

impl Default for StochasticDefaultSpec {
    fn default() -> Self {
        StochasticDefaultSpec::Deterministic(DefaultModelSpec::cdr_2pct())
    }
}

impl StochasticDefaultSpec {
    /// Create a deterministic (non-stochastic) default spec.
    pub fn deterministic(spec: DefaultModelSpec) -> Self {
        StochasticDefaultSpec::Deterministic(spec)
    }

    /// Create a copula-based default spec with Gaussian copula.
    pub fn gaussian_copula(base_cdr: f64, correlation: f64) -> Self {
        StochasticDefaultSpec::Copula {
            base_cdr: base_cdr.clamp(0.0, 1.0),
            copula_spec: CopulaSpec::Gaussian,
            correlation: correlation.clamp(0.0, 0.99),
        }
    }

    /// Create a copula-based default spec with Student-t copula.
    pub fn student_t_copula(base_cdr: f64, correlation: f64, degrees_of_freedom: f64) -> Self {
        StochasticDefaultSpec::Copula {
            base_cdr: base_cdr.clamp(0.0, 1.0),
            copula_spec: CopulaSpec::StudentT { degrees_of_freedom },
            correlation: correlation.clamp(0.0, 0.99),
        }
    }

    /// Create an intensity process default spec.
    pub fn intensity_process(
        base_hazard: f64,
        factor_sensitivity: f64,
        mean_reversion: f64,
        volatility: f64,
    ) -> Self {
        StochasticDefaultSpec::IntensityProcess {
            base_hazard: base_hazard.clamp(0.0, 1.0),
            factor_sensitivity: factor_sensitivity.clamp(-2.0, 2.0),
            mean_reversion: mean_reversion.clamp(0.0, 10.0),
            volatility: volatility.clamp(0.0, 2.0),
            correlation: 0.20,
        }
    }

    /// Create a factor-correlated default spec.
    pub fn factor_correlated(
        base_spec: DefaultModelSpec,
        factor_loading: f64,
        cdr_volatility: f64,
    ) -> Self {
        StochasticDefaultSpec::FactorCorrelated {
            base_spec,
            factor_loading: factor_loading.clamp(-1.0, 1.0),
            cdr_volatility: cdr_volatility.clamp(0.0, 1.0),
        }
    }

    /// RMBS standard calibration.
    pub fn rmbs_standard() -> Self {
        StochasticDefaultSpec::Copula {
            base_cdr: 0.02,
            copula_spec: CopulaSpec::Gaussian,
            correlation: 0.05,
        }
    }

    /// CLO standard calibration.
    pub fn clo_standard() -> Self {
        StochasticDefaultSpec::Copula {
            base_cdr: 0.03,
            copula_spec: CopulaSpec::Gaussian,
            correlation: 0.20,
        }
    }

    /// Build the stochastic default model from this specification.
    ///
    /// Returns None for deterministic specs.
    pub fn build(&self) -> Option<Box<dyn StochasticDefault>> {
        match self {
            StochasticDefaultSpec::Deterministic(_) => None,

            StochasticDefaultSpec::Copula {
                base_cdr,
                copula_spec,
                correlation,
            } => Some(Box::new(CopulaBasedDefault::new(
                *base_cdr,
                copula_spec.clone(),
                *correlation,
            ))),

            StochasticDefaultSpec::IntensityProcess {
                base_hazard,
                factor_sensitivity,
                mean_reversion,
                volatility,
                correlation,
            } => Some(Box::new(
                IntensityProcessDefault::new(
                    *base_hazard,
                    *factor_sensitivity,
                    *mean_reversion,
                    *volatility,
                )
                .with_correlation(*correlation),
            )),

            StochasticDefaultSpec::FactorCorrelated { .. } => {
                // Factor-correlated default would need a separate implementation
                // For now, use copula-based as fallback
                None
            }
        }
    }

    /// Check if this is a stochastic specification.
    pub fn is_stochastic(&self) -> bool {
        !matches!(self, StochasticDefaultSpec::Deterministic(_))
    }

    /// Get the correlation if this is a stochastic model.
    pub fn correlation(&self) -> Option<f64> {
        match self {
            StochasticDefaultSpec::Deterministic(_) => None,
            StochasticDefaultSpec::Copula { correlation, .. } => Some(*correlation),
            StochasticDefaultSpec::IntensityProcess { correlation, .. } => Some(*correlation),
            StochasticDefaultSpec::FactorCorrelated { factor_loading, .. } => Some(*factor_loading),
        }
    }

    /// Get the base CDR/hazard rate.
    pub fn base_rate(&self) -> f64 {
        match self {
            StochasticDefaultSpec::Deterministic(spec) => spec.cdr,
            StochasticDefaultSpec::Copula { base_cdr, .. } => *base_cdr,
            StochasticDefaultSpec::IntensityProcess { base_hazard, .. } => *base_hazard,
            StochasticDefaultSpec::FactorCorrelated { base_spec, .. } => base_spec.cdr,
        }
    }

    /// Get the base MDR (monthly default rate) for this specification.
    ///
    /// Returns the unconditional expected MDR before factor shocks are applied.
    pub fn base_mdr(&self) -> f64 {
        let annual_cdr = self.base_rate();
        // Convert annual CDR to monthly MDR
        // MDR = 1 - (1 - CDR)^(1/12)
        if annual_cdr <= 0.0 {
            0.0
        } else if annual_cdr >= 1.0 {
            1.0
        } else {
            1.0 - (1.0 - annual_cdr).powf(1.0 / 12.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spec_default() {
        let spec = StochasticDefaultSpec::default();
        assert!(!spec.is_stochastic());
    }

    #[test]
    fn test_gaussian_copula_spec() {
        let spec = StochasticDefaultSpec::gaussian_copula(0.02, 0.20);

        assert!(spec.is_stochastic());
        assert_eq!(spec.correlation(), Some(0.20));
        assert!((spec.base_rate() - 0.02).abs() < 1e-10);

        let model = spec.build();
        assert!(model.is_some());
    }

    #[test]
    fn test_intensity_process_spec() {
        let spec = StochasticDefaultSpec::intensity_process(0.02, 0.5, 0.5, 0.30);

        assert!(spec.is_stochastic());

        let model = spec.build();
        assert!(model.is_some());
        let model = model.expect("Should build intensity process model");
        assert_eq!(model.model_name(), "Intensity Process Default Model");
    }

    #[test]
    fn test_standard_calibrations() {
        let rmbs = StochasticDefaultSpec::rmbs_standard();
        assert!(rmbs.is_stochastic());
        assert!((rmbs.base_rate() - 0.02).abs() < 1e-10);

        let clo = StochasticDefaultSpec::clo_standard();
        let clo_corr = clo.correlation().expect("CLO should have correlation");
        let rmbs_corr = rmbs.correlation().expect("RMBS should have correlation");
        assert!(clo_corr > rmbs_corr);
    }

    #[test]
    fn test_deterministic_build_returns_none() {
        let spec = StochasticDefaultSpec::deterministic(DefaultModelSpec::constant_cdr(0.02));

        assert!(!spec.is_stochastic());
        assert!(spec.build().is_none());
    }
}
