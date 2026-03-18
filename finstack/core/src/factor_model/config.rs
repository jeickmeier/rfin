use super::types::{FactorId, FactorType};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeMap;

/// Strategy used when extracting factor sensitivities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PricingMode {
    /// Use central finite differences to approximate linear deltas.
    DeltaBased,
    /// Reprice across a scenario grid and derive deltas from the P&L profile.
    FullRepricing,
}

/// Risk measure used when aggregating factor exposures.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskMeasure {
    /// Aggregate exposures using factor covariance and portfolio variance.
    Variance,
    /// Aggregate exposures using portfolio volatility.
    Volatility,
    /// Aggregate exposures using Value at Risk at a fixed one-sided loss confidence level.
    #[serde(rename = "var")]
    VaR {
        /// Confidence level in the open interval `(0.5, 1)`.
        confidence: f64,
    },
    /// Aggregate exposures using expected shortfall at a fixed one-sided loss confidence level.
    ExpectedShortfall {
        /// Confidence level in the open interval `(0.5, 1)`.
        confidence: f64,
    },
}

impl Default for RiskMeasure {
    fn default() -> Self {
        Self::Variance
    }
}

impl RiskMeasure {
    /// Validate any embedded confidence levels before downstream risk calculations use them.
    pub fn validate(&self) -> crate::Result<()> {
        match self {
            Self::Variance | Self::Volatility => Ok(()),
            Self::VaR { confidence } | Self::ExpectedShortfall { confidence } => {
                validate_confidence(*confidence)
            }
        }
    }
}

fn validate_confidence(confidence: f64) -> crate::Result<()> {
    if confidence.is_finite() && confidence > 0.5 && confidence < 1.0 {
        Ok(())
    } else {
        Err(crate::Error::Validation(format!(
            "RiskMeasure confidence must be in the open interval (0.5, 1), got {confidence}"
        )))
    }
}

impl<'de> Deserialize<'de> for RiskMeasure {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "snake_case")]
        enum RiskMeasureSerde {
            Variance,
            Volatility,
            #[serde(rename = "var")]
            VaR {
                confidence: f64,
            },
            ExpectedShortfall {
                confidence: f64,
            },
        }

        let measure = match RiskMeasureSerde::deserialize(deserializer)? {
            RiskMeasureSerde::Variance => Self::Variance,
            RiskMeasureSerde::Volatility => Self::Volatility,
            RiskMeasureSerde::VaR { confidence } => Self::VaR { confidence },
            RiskMeasureSerde::ExpectedShortfall { confidence } => {
                Self::ExpectedShortfall { confidence }
            }
        };

        measure.validate().map_err(serde::de::Error::custom)?;
        Ok(measure)
    }
}

/// Per-factor-type bump magnitudes for finite-difference sensitivity engines.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BumpSizeConfig {
    /// Default rates bump in basis points.
    #[serde(default = "default_one")]
    pub rates_bp: f64,
    /// Default credit bump in basis points.
    #[serde(default = "default_one")]
    pub credit_bp: f64,
    /// Default equity spot bump in percent.
    #[serde(default = "default_one")]
    pub equity_pct: f64,
    /// Default FX spot bump in percent.
    #[serde(default = "default_one")]
    pub fx_pct: f64,
    /// Default volatility bump in absolute vol points.
    #[serde(default = "default_one")]
    pub vol_points: f64,
    /// Per-factor overrides that take precedence over factor-type defaults.
    #[serde(default)]
    pub overrides: BTreeMap<FactorId, f64>,
}

fn default_one() -> f64 {
    1.0
}

impl Default for BumpSizeConfig {
    fn default() -> Self {
        Self {
            rates_bp: 1.0,
            credit_bp: 1.0,
            equity_pct: 1.0,
            fx_pct: 1.0,
            vol_points: 1.0,
            overrides: BTreeMap::new(),
        }
    }
}

impl BumpSizeConfig {
    /// Return the configured bump size for `factor_id`, checking overrides first.
    #[must_use]
    pub fn bump_size_for_factor(&self, factor_id: &FactorId, factor_type: &FactorType) -> f64 {
        if let Some(&size) = self.overrides.get(factor_id) {
            return size;
        }

        match factor_type {
            FactorType::Rates | FactorType::Inflation | FactorType::Custom(_) => self.rates_bp,
            FactorType::Credit => self.credit_bp,
            FactorType::Equity | FactorType::Commodity => self.equity_pct,
            FactorType::FX => self.fx_pct,
            FactorType::Volatility => self.vol_points,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_risk_measure_serde_roundtrip_for_all_variants() {
        let cases = [
            (RiskMeasure::Variance, "\"variance\""),
            (RiskMeasure::Volatility, "\"volatility\""),
            (
                RiskMeasure::VaR { confidence: 0.99 },
                r#"{"var":{"confidence":0.99}}"#,
            ),
            (
                RiskMeasure::ExpectedShortfall { confidence: 0.975 },
                r#"{"expected_shortfall":{"confidence":0.975}}"#,
            ),
        ];

        for (measure, expected_json) in cases {
            let json_result = serde_json::to_string(&measure);
            assert!(json_result.is_ok());
            let Ok(json) = json_result else {
                return;
            };

            assert_eq!(json, expected_json);

            let back_result: Result<RiskMeasure, _> = serde_json::from_str(&json);
            assert!(back_result.is_ok());
            let Ok(back) = back_result else {
                return;
            };

            assert_eq!(measure, back);
        }
    }

    #[test]
    fn test_risk_measure_default_is_variance() {
        assert_eq!(RiskMeasure::default(), RiskMeasure::Variance);
    }

    #[test]
    fn test_risk_measure_validate_rejects_invalid_confidence() {
        let invalid_measures = [
            RiskMeasure::VaR { confidence: 0.1 },
            RiskMeasure::VaR { confidence: 0.5 },
            RiskMeasure::VaR { confidence: 0.0 },
            RiskMeasure::VaR { confidence: 1.0 },
            RiskMeasure::ExpectedShortfall { confidence: 0.25 },
            RiskMeasure::ExpectedShortfall { confidence: 0.5 },
            RiskMeasure::ExpectedShortfall { confidence: -0.1 },
            RiskMeasure::ExpectedShortfall { confidence: 1.1 },
        ];

        for measure in invalid_measures {
            assert!(measure.validate().is_err());
        }
    }

    #[test]
    fn test_risk_measure_serde_rejects_invalid_confidence() {
        let invalid_payloads = [
            r#"{"var":{"confidence":0.1}}"#,
            r#"{"var":{"confidence":0.5}}"#,
            r#"{"var":{"confidence":0.0}}"#,
            r#"{"var":{"confidence":1.0}}"#,
            r#"{"expected_shortfall":{"confidence":0.25}}"#,
            r#"{"expected_shortfall":{"confidence":0.5}}"#,
            r#"{"expected_shortfall":{"confidence":-0.1}}"#,
            r#"{"expected_shortfall":{"confidence":1.1}}"#,
        ];

        for payload in invalid_payloads {
            let result: Result<RiskMeasure, _> = serde_json::from_str(payload);
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_pricing_mode_serde() {
        let mode = PricingMode::DeltaBased;
        let json_result = serde_json::to_string(&mode);
        assert!(json_result.is_ok());
        let Ok(json) = json_result else {
            return;
        };

        let back_result: Result<PricingMode, _> = serde_json::from_str(&json);
        assert!(back_result.is_ok());
        let Ok(back) = back_result else {
            return;
        };

        assert_eq!(mode, back);
    }

    #[test]
    fn test_bump_size_config_defaults() {
        let config = BumpSizeConfig::default();
        assert!((config.rates_bp - 1.0).abs() < 1e-12);
        assert!((config.credit_bp - 1.0).abs() < 1e-12);
        assert!((config.equity_pct - 1.0).abs() < 1e-12);
        assert!((config.fx_pct - 1.0).abs() < 1e-12);
        assert!((config.vol_points - 1.0).abs() < 1e-12);
        assert!(config.overrides.is_empty());
    }

    #[test]
    fn test_bump_size_for_factor_override() {
        let mut config = BumpSizeConfig::default();
        config.overrides.insert(FactorId::new("USD-Rates"), 0.5);

        let overridden =
            config.bump_size_for_factor(&FactorId::new("USD-Rates"), &FactorType::Rates);
        assert!((overridden - 0.5).abs() < 1e-12);

        let fallback = config.bump_size_for_factor(&FactorId::new("EUR-Rates"), &FactorType::Rates);
        assert!((fallback - 1.0).abs() < 1e-12);
    }

    #[test]
    fn test_bump_size_config_serde() {
        let config = BumpSizeConfig::default();
        let json_result = serde_json::to_string(&config);
        assert!(json_result.is_ok());
        let Ok(json) = json_result else {
            return;
        };

        let back_result: Result<BumpSizeConfig, _> = serde_json::from_str(&json);
        assert!(back_result.is_ok());
        let Ok(back) = back_result else {
            return;
        };

        assert!((config.rates_bp - back.rates_bp).abs() < 1e-12);
        assert!((config.credit_bp - back.credit_bp).abs() < 1e-12);
        assert!((config.equity_pct - back.equity_pct).abs() < 1e-12);
        assert!((config.fx_pct - back.fx_pct).abs() < 1e-12);
        assert!((config.vol_points - back.vol_points).abs() < 1e-12);
        assert_eq!(config.overrides, back.overrides);
    }
}
