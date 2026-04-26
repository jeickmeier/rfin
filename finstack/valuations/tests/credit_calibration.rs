//! Integration tests for [`finstack_valuations::factor_model::credit_calibration`].
//!
//! Implements the seven required PR-4 tests from the design.

use std::collections::BTreeMap;

use finstack_core::dates::{create_date, Date};
use finstack_core::factor_model::credit_hierarchy::{
    CreditHierarchySpec, GenericFactorSpec, HierarchyDimension, IssuerBetaMode, IssuerBetaOverride,
    IssuerBetaPolicy, IssuerTags,
};
use finstack_core::types::IssuerId;
use finstack_valuations::factor_model::{
    BetaShrinkage, BucketSizeThresholds, CovarianceStrategy, CreditCalibrationConfig,
    CreditCalibrationInputs, CreditCalibrator, GenericFactorSeries, HistoryPanel, IssuerTagPanel,
    PanelSpace, VolModelChoice,
};
use time::Month;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn d(year: i32, month: Month, day: u8) -> Date {
    create_date(year, month, day).expect("valid date")
}

/// 24-month grid of monthly dates ending at as_of.
fn monthly_dates(n: usize, end: Date) -> Vec<Date> {
    let mut out = Vec::with_capacity(n);
    let mut current = end;
    for _ in 0..n {
        out.push(current);
        // Naively step back by ~30 days. Calendar-month exactness doesn't
        // matter for the calibration math (only ordering does).
        for _ in 0..30 {
            current = current.previous_day().expect("date in range");
        }
    }
    out.reverse();
    out
}

fn tags_for(rating: &str, region: &str) -> IssuerTags {
    let mut t = BTreeMap::new();
    t.insert("rating".to_owned(), rating.to_owned());
    t.insert("region".to_owned(), region.to_owned());
    IssuerTags(t)
}

/// Synthesize a deterministic 24-month panel with 6 issuers in 2 ratings × 3 regions.
fn fixture_panel() -> CalibrationFixture {
    let n = 24;
    let as_of = d(2024, Month::March, 31);
    let dates = monthly_dates(n, as_of);

    // Generic factor: simple deterministic increments.
    let generic_values: Vec<f64> = (0..n).map(|i| 100.0 + 0.5 * (i as f64).sin()).collect();

    // 6 issuers — 3 IG (across regions) + 3 HY (across regions).
    let issuer_specs = [
        ("ISSUER-A", "IG", "EU"),
        ("ISSUER-B", "IG", "NA"),
        ("ISSUER-C", "IG", "APAC"),
        ("ISSUER-D", "HY", "EU"),
        ("ISSUER-E", "HY", "NA"),
        ("ISSUER-F", "HY", "APAC"),
    ];

    let mut spreads: BTreeMap<IssuerId, Vec<Option<f64>>> = BTreeMap::new();
    let mut tags: BTreeMap<IssuerId, IssuerTags> = BTreeMap::new();
    let mut asof_spreads: BTreeMap<IssuerId, f64> = BTreeMap::new();

    for (idx, (id, rating, region)) in issuer_specs.iter().enumerate() {
        let issuer_id = IssuerId::new(*id);
        let base = 100.0 + (idx as f64) * 25.0;
        let beta_pc = 0.7 + 0.05 * (idx as f64);
        let series: Vec<Option<f64>> = (0..n)
            .map(|i| {
                let val = base
                    + beta_pc * (generic_values[i] - 100.0)
                    + 0.1 * ((idx as f64) + (i as f64) * 0.5).cos();
                Some(val)
            })
            .collect();
        asof_spreads.insert(issuer_id.clone(), series[n - 1].unwrap());
        spreads.insert(issuer_id.clone(), series);
        tags.insert(issuer_id, tags_for(rating, region));
    }

    CalibrationFixture {
        history: HistoryPanel { dates, spreads },
        tags: IssuerTagPanel { tags },
        generic: GenericFactorSeries {
            spec: GenericFactorSpec {
                name: "CDX IG 5Y".to_owned(),
                series_id: "cdx.ig.5y".to_owned(),
            },
            values: generic_values,
        },
        as_of,
        asof_spreads,
    }
}

struct CalibrationFixture {
    history: HistoryPanel,
    tags: IssuerTagPanel,
    generic: GenericFactorSeries,
    as_of: Date,
    asof_spreads: BTreeMap<IssuerId, f64>,
}

impl CalibrationFixture {
    fn into_inputs(self) -> CreditCalibrationInputs {
        CreditCalibrationInputs {
            history_panel: self.history,
            issuer_tags: self.tags,
            generic_factor: self.generic,
            as_of: self.as_of,
            asof_spreads: self.asof_spreads,
            idiosyncratic_overrides: BTreeMap::new(),
        }
    }
}

fn config_with(
    policy: IssuerBetaPolicy,
    levels: Vec<HierarchyDimension>,
) -> CreditCalibrationConfig {
    CreditCalibrationConfig {
        policy,
        hierarchy: CreditHierarchySpec {
            levels: levels.clone(),
        },
        min_bucket_size_per_level: BucketSizeThresholds::default_for_levels(levels.len()),
        vol_model: VolModelChoice::Sample,
        covariance_strategy: CovarianceStrategy::Diagonal,
        beta_shrinkage: BetaShrinkage::None,
        use_returns_or_levels: PanelSpace::Returns,
        annualization_factor: 12.0,
    }
}

// ---------------------------------------------------------------------------
// PR-4 Test 1: bit-identical determinism
// ---------------------------------------------------------------------------

#[test]
fn calibration_is_bit_identical_for_same_inputs() {
    let policy = IssuerBetaPolicy::Dynamic {
        min_history: 12,
        overrides: BTreeMap::new(),
    };
    let cfg = config_with(
        policy,
        vec![HierarchyDimension::Rating, HierarchyDimension::Region],
    );
    // Lower bucket-size thresholds so the test fixture (1 issuer per leaf
    // bucket) doesn't hit fold-up by accident.
    let cfg_a = CreditCalibrationConfig {
        min_bucket_size_per_level: BucketSizeThresholds {
            per_level: vec![1, 1],
        },
        ..cfg.clone()
    };
    let cfg_b = cfg_a.clone();

    let inputs_a = fixture_panel().into_inputs();
    let inputs_b = fixture_panel().into_inputs();

    let model_a = CreditCalibrator::new(cfg_a)
        .calibrate(inputs_a)
        .expect("calibration A succeeds");
    let model_b = CreditCalibrator::new(cfg_b)
        .calibrate(inputs_b)
        .expect("calibration B succeeds");

    let json_a = serde_json::to_string(&model_a).expect("serialize A");
    let json_b = serde_json::to_string(&model_b).expect("serialize B");
    assert_eq!(json_a, json_b, "calibration must be bit-identical");

    // Validation must still pass.
    model_a.validate().expect("validate model A");
}

// ---------------------------------------------------------------------------
// PR-4 Test 2: GloballyOff sets all betas to 1.0
// ---------------------------------------------------------------------------

#[test]
fn globally_off_sets_all_betas_to_one() {
    let cfg = CreditCalibrationConfig {
        min_bucket_size_per_level: BucketSizeThresholds {
            per_level: vec![1, 1],
        },
        ..config_with(
            IssuerBetaPolicy::GloballyOff,
            vec![HierarchyDimension::Rating, HierarchyDimension::Region],
        )
    };
    let inputs = fixture_panel().into_inputs();
    let model = CreditCalibrator::new(cfg)
        .calibrate(inputs)
        .expect("calibration succeeds");

    assert!(!model.issuer_betas.is_empty());
    for row in &model.issuer_betas {
        assert!(
            matches!(row.mode, IssuerBetaMode::BucketOnly),
            "mode must be BucketOnly under GloballyOff"
        );
        assert!(
            (row.betas.pc - 1.0).abs() < 1e-12,
            "pc beta must be 1.0; got {}",
            row.betas.pc
        );
        for (k, b) in row.betas.levels.iter().enumerate() {
            assert!(
                (b - 1.0).abs() < 1e-12,
                "level {k} beta must be 1.0; got {b}"
            );
        }
        assert!(row.fit_quality.is_none());
    }
}

// ---------------------------------------------------------------------------
// PR-4 Test 3: Dynamic policy classifies short history as BucketOnly
// ---------------------------------------------------------------------------

#[test]
fn dynamic_policy_classifies_short_history_as_bucket_only() {
    // Set min_history above the fixture's 24 months so every issuer fails the
    // gate.
    let policy = IssuerBetaPolicy::Dynamic {
        min_history: 36,
        overrides: BTreeMap::new(),
    };
    let cfg = CreditCalibrationConfig {
        min_bucket_size_per_level: BucketSizeThresholds {
            per_level: vec![1, 1],
        },
        ..config_with(
            policy,
            vec![HierarchyDimension::Rating, HierarchyDimension::Region],
        )
    };
    let inputs = fixture_panel().into_inputs();
    let model = CreditCalibrator::new(cfg)
        .calibrate(inputs)
        .expect("calibration succeeds");

    for row in &model.issuer_betas {
        assert!(
            matches!(row.mode, IssuerBetaMode::BucketOnly),
            "issuer {:?} should be BucketOnly with insufficient history",
            row.issuer_id.as_str()
        );
        assert!((row.betas.pc - 1.0).abs() < 1e-12);
        for b in &row.betas.levels {
            assert!((b - 1.0).abs() < 1e-12);
        }
    }
}

// ---------------------------------------------------------------------------
// PR-4 Test 4: ForceIssuerBeta override wins over short-history rule
// ---------------------------------------------------------------------------

#[test]
fn override_force_issuer_beta_wins() {
    let mut overrides = BTreeMap::new();
    overrides.insert(
        IssuerId::new("ISSUER-A"),
        IssuerBetaOverride::ForceIssuerBeta,
    );
    let policy = IssuerBetaPolicy::Dynamic {
        min_history: 100, // way above the fixture's 24
        overrides,
    };
    let cfg = CreditCalibrationConfig {
        min_bucket_size_per_level: BucketSizeThresholds {
            per_level: vec![1, 1],
        },
        ..config_with(
            policy,
            vec![HierarchyDimension::Rating, HierarchyDimension::Region],
        )
    };
    let inputs = fixture_panel().into_inputs();
    let model = CreditCalibrator::new(cfg)
        .calibrate(inputs)
        .expect("calibration succeeds");

    let row_a = model
        .issuer_betas
        .iter()
        .find(|r| r.issuer_id.as_str() == "ISSUER-A")
        .expect("ISSUER-A row present");
    assert!(
        matches!(row_a.mode, IssuerBetaMode::IssuerBeta),
        "ForceIssuerBeta override must produce IssuerBeta mode despite short history"
    );

    // All others must remain BucketOnly because they hit the min_history gate.
    for row in &model.issuer_betas {
        if row.issuer_id.as_str() == "ISSUER-A" {
            continue;
        }
        assert!(matches!(row.mode, IssuerBetaMode::BucketOnly));
    }
}

// ---------------------------------------------------------------------------
// PR-4 Test 5: Sparse bucket folds to parent
// ---------------------------------------------------------------------------

#[test]
fn sparse_bucket_folds_to_parent() {
    // Threshold = 5 at level 0 means each rating bucket needs ≥ 5 IssuerBeta
    // issuers. The fixture has 3 IG + 3 HY → both buckets fold up.
    let policy = IssuerBetaPolicy::Dynamic {
        min_history: 12,
        overrides: BTreeMap::new(),
    };
    let cfg = CreditCalibrationConfig {
        min_bucket_size_per_level: BucketSizeThresholds {
            per_level: vec![5, 5],
        },
        ..config_with(
            policy,
            vec![HierarchyDimension::Rating, HierarchyDimension::Region],
        )
    };
    let inputs = fixture_panel().into_inputs();
    let model = CreditCalibrator::new(cfg)
        .calibrate(inputs)
        .expect("calibration succeeds");

    // Folded issuers must have β = 0 at level 0.
    for row in &model.issuer_betas {
        if matches!(row.mode, IssuerBetaMode::IssuerBeta) {
            assert!(
                (row.betas.levels[0]).abs() < 1e-12,
                "issuer {:?} level0 beta should be 0 after fold-up; got {}",
                row.issuer_id.as_str(),
                row.betas.levels[0]
            );
        }
    }

    // FoldUpRecord must be populated.
    assert!(
        !model.diagnostics.fold_ups.is_empty(),
        "diagnostics.fold_ups must record the fold-ups"
    );
    let any_level0 = model
        .diagnostics
        .fold_ups
        .iter()
        .any(|f| f.level_index == 0);
    assert!(any_level0, "fold-up at level 0 must be recorded");
}

// ---------------------------------------------------------------------------
// PR-4 Test 6: Single-level hierarchy → expected factor IDs
// ---------------------------------------------------------------------------

#[test]
fn single_level_hierarchy_builds_expected_factor_ids() {
    let policy = IssuerBetaPolicy::Dynamic {
        min_history: 12,
        overrides: BTreeMap::new(),
    };
    let cfg = CreditCalibrationConfig {
        min_bucket_size_per_level: BucketSizeThresholds { per_level: vec![1] },
        ..config_with(policy, vec![HierarchyDimension::Rating])
    };
    let inputs = fixture_panel().into_inputs();
    let model = CreditCalibrator::new(cfg)
        .calibrate(inputs)
        .expect("calibration succeeds");

    let factor_ids: Vec<String> = model
        .config
        .factors
        .iter()
        .map(|f| f.id.as_str().to_owned())
        .collect();

    let expected = vec![
        "credit::generic".to_owned(),
        "credit::level0::Rating::HY".to_owned(),
        "credit::level0::Rating::IG".to_owned(),
    ];
    assert_eq!(factor_ids, expected);
}

// ---------------------------------------------------------------------------
// PR-4 Test 7: All-BucketOnly calibration succeeds
// ---------------------------------------------------------------------------

#[test]
fn all_bucket_only_calibration_succeeds() {
    let cfg = CreditCalibrationConfig {
        min_bucket_size_per_level: BucketSizeThresholds {
            per_level: vec![1, 1],
        },
        ..config_with(
            IssuerBetaPolicy::GloballyOff,
            vec![HierarchyDimension::Rating, HierarchyDimension::Region],
        )
    };
    let inputs = fixture_panel().into_inputs();
    let model = CreditCalibrator::new(cfg)
        .calibrate(inputs)
        .expect("calibration succeeds");

    // Every issuer is BucketOnly.
    for row in &model.issuer_betas {
        assert!(matches!(row.mode, IssuerBetaMode::BucketOnly));
    }

    // The bucket factor at level 0 (Rating) must equal the cross-sectional
    // mean of issuer residuals after PC peel — since all PC betas = 1.0, the
    // residual is `r_i = ΔS_i - Δgeneric` per period, and the bucket factor
    // series equals the simple average.
    // We don't recompute it numerically here, but ensure validate() holds and
    // each surviving bucket factor has a sample-variance entry.
    model.validate().expect("validate succeeds");
    assert!(model.factor_histories.is_some());
    let fh = model.factor_histories.as_ref().unwrap();
    assert!(fh
        .values
        .contains_key(&finstack_core::factor_model::FactorId::new(
            "credit::generic"
        )));
    assert!(!model.vol_state.factors.is_empty());
}

// ---------------------------------------------------------------------------
// Additional: unsupported PR-5a/b features error cleanly
// ---------------------------------------------------------------------------

#[test]
fn rejects_garch_vol_model() {
    let cfg = CreditCalibrationConfig {
        vol_model: VolModelChoice::Garch,
        ..config_with(
            IssuerBetaPolicy::GloballyOff,
            vec![HierarchyDimension::Rating],
        )
    };
    let inputs = fixture_panel().into_inputs();
    assert!(CreditCalibrator::new(cfg).calibrate(inputs).is_err());
}

#[test]
fn rejects_ridge_covariance() {
    let cfg = CreditCalibrationConfig {
        covariance_strategy: CovarianceStrategy::Ridge { alpha: 0.1 },
        ..config_with(
            IssuerBetaPolicy::GloballyOff,
            vec![HierarchyDimension::Rating],
        )
    };
    let inputs = fixture_panel().into_inputs();
    assert!(CreditCalibrator::new(cfg).calibrate(inputs).is_err());
}
