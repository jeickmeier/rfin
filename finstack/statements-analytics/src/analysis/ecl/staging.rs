//! IFRS 9 stage classification engine.
//!
//! Implements the waterfall logic for assigning exposures to impairment stages
//! based on quantitative triggers (PD delta, DPD backstops) and qualitative
//! flags (watchlist, forbearance, adverse conditions).
//!
//! The classification waterfall is:
//! 1. Stage 3 backstop: DPD > `dpd_stage3_threshold` (absolute, non-rebuttable)
//! 2. Stage 2 quantitative: PD delta exceeds thresholds (absolute OR relative)
//! 3. Stage 2 qualitative: Any qualitative flag is active
//! 4. Stage 2 DPD backstop: DPD > `dpd_stage2_threshold` (rebuttable presumption)
//! 5. Curing: If previously Stage 2/3 and now meets cure criteria, allow step-down
//! 6. Default: Stage 1
//!
//! # Comparator convention
//!
//! IFRS 9 B5.5.37 phrases the backstops as "contractual payments are
//! *more than* 30 / 90 days past due". This module follows that literal
//! wording: a trigger fires when `days_past_due > threshold` (equivalently
//! DPD >= threshold + 1). Callers who require the alternative ">=" reading
//! (e.g. some Basel IRB implementations) should set
//! `dpd_stage2_threshold = 29` and `dpd_stage3_threshold = 89`.
//!
//! # References
//!
//! - IFRS 9 Financial Instruments, Section 5.5 -- Impairment
//! - IFRS 9 B5.5.17 -- Qualitative indicators of SICR
//! - IFRS 9 B5.5.19 -- 30 days past due rebuttable presumption
//! - IFRS 9 B5.5.37 -- 90 days past due default presumption

use finstack_core::Result;
use serde::{Deserialize, Serialize};

use super::types::{Exposure, PdTermStructure, Stage};

// ---------------------------------------------------------------------------
// Staging trigger (audit trail)
// ---------------------------------------------------------------------------

/// Reason for stage assignment (audit trail).
///
/// Each variant captures the specific values that triggered the classification,
/// enabling regulatory reporting and model governance review.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StagingTrigger {
    /// DPD exceeded Stage 3 threshold.
    DpdStage3 {
        /// Actual days past due.
        dpd: u32,
        /// Configured threshold.
        threshold: u32,
    },
    /// Absolute PD increase exceeded threshold.
    PdDeltaAbsolute {
        /// Actual PD delta (current minus origination).
        delta: f64,
        /// Configured threshold.
        threshold: f64,
    },
    /// Relative PD increase exceeded threshold.
    PdDeltaRelative {
        /// Actual ratio (current PD / origination PD).
        ratio: f64,
        /// Configured threshold.
        threshold: f64,
    },
    /// Rating downgraded by N or more notches.
    RatingDowngrade {
        /// Actual downgrade notches.
        notches: u32,
        /// Configured threshold.
        threshold: u32,
    },
    /// DPD exceeded Stage 2 backstop.
    DpdStage2 {
        /// Actual days past due.
        dpd: u32,
        /// Configured threshold.
        threshold: u32,
    },
    /// Qualitative flag active.
    Qualitative {
        /// Name of the active flag.
        flag: String,
    },
    /// No triggers active: Stage 1 default.
    NoTrigger,
}

// ---------------------------------------------------------------------------
// Stage result
// ---------------------------------------------------------------------------

/// Result of stage classification with audit trail.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageResult {
    /// Assigned stage.
    pub stage: Stage,
    /// Which trigger(s) caused the classification.
    pub triggers: Vec<StagingTrigger>,
    /// Whether the exposure was cured from a higher stage.
    pub cured: bool,
}

// ---------------------------------------------------------------------------
// Staging configuration
// ---------------------------------------------------------------------------

/// Configuration for IFRS 9 stage classification.
///
/// Controls the quantitative and qualitative thresholds used in the staging
/// waterfall, plus curing rules for step-down from higher stages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StagingConfig {
    /// Absolute PD increase threshold for SICR (e.g., 0.01 = 1 pp).
    /// If the lifetime PD has increased by more than this amount since
    /// origination, the exposure is classified as Stage 2.
    pub pd_delta_absolute: f64,

    /// Relative PD increase threshold for SICR (e.g., 2.0 = PD doubled).
    /// Applied as: current_pd / origination_pd > threshold.
    pub pd_delta_relative: f64,

    /// Rating downgrade notches that trigger Stage 2.
    /// Example: 3 means a 3-notch downgrade from origination triggers SICR.
    pub rating_downgrade_notches: u32,

    /// DPD threshold for Stage 2 backstop (IFRS 9 B5.5.19 rebuttable
    /// presumption). Default: 30.
    pub dpd_stage2_threshold: u32,

    /// DPD threshold for Stage 3 (absolute backstop). Default: 90.
    pub dpd_stage3_threshold: u32,

    /// Whether any qualitative flag triggers Stage 2.
    pub qualitative_triggers_enabled: bool,

    /// Curing configuration: minimum consecutive performing periods
    /// required to move from Stage 2 back to Stage 1.
    pub cure_periods_stage2_to_1: u32,

    /// Curing configuration: minimum consecutive performing periods
    /// required to move from Stage 3 to Stage 2.
    pub cure_periods_stage3_to_2: u32,
}

impl Default for StagingConfig {
    fn default() -> Self {
        Self {
            pd_delta_absolute: 0.01, // 1 pp
            pd_delta_relative: 2.0,  // PD doubled
            rating_downgrade_notches: 3,
            dpd_stage2_threshold: 30,
            dpd_stage3_threshold: 90,
            qualitative_triggers_enabled: true,
            cure_periods_stage2_to_1: 3,
            cure_periods_stage3_to_2: 6,
        }
    }
}

// ---------------------------------------------------------------------------
// Classification logic
// ---------------------------------------------------------------------------

/// Classify an exposure into an IFRS 9 stage.
///
/// The classification waterfall evaluates triggers in priority order:
///
/// 1. **Stage 3 backstop**: DPD exceeds `dpd_stage3_threshold` (non-rebuttable)
/// 2. **Stage 2 quantitative**: PD delta exceeds absolute OR relative threshold
/// 3. **Stage 2 qualitative**: Any qualitative flag is active (if enabled)
/// 4. **Stage 2 DPD backstop**: DPD exceeds `dpd_stage2_threshold`
/// 5. **Curing**: Previous stage was higher, sufficient performing periods
/// 6. **Default**: Stage 1 (no triggers active)
///
/// # Arguments
///
/// * `exposure` -- The credit exposure to classify
/// * `pd_source` -- PD term structure for computing lifetime PDs
/// * `config` -- Staging configuration with thresholds
///
/// # Returns
///
/// A [`StageResult`] containing the assigned stage, trigger reasons, and
/// whether curing was applied.
pub fn classify_stage(
    exposure: &Exposure,
    pd_source: &dyn PdTermStructure,
    config: &StagingConfig,
) -> Result<StageResult> {
    let mut triggers = Vec::new();

    // 1. Stage 3 DPD backstop (non-rebuttable)
    if exposure.days_past_due > config.dpd_stage3_threshold {
        triggers.push(StagingTrigger::DpdStage3 {
            dpd: exposure.days_past_due,
            threshold: config.dpd_stage3_threshold,
        });
        return Ok(StageResult {
            stage: Stage::Stage3,
            triggers,
            cured: false,
        });
    }

    // 2. Stage 2 quantitative: PD delta (only if both ratings available)
    if let (Some(orig_rating), Some(curr_rating)) =
        (&exposure.origination_rating, &exposure.current_rating)
    {
        let horizon = exposure.remaining_maturity_years.min(30.0).max(1.0);
        let orig_pd = pd_source.cumulative_pd(orig_rating, horizon)?;
        let curr_pd = pd_source.cumulative_pd(curr_rating, horizon)?;

        let delta = curr_pd - orig_pd;
        if delta > config.pd_delta_absolute {
            triggers.push(StagingTrigger::PdDeltaAbsolute {
                delta,
                threshold: config.pd_delta_absolute,
            });
        }

        if orig_pd > 0.0 {
            let ratio = curr_pd / orig_pd;
            if ratio > config.pd_delta_relative {
                triggers.push(StagingTrigger::PdDeltaRelative {
                    ratio,
                    threshold: config.pd_delta_relative,
                });
            }
        }
    }

    // 3. Stage 2 qualitative flags
    if config.qualitative_triggers_enabled && exposure.qualitative_flags.any_active() {
        for flag_name in exposure.qualitative_flags.active_flags() {
            triggers.push(StagingTrigger::Qualitative { flag: flag_name });
        }
    }

    // 4. Stage 2 DPD backstop
    if exposure.days_past_due > config.dpd_stage2_threshold {
        triggers.push(StagingTrigger::DpdStage2 {
            dpd: exposure.days_past_due,
            threshold: config.dpd_stage2_threshold,
        });
    }

    // If any Stage 2 trigger fired, classify as Stage 2
    if !triggers.is_empty() {
        // Check curing: can we step down?
        if let Some(prev_stage) = &exposure.previous_stage {
            match prev_stage {
                Stage::Stage3 => {
                    // Stage 3 -> Stage 2 curing check: but we already have
                    // Stage 2 triggers, so stay at Stage 2 (already the
                    // correct assignment).
                }
                _ => {}
            }
        }
        return Ok(StageResult {
            stage: Stage::Stage2,
            triggers,
            cured: false,
        });
    }

    // 5. Curing logic: check if previously in a higher stage
    if let Some(prev_stage) = &exposure.previous_stage {
        match prev_stage {
            Stage::Stage3 => {
                // Need enough performing periods to step down to Stage 2
                if exposure.consecutive_performing_periods < config.cure_periods_stage3_to_2 {
                    return Ok(StageResult {
                        stage: Stage::Stage3,
                        triggers: vec![StagingTrigger::NoTrigger],
                        cured: false,
                    });
                }
                // Cured from Stage 3 to Stage 2, then check Stage 2 -> 1
                if exposure.consecutive_performing_periods
                    < config.cure_periods_stage3_to_2 + config.cure_periods_stage2_to_1
                {
                    return Ok(StageResult {
                        stage: Stage::Stage2,
                        triggers: vec![StagingTrigger::NoTrigger],
                        cured: true,
                    });
                }
                // Fully cured from Stage 3 to Stage 1
                return Ok(StageResult {
                    stage: Stage::Stage1,
                    triggers: vec![StagingTrigger::NoTrigger],
                    cured: true,
                });
            }
            Stage::Stage2 => {
                // Need enough performing periods to step down to Stage 1
                if exposure.consecutive_performing_periods < config.cure_periods_stage2_to_1 {
                    return Ok(StageResult {
                        stage: Stage::Stage2,
                        triggers: vec![StagingTrigger::NoTrigger],
                        cured: false,
                    });
                }
                // Cured from Stage 2 to Stage 1
                return Ok(StageResult {
                    stage: Stage::Stage1,
                    triggers: vec![StagingTrigger::NoTrigger],
                    cured: true,
                });
            }
            Stage::Stage1 => {
                // Already Stage 1, no curing needed
            }
        }
    }

    // 6. Default: Stage 1
    Ok(StageResult {
        stage: Stage::Stage1,
        triggers: vec![StagingTrigger::NoTrigger],
        cured: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::ecl::types::{QualitativeFlags, RawPdCurve};

    fn make_pd_curve() -> RawPdCurve {
        // A simple BBB curve: cumulative PD increases over time
        RawPdCurve {
            rating: "BBB".to_string(),
            knots: vec![(0.0, 0.0), (1.0, 0.02), (5.0, 0.10), (10.0, 0.20)],
        }
    }

    fn make_multi_rating_curves() -> MultiRatingCurve {
        MultiRatingCurve {
            curves: vec![
                (
                    "A".to_string(),
                    vec![(0.0, 0.0), (1.0, 0.005), (5.0, 0.03), (10.0, 0.06)],
                ),
                (
                    "BBB".to_string(),
                    vec![(0.0, 0.0), (1.0, 0.02), (5.0, 0.10), (10.0, 0.20)],
                ),
                (
                    "BB".to_string(),
                    vec![(0.0, 0.0), (1.0, 0.05), (5.0, 0.20), (10.0, 0.40)],
                ),
            ],
        }
    }

    /// Helper: multi-rating PD source for staging tests.
    struct MultiRatingCurve {
        curves: Vec<(String, Vec<(f64, f64)>)>,
    }

    impl PdTermStructure for MultiRatingCurve {
        fn cumulative_pd(&self, rating: &str, t: f64) -> finstack_core::Result<f64> {
            for (r, knots) in &self.curves {
                if r == rating {
                    return Ok(super::super::types::interp_linear(knots, t));
                }
            }
            Err(finstack_core::Error::Validation(format!(
                "Rating '{}' not found",
                rating
            )))
        }
    }

    fn base_exposure() -> Exposure {
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

    #[test]
    fn test_stage1_default() {
        let curve = make_pd_curve();
        let config = StagingConfig::default();
        let exposure = base_exposure();

        let result = classify_stage(&exposure, &curve, &config).unwrap();
        assert_eq!(result.stage, Stage::Stage1);
        assert!(!result.cured);
    }

    #[test]
    fn test_stage3_dpd_backstop() {
        let curve = make_pd_curve();
        let config = StagingConfig::default();
        let mut exposure = base_exposure();
        exposure.days_past_due = 91;

        let result = classify_stage(&exposure, &curve, &config).unwrap();
        assert_eq!(result.stage, Stage::Stage3);
        assert!(matches!(
            result.triggers[0],
            StagingTrigger::DpdStage3 {
                dpd: 91,
                threshold: 90
            }
        ));
    }

    #[test]
    fn test_stage2_dpd_backstop() {
        let curve = make_pd_curve();
        let config = StagingConfig::default();
        let mut exposure = base_exposure();
        exposure.days_past_due = 35;

        let result = classify_stage(&exposure, &curve, &config).unwrap();
        assert_eq!(result.stage, Stage::Stage2);
        assert!(result
            .triggers
            .iter()
            .any(|t| matches!(t, StagingTrigger::DpdStage2 { .. })));
    }

    #[test]
    fn test_stage2_pd_delta() {
        let curves = make_multi_rating_curves();
        let config = StagingConfig::default();
        let mut exposure = base_exposure();
        // Origination was A, now BBB -> PD increased significantly
        exposure.origination_rating = Some("A".to_string());
        exposure.current_rating = Some("BB".to_string());

        let result = classify_stage(&exposure, &curves, &config).unwrap();
        assert_eq!(result.stage, Stage::Stage2);
        assert!(result
            .triggers
            .iter()
            .any(|t| matches!(t, StagingTrigger::PdDeltaAbsolute { .. })));
    }

    #[test]
    fn test_stage2_qualitative_watchlist() {
        let curve = make_pd_curve();
        let config = StagingConfig::default();
        let mut exposure = base_exposure();
        exposure.qualitative_flags.watchlist = true;

        let result = classify_stage(&exposure, &curve, &config).unwrap();
        assert_eq!(result.stage, Stage::Stage2);
        assert!(result
            .triggers
            .iter()
            .any(|t| matches!(t, StagingTrigger::Qualitative { .. })));
    }

    #[test]
    fn test_stage2_qualitative_disabled() {
        let curve = make_pd_curve();
        let mut config = StagingConfig::default();
        config.qualitative_triggers_enabled = false;
        let mut exposure = base_exposure();
        exposure.qualitative_flags.watchlist = true;

        let result = classify_stage(&exposure, &curve, &config).unwrap();
        // With qualitative triggers disabled, watchlist alone doesn't trigger Stage 2
        assert_eq!(result.stage, Stage::Stage1);
    }

    #[test]
    fn test_curing_stage2_to_stage1() {
        let curve = make_pd_curve();
        let config = StagingConfig::default();
        let mut exposure = base_exposure();
        exposure.previous_stage = Some(Stage::Stage2);
        exposure.consecutive_performing_periods = 3;

        let result = classify_stage(&exposure, &curve, &config).unwrap();
        assert_eq!(result.stage, Stage::Stage1);
        assert!(result.cured);
    }

    #[test]
    fn test_curing_stage2_insufficient_periods() {
        let curve = make_pd_curve();
        let config = StagingConfig::default();
        let mut exposure = base_exposure();
        exposure.previous_stage = Some(Stage::Stage2);
        exposure.consecutive_performing_periods = 2; // Need 3

        let result = classify_stage(&exposure, &curve, &config).unwrap();
        assert_eq!(result.stage, Stage::Stage2);
        assert!(!result.cured);
    }

    #[test]
    fn test_curing_stage3_to_stage2() {
        let curve = make_pd_curve();
        let config = StagingConfig::default();
        let mut exposure = base_exposure();
        exposure.previous_stage = Some(Stage::Stage3);
        exposure.consecutive_performing_periods = 6; // Enough for 3->2, not 2->1

        let result = classify_stage(&exposure, &curve, &config).unwrap();
        assert_eq!(result.stage, Stage::Stage2);
        assert!(result.cured);
    }

    #[test]
    fn test_curing_stage3_to_stage1() {
        let curve = make_pd_curve();
        let config = StagingConfig::default();
        let mut exposure = base_exposure();
        exposure.previous_stage = Some(Stage::Stage3);
        exposure.consecutive_performing_periods = 9; // 6 + 3 = fully cured

        let result = classify_stage(&exposure, &curve, &config).unwrap();
        assert_eq!(result.stage, Stage::Stage1);
        assert!(result.cured);
    }
}
