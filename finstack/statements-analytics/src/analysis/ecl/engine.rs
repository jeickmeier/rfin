//! ECL calculation engine.
//!
//! Provides the core ECL computation functions:
//!
//! - [`compute_ecl_single`] -- single exposure, single scenario
//! - [`compute_ecl_weighted`] -- single exposure, probability-weighted across scenarios
//! - [`EclEngine`] -- stateful facade wrapping staging + calculation
//!
//! # ECL Formula
//!
//! For each time bucket \[t_{i-1}, t_i\]:
//!
//! ```text
//! bucket_ECL = marginal_PD(t_{i-1}, t_i) * LGD * EAD * DF(t_mid)
//! ```
//!
//! where DF(t) = 1 / (1 + EIR)^t is the IFRS 9 effective interest rate
//! discount factor.
//!
//! Total ECL = sum of bucket ECLs over the appropriate horizon:
//! - Stage 1: min(1 year, remaining maturity)
//! - Stage 2/3: remaining maturity
//!
//! # References
//!
//! - IFRS 9 B5.5.28-33 -- Measurement of expected credit losses
//! - IFRS 9 B5.5.44 -- Discount rate (effective interest rate)
//! - IFRS 9 B5.5.42 -- Probability-weighted scenarios

use finstack_core::{Error, Result};
use serde::{Deserialize, Serialize};

use super::staging::{classify_stage, StagingConfig, StageResult};
use super::types::{Exposure, PdTermStructure, Stage};

// ---------------------------------------------------------------------------
// Macro scenario
// ---------------------------------------------------------------------------

/// A forward-looking macro scenario with a probability weight.
///
/// Used for probability-weighted ECL calculation per IFRS 9 B5.5.42.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroScenario {
    /// Scenario identifier (e.g., "base", "upside", "downside").
    pub id: String,

    /// Probability weight in \[0, 1\]. All scenario weights must sum to 1.0.
    pub weight: f64,

    /// Optional LGD override for this scenario (downturn LGD).
    pub lgd_override: Option<f64>,
}

// ---------------------------------------------------------------------------
// LGD type
// ---------------------------------------------------------------------------

/// LGD methodology selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LgdType {
    /// Point-in-time LGD from the exposure.
    PointInTime,
    /// Through-the-cycle average LGD.
    ThroughTheCycle,
    /// Downturn LGD (stressed scenario).
    Downturn,
}

// ---------------------------------------------------------------------------
// ECL configuration
// ---------------------------------------------------------------------------

/// Configuration for ECL calculation.
///
/// Controls time bucket granularity, scenario specifications, staging
/// parameters, and LGD methodology.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EclConfig {
    /// Time bucket width in years for the PD-LGD-EAD integration.
    /// Default: quarterly (0.25).
    pub bucket_width_years: f64,

    /// Macro scenario specifications with probability weights.
    /// Weights must sum to 1.0.
    pub scenarios: Vec<MacroScenario>,

    /// Staging configuration for IFRS 9.
    pub staging: StagingConfig,

    /// Whether to use point-in-time LGD or through-the-cycle.
    pub lgd_type: LgdType,
}

impl Default for EclConfig {
    fn default() -> Self {
        Self {
            bucket_width_years: 0.25,
            scenarios: vec![MacroScenario {
                id: "base".into(),
                weight: 1.0,
                lgd_override: None,
            }],
            staging: StagingConfig::default(),
            lgd_type: LgdType::PointInTime,
        }
    }
}

// ---------------------------------------------------------------------------
// Builder
// ---------------------------------------------------------------------------

/// Builder for [`EclConfig`].
///
/// Validates configuration on `build()`:
/// - Scenario weights must sum to 1.0 (within 1e-6 tolerance)
/// - Bucket width must be positive
pub struct EclConfigBuilder {
    config: EclConfig,
}

impl EclConfigBuilder {
    /// Create a new builder with default configuration.
    pub fn new() -> Self {
        Self {
            config: EclConfig::default(),
        }
    }

    /// Set the time bucket width in years.
    pub fn bucket_width(mut self, years: f64) -> Self {
        self.config.bucket_width_years = years;
        self
    }

    /// Set the staging configuration.
    pub fn staging(mut self, staging: StagingConfig) -> Self {
        self.config.staging = staging;
        self
    }

    /// Replace all scenarios.
    pub fn scenarios(mut self, scenarios: Vec<MacroScenario>) -> Self {
        self.config.scenarios = scenarios;
        self
    }

    /// Add a single scenario.
    pub fn add_scenario(mut self, scenario: MacroScenario) -> Self {
        self.config.scenarios.push(scenario);
        self
    }

    /// Set the LGD methodology.
    pub fn lgd_type(mut self, lgd_type: LgdType) -> Self {
        self.config.lgd_type = lgd_type;
        self
    }

    /// Validate and build the configuration.
    pub fn build(self) -> Result<EclConfig> {
        let total_weight: f64 = self.config.scenarios.iter().map(|s| s.weight).sum();
        if (total_weight - 1.0).abs() > 1e-6 {
            return Err(Error::Validation(format!(
                "Scenario weights must sum to 1.0, got {:.6}",
                total_weight
            )));
        }
        if self.config.bucket_width_years <= 0.0 {
            return Err(Error::Validation(
                "bucket_width_years must be positive".to_string(),
            ));
        }
        if self.config.scenarios.is_empty() {
            return Err(Error::Validation(
                "At least one scenario is required".to_string(),
            ));
        }
        Ok(self.config)
    }
}

impl Default for EclConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Result types
// ---------------------------------------------------------------------------

/// ECL result for a single time bucket.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EclBucket {
    /// Start of the time bucket (years).
    pub t_start: f64,
    /// End of the time bucket (years).
    pub t_end: f64,
    /// Marginal PD for this bucket, conditional on survival.
    pub marginal_pd: f64,
    /// LGD used for this bucket.
    pub lgd: f64,
    /// EAD used for this bucket.
    pub ead: f64,
    /// Discount factor at the bucket midpoint.
    pub discount_factor: f64,
    /// ECL contribution from this bucket.
    pub ecl: f64,
}

/// ECL result for a single exposure under a single scenario.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EclResult {
    /// Exposure identifier.
    pub exposure_id: String,
    /// Assigned IFRS 9 stage.
    pub stage: Stage,
    /// Total ECL for this exposure under this scenario.
    pub ecl: f64,
    /// ECL horizon in years.
    pub horizon: f64,
    /// Per-bucket breakdown.
    pub buckets: Vec<EclBucket>,
}

/// Probability-weighted ECL result across scenarios.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeightedEclResult {
    /// Exposure identifier.
    pub exposure_id: String,
    /// Assigned IFRS 9 stage.
    pub stage: Stage,
    /// Probability-weighted ECL.
    pub ecl: f64,
    /// Per-scenario breakdown: (scenario_id, weight, result).
    pub scenario_breakdown: Vec<(String, f64, EclResult)>,
}

/// Combined staging + ECL result for one exposure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExposureEclResult {
    /// Stage classification result with audit trail.
    pub stage_result: StageResult,
    /// Probability-weighted ECL result.
    pub ecl_result: WeightedEclResult,
}

// ---------------------------------------------------------------------------
// Core computation (stateless)
// ---------------------------------------------------------------------------

/// Compute ECL for a single exposure under a single scenario.
///
/// Integrates marginal PD x LGD x EAD x DF over time buckets up to the
/// appropriate horizon (12 months for Stage 1, remaining maturity for
/// Stage 2/3).
///
/// # Arguments
///
/// * `exposure` -- The credit exposure
/// * `stage` -- Assigned IFRS 9 stage (determines ECL horizon)
/// * `pd_source` -- PD term structure for the exposure's rating
/// * `config` -- ECL calculation parameters
pub fn compute_ecl_single(
    exposure: &Exposure,
    stage: Stage,
    pd_source: &dyn PdTermStructure,
    config: &EclConfig,
) -> Result<EclResult> {
    let horizon = match stage {
        Stage::Stage1 => 1.0_f64.min(exposure.remaining_maturity_years),
        Stage::Stage2 | Stage::Stage3 => exposure.remaining_maturity_years,
    };

    let rating = exposure.current_rating.as_deref().unwrap_or("NR");
    let dt = config.bucket_width_years;
    let n_buckets = (horizon / dt).ceil() as usize;
    let n_buckets = n_buckets.max(1); // At least one bucket

    let mut ecl = 0.0;
    let mut bucket_details = Vec::with_capacity(n_buckets);

    for i in 0..n_buckets {
        let t_start = i as f64 * dt;
        let t_end = ((i + 1) as f64 * dt).min(horizon);
        let t_mid = (t_start + t_end) / 2.0;

        let mpd = pd_source.marginal_pd(rating, t_start, t_end)?;
        let lgd = exposure.lgd;
        let ead = exposure.ead;
        let df = 1.0 / (1.0 + exposure.eir).powf(t_mid);

        let bucket_ecl = mpd * lgd * ead * df;
        ecl += bucket_ecl;

        bucket_details.push(EclBucket {
            t_start,
            t_end,
            marginal_pd: mpd,
            lgd,
            ead,
            discount_factor: df,
            ecl: bucket_ecl,
        });
    }

    Ok(EclResult {
        exposure_id: exposure.id.clone(),
        stage,
        ecl,
        horizon,
        buckets: bucket_details,
    })
}

/// Compute probability-weighted ECL across macro scenarios.
///
/// IFRS 9 B5.5.42 requires that ECL reflects an unbiased and
/// probability-weighted amount determined by evaluating a range of
/// possible outcomes.
///
/// # Arguments
///
/// * `exposure` -- The credit exposure
/// * `stage` -- Assigned IFRS 9 stage
/// * `pd_sources` -- Slice of (scenario, PD source) pairs
/// * `config` -- ECL calculation parameters
pub fn compute_ecl_weighted(
    exposure: &Exposure,
    stage: Stage,
    pd_sources: &[(&MacroScenario, &dyn PdTermStructure)],
    config: &EclConfig,
) -> Result<WeightedEclResult> {
    let mut weighted_ecl = 0.0;
    let mut scenario_results = Vec::with_capacity(pd_sources.len());

    for (scenario, pd_source) in pd_sources {
        let lgd_adj = scenario.lgd_override.unwrap_or(exposure.lgd);
        let adj_exposure = Exposure {
            lgd: lgd_adj,
            ..exposure.clone()
        };
        let result = compute_ecl_single(&adj_exposure, stage, *pd_source, config)?;
        weighted_ecl += scenario.weight * result.ecl;
        scenario_results.push((scenario.id.clone(), scenario.weight, result));
    }

    Ok(WeightedEclResult {
        exposure_id: exposure.id.clone(),
        stage,
        ecl: weighted_ecl,
        scenario_breakdown: scenario_results,
    })
}

// ---------------------------------------------------------------------------
// Stateful facade
// ---------------------------------------------------------------------------

/// Stateful ECL engine wrapping staging + calculation + aggregation.
///
/// Holds configuration and PD sources, provides a single entry point for
/// portfolio-level ECL computation.
pub struct EclEngine<'a> {
    config: EclConfig,
    pd_sources: Vec<(&'a MacroScenario, &'a dyn PdTermStructure)>,
}

impl<'a> EclEngine<'a> {
    /// Create a new engine with the given configuration and PD sources.
    ///
    /// The first element in `pd_sources` is used as the base scenario for
    /// stage classification.
    pub fn new(
        config: EclConfig,
        pd_sources: Vec<(&'a MacroScenario, &'a dyn PdTermStructure)>,
    ) -> Self {
        Self { config, pd_sources }
    }

    /// Classify and compute ECL for a single exposure.
    pub fn process_exposure(&self, exposure: &Exposure) -> Result<ExposureEclResult> {
        // Use base scenario PD for staging
        let base_pd = self.pd_sources[0].1;
        let stage_result = classify_stage(exposure, base_pd, &self.config.staging)?;
        let ecl_result = compute_ecl_weighted(
            exposure,
            stage_result.stage,
            &self.pd_sources,
            &self.config,
        )?;
        Ok(ExposureEclResult {
            stage_result,
            ecl_result,
        })
    }

    /// Access the engine's configuration.
    pub fn config(&self) -> &EclConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::ecl::types::{QualitativeFlags, RawPdCurve};

    fn make_exposure() -> Exposure {
        Exposure {
            id: "EXP-001".to_string(),
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
    fn test_ecl_config_builder_valid() {
        let config = EclConfigBuilder::new()
            .bucket_width(0.5)
            .scenarios(vec![
                MacroScenario { id: "base".into(), weight: 0.6, lgd_override: None },
                MacroScenario { id: "down".into(), weight: 0.4, lgd_override: Some(0.55) },
            ])
            .build();
        assert!(config.is_ok());
        let config = config.unwrap();
        assert!((config.bucket_width_years - 0.5).abs() < 1e-10);
        assert_eq!(config.scenarios.len(), 2);
    }

    #[test]
    fn test_ecl_config_builder_invalid_weights() {
        let config = EclConfigBuilder::new()
            .scenarios(vec![
                MacroScenario { id: "base".into(), weight: 0.5, lgd_override: None },
                MacroScenario { id: "down".into(), weight: 0.3, lgd_override: None },
            ])
            .build();
        assert!(config.is_err());
    }

    #[test]
    fn test_ecl_config_builder_invalid_bucket_width() {
        let config = EclConfigBuilder::new().bucket_width(0.0).build();
        assert!(config.is_err());
    }

    #[test]
    fn test_compute_ecl_single_stage1() {
        let exposure = make_exposure();
        let curve = make_pd_curve();
        let config = EclConfig::default();

        let result = compute_ecl_single(&exposure, Stage::Stage1, &curve, &config).unwrap();

        // Stage 1 horizon = min(1.0, 5.0) = 1.0
        assert!((result.horizon - 1.0).abs() < 1e-10);
        assert!(result.ecl > 0.0);
        assert_eq!(result.buckets.len(), 4); // 1.0 / 0.25 = 4 quarterly buckets
    }

    #[test]
    fn test_compute_ecl_single_stage2() {
        let exposure = make_exposure();
        let curve = make_pd_curve();
        let config = EclConfig::default();

        let result = compute_ecl_single(&exposure, Stage::Stage2, &curve, &config).unwrap();

        // Stage 2 horizon = remaining maturity = 5.0
        assert!((result.horizon - 5.0).abs() < 1e-10);
        assert!(result.ecl > 0.0);
        assert_eq!(result.buckets.len(), 20); // 5.0 / 0.25 = 20 buckets
    }

    #[test]
    fn test_stage1_ecl_less_than_stage2() {
        let exposure = make_exposure();
        let curve = make_pd_curve();
        let config = EclConfig::default();

        let s1 = compute_ecl_single(&exposure, Stage::Stage1, &curve, &config).unwrap();
        let s2 = compute_ecl_single(&exposure, Stage::Stage2, &curve, &config).unwrap();

        // Stage 1 (12-month) ECL must be less than Stage 2 (lifetime)
        assert!(
            s1.ecl < s2.ecl,
            "Stage 1 ECL ({}) should be < Stage 2 ECL ({})",
            s1.ecl,
            s2.ecl
        );
    }

    #[test]
    fn test_ecl_hand_computed() {
        // Simple 2-bucket test with known marginal PDs.
        // Curve: cumulative PD at t=0 is 0, at t=0.5 is 0.01, at t=1.0 is 0.02
        let curve = RawPdCurve {
            rating: "TEST".to_string(),
            knots: vec![(0.0, 0.0), (0.5, 0.01), (1.0, 0.02)],
        };
        let exposure = Exposure {
            id: "HAND".to_string(),
            segments: vec![],
            ead: 100_000.0,
            eir: 0.0, // No discounting for simplicity
            remaining_maturity_years: 1.0,
            lgd: 0.40,
            days_past_due: 0,
            current_rating: Some("TEST".to_string()),
            origination_rating: Some("TEST".to_string()),
            qualitative_flags: QualitativeFlags::default(),
            consecutive_performing_periods: 0,
            previous_stage: None,
        };

        let config = EclConfigBuilder::new().bucket_width(0.5).build().unwrap();

        let result = compute_ecl_single(&exposure, Stage::Stage1, &curve, &config).unwrap();

        // Bucket 1: [0, 0.5]
        // marginal_pd = 1 - (1-0.01)/(1-0.0) = 0.01
        // bucket_ecl = 0.01 * 0.40 * 100000 * 1.0 = 400.0
        //
        // Bucket 2: [0.5, 1.0]
        // marginal_pd = 1 - (1-0.02)/(1-0.01) = 1 - 0.98/0.99 = 0.01010101...
        // bucket_ecl = 0.010101... * 0.40 * 100000 * 1.0 = 404.04...
        //
        // Total ECL ~ 804.04
        assert_eq!(result.buckets.len(), 2);
        assert!((result.buckets[0].ecl - 400.0).abs() < 1.0);
        assert!((result.ecl - 804.0).abs() < 2.0);
    }

    #[test]
    fn test_scenario_weighting() {
        let exposure = make_exposure();
        let curve = make_pd_curve();
        let config = EclConfig::default();

        // Single scenario with weight 1.0 should equal unweighted
        let single = compute_ecl_single(&exposure, Stage::Stage1, &curve, &config).unwrap();

        let scenario = MacroScenario {
            id: "base".into(),
            weight: 1.0,
            lgd_override: None,
        };
        let pd_sources: Vec<(&MacroScenario, &dyn PdTermStructure)> =
            vec![(&scenario, &curve as &dyn PdTermStructure)];
        let weighted = compute_ecl_weighted(&exposure, Stage::Stage1, &pd_sources, &config).unwrap();

        assert!(
            (single.ecl - weighted.ecl).abs() < 1e-10,
            "Single-scenario weighted ECL should equal unweighted: {} vs {}",
            single.ecl,
            weighted.ecl
        );
    }

    #[test]
    fn test_scenario_weighting_two_scenarios() {
        let exposure = make_exposure();
        let curve = make_pd_curve();
        let config = EclConfig::default();

        let base_scenario = MacroScenario {
            id: "base".into(),
            weight: 0.6,
            lgd_override: None,
        };
        let down_scenario = MacroScenario {
            id: "downside".into(),
            weight: 0.4,
            lgd_override: Some(0.60), // Higher LGD in downside
        };

        let pd_sources: Vec<(&MacroScenario, &dyn PdTermStructure)> = vec![
            (&base_scenario, &curve as &dyn PdTermStructure),
            (&down_scenario, &curve as &dyn PdTermStructure),
        ];

        let result =
            compute_ecl_weighted(&exposure, Stage::Stage1, &pd_sources, &config).unwrap();

        // Verify manual calculation
        let base_ecl = result.scenario_breakdown[0].2.ecl;
        let down_ecl = result.scenario_breakdown[1].2.ecl;
        let expected = 0.6 * base_ecl + 0.4 * down_ecl;

        assert!(
            (result.ecl - expected).abs() < 1e-6,
            "Weighted ECL mismatch: {} vs expected {}",
            result.ecl,
            expected
        );

        // Downside ECL should be higher due to higher LGD
        assert!(down_ecl > base_ecl);
    }

    #[test]
    fn test_ecl_engine_process_exposure() {
        let curve = make_pd_curve();
        let config = EclConfig::default();
        let scenario = &config.scenarios[0];

        let pd_sources: Vec<(&MacroScenario, &dyn PdTermStructure)> =
            vec![(scenario, &curve as &dyn PdTermStructure)];
        let engine = EclEngine::new(config.clone(), pd_sources);

        let exposure = make_exposure();
        let result = engine.process_exposure(&exposure).unwrap();

        assert_eq!(result.stage_result.stage, Stage::Stage1);
        assert!(result.ecl_result.ecl > 0.0);
    }
}
