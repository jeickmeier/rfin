//! CECL (ASC 326) variant of expected credit loss computation.
//!
//! CECL (Current Expected Credit Losses) under US GAAP ASC 326 uses a
//! different model from IFRS 9:
//!
//! - **Always lifetime**: No staging; all exposures use remaining maturity
//! - **Reasonable-and-supportable (R&S) forecast**: Forward-looking PD applies
//!   only within the R&S horizon, then reverts to historical loss rates
//! - **Reversion methods**: Immediate or linear blending from forecast to
//!   historical
//!
//! # References
//!
//! - ASC 326-20 -- Financial Instruments: Credit Losses
//! - FASB Staff Q&A 2019 -- Reasonable and Supportable Forecast Periods

use finstack_core::{Error, Result};
use serde::{Deserialize, Serialize};

use super::engine::MacroScenario;
use super::types::{Exposure, PdTermStructure};

// ---------------------------------------------------------------------------
// CECL configuration
// ---------------------------------------------------------------------------

/// How the PD curve reverts from forecast to historical after the R&S period.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ReversionMethod {
    /// Immediate: PD jumps to historical at the R&S boundary.
    Immediate,
    /// Linear: PD linearly interpolates from forecast to historical over
    /// a specified reversion period.
    Linear {
        /// Reversion period in years (e.g., 1.0 = 1-year linear fade).
        reversion_years: f64,
    },
}

/// CECL calculation methodology.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CeclMethodology {
    /// PD-LGD-EAD approach (same formula as IFRS 9, always lifetime).
    PdLgdEad,
    /// Weighted Average Remaining Maturity method.
    Warm,
    /// Vintage/cohort analysis.
    Vintage,
}

/// Configuration for CECL (US GAAP ASC 326) calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CeclConfig {
    /// Time bucket width in years (same as IFRS 9). Default: 0.25.
    pub bucket_width_years: f64,

    /// Reasonable and supportable (R&S) forecast period in years.
    /// Beyond this horizon, PD reverts to historical average.
    /// Typical range: 1-3 years.
    pub forecast_horizon_years: f64,

    /// Reversion method: how PD transitions from R&S to historical.
    pub reversion_method: ReversionMethod,

    /// Historical long-run annual PD used after the R&S period.
    pub historical_annual_pd: f64,

    /// Macro scenario specifications (same structure as IFRS 9).
    pub scenarios: Vec<MacroScenario>,

    /// CECL methodology selection.
    pub methodology: CeclMethodology,
}

impl Default for CeclConfig {
    fn default() -> Self {
        Self {
            bucket_width_years: 0.25,
            forecast_horizon_years: 2.0,
            reversion_method: ReversionMethod::Immediate,
            historical_annual_pd: 0.02,
            scenarios: vec![MacroScenario {
                id: "base".into(),
                weight: 1.0,
                lgd_override: None,
            }],
            methodology: CeclMethodology::PdLgdEad,
        }
    }
}

impl CeclConfig {
    /// Validate the configuration.
    pub fn validate(&self) -> Result<()> {
        if self.bucket_width_years <= 0.0 {
            return Err(Error::Validation(
                "bucket_width_years must be positive".to_string(),
            ));
        }
        if self.forecast_horizon_years < 0.0 {
            return Err(Error::Validation(
                "forecast_horizon_years must be non-negative".to_string(),
            ));
        }
        if self.historical_annual_pd < 0.0 || self.historical_annual_pd > 1.0 {
            return Err(Error::Validation(
                "historical_annual_pd must be in [0, 1]".to_string(),
            ));
        }
        let total_weight: f64 = self.scenarios.iter().map(|s| s.weight).sum();
        if (total_weight - 1.0).abs() > 1e-6 {
            return Err(Error::Validation(format!(
                "Scenario weights must sum to 1.0, got {:.6}",
                total_weight
            )));
        }
        if let ReversionMethod::Linear { reversion_years } = self.reversion_method {
            if reversion_years <= 0.0 {
                return Err(Error::Validation(
                    "Linear reversion_years must be positive".to_string(),
                ));
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// CECL result
// ---------------------------------------------------------------------------

/// CECL result for a single exposure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CeclResult {
    /// Exposure identifier.
    pub exposure_id: String,
    /// Total lifetime ECL.
    pub ecl: f64,
    /// ECL horizon in years (always remaining maturity).
    pub horizon: f64,
    /// Methodology used.
    pub methodology: CeclMethodology,
}

// ---------------------------------------------------------------------------
// CECL engine
// ---------------------------------------------------------------------------

/// CECL engine computing lifetime ECL with R&S forecast and historical reversion.
pub struct CeclEngine<'a> {
    config: CeclConfig,
    pd_sources: Vec<(&'a MacroScenario, &'a dyn PdTermStructure)>,
}

impl<'a> CeclEngine<'a> {
    /// Create a new CECL engine.
    pub fn new(
        config: CeclConfig,
        pd_sources: Vec<(&'a MacroScenario, &'a dyn PdTermStructure)>,
    ) -> Result<Self> {
        config.validate()?;
        Ok(Self { config, pd_sources })
    }

    /// Compute CECL for a single exposure.
    ///
    /// Always uses the full remaining maturity (no staging).
    /// PD term structure is blended: forecast PD for the R&S period,
    /// then reverts to historical.
    pub fn compute_cecl(&self, exposure: &Exposure) -> Result<CeclResult> {
        let horizon = exposure.remaining_maturity_years;
        let rating = exposure.current_rating.as_deref().unwrap_or("NR");
        let dt = self.config.bucket_width_years;
        let n_buckets = (horizon / dt).ceil() as usize;
        let n_buckets = n_buckets.max(1);

        let mut weighted_ecl = 0.0;

        for (scenario, pd_source) in &self.pd_sources {
            let mut scenario_ecl = 0.0;
            for i in 0..n_buckets {
                let t_start = i as f64 * dt;
                let t_end = ((i + 1) as f64 * dt).min(horizon);
                let t_mid = (t_start + t_end) / 2.0;

                let mpd = self.blended_marginal_pd(*pd_source, rating, t_start, t_end)?;

                let lgd = scenario.lgd_override.unwrap_or(exposure.lgd);
                let df = 1.0 / (1.0 + exposure.eir).powf(t_mid);
                scenario_ecl += mpd * lgd * exposure.ead * df;
            }
            weighted_ecl += scenario.weight * scenario_ecl;
        }

        Ok(CeclResult {
            exposure_id: exposure.id.clone(),
            ecl: weighted_ecl,
            horizon,
            methodology: self.config.methodology,
        })
    }

    /// Blend forecast PD with historical reversion.
    ///
    /// - Fully within R&S period: use forecast PD from `pd_source`.
    /// - Fully beyond R&S: use historical PD (via reversion method).
    /// - Straddling boundary: weighted blend.
    fn blended_marginal_pd(
        &self,
        pd_source: &dyn PdTermStructure,
        rating: &str,
        t1: f64,
        t2: f64,
    ) -> Result<f64> {
        let rs = self.config.forecast_horizon_years;

        if t2 <= rs {
            // Fully within R&S period: use forecast PD.
            return pd_source.marginal_pd(rating, t1, t2);
        }

        if t1 >= rs {
            // Fully beyond R&S: use reverted PD.
            return self.reverted_marginal_pd(pd_source, rating, t1, t2);
        }

        // Straddles the boundary: split the bucket.
        let w_rs = (rs - t1) / (t2 - t1);
        let pd_forecast = pd_source.marginal_pd(rating, t1, rs)?;
        let pd_reverted = self.reverted_marginal_pd(pd_source, rating, rs, t2)?;
        Ok(w_rs * pd_forecast + (1.0 - w_rs) * pd_reverted)
    }

    /// Compute the reverted marginal PD beyond the R&S period.
    ///
    /// For immediate reversion, uses the historical annual PD converted to
    /// the bucket interval. For linear reversion, blends forecast and
    /// historical PDs over the reversion window.
    fn reverted_marginal_pd(
        &self,
        pd_source: &dyn PdTermStructure,
        rating: &str,
        t1: f64,
        t2: f64,
    ) -> Result<f64> {
        let annual_pd = self.config.historical_annual_pd;
        let dt = t2 - t1;
        // Convert annual PD to marginal PD for the bucket via hazard rate
        let lambda = if annual_pd < 1.0 {
            -(1.0 - annual_pd).ln()
        } else {
            // PD = 1.0 means certain default
            f64::INFINITY
        };
        let historical_mpd = 1.0 - (-lambda * dt).exp();

        match self.config.reversion_method {
            ReversionMethod::Immediate => Ok(historical_mpd),
            ReversionMethod::Linear { reversion_years } => {
                let rs = self.config.forecast_horizon_years;
                let reversion_end = rs + reversion_years;
                let t_mid = (t1 + t2) / 2.0;

                if t_mid >= reversion_end {
                    // Past reversion window: fully historical
                    Ok(historical_mpd)
                } else {
                    // Within reversion window: linear blend
                    let blend = (t_mid - rs) / reversion_years;
                    let forecast_mpd = pd_source.marginal_pd(rating, t1, t2)?;
                    Ok((1.0 - blend) * forecast_mpd + blend * historical_mpd)
                }
            }
        }
    }

    /// Access the engine's configuration.
    pub fn config(&self) -> &CeclConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::ecl::types::{QualitativeFlags, RawPdCurve};

    fn make_exposure() -> Exposure {
        Exposure {
            id: "CECL-001".to_string(),
            segments: vec!["corporate".to_string()],
            ead: 1_000_000.0,
            eir: 0.05,
            remaining_maturity_years: 5.0,
            lgd: 0.45,
            days_past_due: 0,
            current_rating: Some("BBB".to_string()),
            origination_rating: Some("BBB".to_string()),
            qualitative_flags: QualitativeFlags::default(),
            consecutive_performing_periods: 0,
            previous_stage: None,
        }
    }

    fn make_pd_curve() -> RawPdCurve {
        RawPdCurve {
            rating: "BBB".to_string(),
            knots: vec![(0.0, 0.0), (1.0, 0.02), (2.0, 0.04), (5.0, 0.10)],
        }
    }

    #[test]
    fn test_cecl_always_lifetime() {
        let curve = make_pd_curve();
        let scenario = MacroScenario {
            id: "base".into(),
            weight: 1.0,
            lgd_override: None,
        };
        let pd_sources: Vec<(&MacroScenario, &dyn PdTermStructure)> =
            vec![(&scenario, &curve as &dyn PdTermStructure)];

        let config = CeclConfig::default();
        let engine = CeclEngine::new(config, pd_sources).unwrap();
        let exposure = make_exposure();
        let result = engine.compute_cecl(&exposure).unwrap();

        // CECL always uses remaining maturity
        assert!(
            (result.horizon - 5.0).abs() < 1e-10,
            "CECL horizon should equal remaining maturity"
        );
        assert!(result.ecl > 0.0);
    }

    #[test]
    fn test_cecl_immediate_reversion() {
        let curve = make_pd_curve();
        let scenario = MacroScenario {
            id: "base".into(),
            weight: 1.0,
            lgd_override: None,
        };
        let pd_sources: Vec<(&MacroScenario, &dyn PdTermStructure)> =
            vec![(&scenario, &curve as &dyn PdTermStructure)];

        let config = CeclConfig {
            forecast_horizon_years: 1.0,
            reversion_method: ReversionMethod::Immediate,
            historical_annual_pd: 0.03,
            ..CeclConfig::default()
        };
        let engine = CeclEngine::new(config, pd_sources).unwrap();
        let exposure = make_exposure();
        let result = engine.compute_cecl(&exposure).unwrap();

        assert!(result.ecl > 0.0);
    }

    #[test]
    fn test_cecl_linear_reversion() {
        let curve = make_pd_curve();
        let scenario = MacroScenario {
            id: "base".into(),
            weight: 1.0,
            lgd_override: None,
        };
        let pd_sources: Vec<(&MacroScenario, &dyn PdTermStructure)> =
            vec![(&scenario, &curve as &dyn PdTermStructure)];

        let config = CeclConfig {
            forecast_horizon_years: 1.0,
            reversion_method: ReversionMethod::Linear {
                reversion_years: 1.0,
            },
            historical_annual_pd: 0.03,
            ..CeclConfig::default()
        };
        let engine = CeclEngine::new(config, pd_sources).unwrap();
        let exposure = make_exposure();
        let result = engine.compute_cecl(&exposure).unwrap();

        assert!(result.ecl > 0.0);
    }

    #[test]
    fn test_cecl_validation_negative_historical_pd() {
        let config = CeclConfig {
            historical_annual_pd: -0.01,
            ..CeclConfig::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_cecl_validation_invalid_weights() {
        let config = CeclConfig {
            scenarios: vec![
                MacroScenario {
                    id: "a".into(),
                    weight: 0.5,
                    lgd_override: None,
                },
                MacroScenario {
                    id: "b".into(),
                    weight: 0.3,
                    lgd_override: None,
                },
            ],
            ..CeclConfig::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_cecl_vs_ifrs9_stage2() {
        // CECL should produce similar results to IFRS 9 Stage 2 (both lifetime)
        // when using same PD curve and no reversion (R&S covers full horizon).
        let curve = make_pd_curve();
        let exposure = make_exposure();

        // CECL with R&S covering full maturity
        let cecl_scenario = MacroScenario {
            id: "base".into(),
            weight: 1.0,
            lgd_override: None,
        };
        let cecl_pd: Vec<(&MacroScenario, &dyn PdTermStructure)> =
            vec![(&cecl_scenario, &curve as &dyn PdTermStructure)];
        let cecl_config = CeclConfig {
            forecast_horizon_years: 10.0, // Covers full 5-year maturity
            reversion_method: ReversionMethod::Immediate,
            historical_annual_pd: 0.02,
            ..CeclConfig::default()
        };
        let cecl_engine = CeclEngine::new(cecl_config, cecl_pd).unwrap();
        let cecl_result = cecl_engine.compute_cecl(&exposure).unwrap();

        // IFRS 9 Stage 2
        let ifrs9_config = super::super::engine::EclConfig::default();
        let ifrs9_result = super::super::engine::compute_ecl_single(
            &exposure,
            crate::analysis::ecl::types::Stage::Stage2,
            &curve,
            &ifrs9_config,
        )
        .unwrap();

        // Both should be close (same formula, same horizon)
        assert!(
            (cecl_result.ecl - ifrs9_result.ecl).abs() / ifrs9_result.ecl < 0.01,
            "CECL ({}) and IFRS 9 Stage 2 ({}) should be close when R&S covers maturity",
            cecl_result.ecl,
            ifrs9_result.ecl
        );
    }
}
