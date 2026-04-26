//! Integration tests for [`finstack_valuations::factor_model::credit_calibration`].
//!
//! Implements the seven required PR-4 tests from the design.

use std::collections::{BTreeMap, BTreeSet};

use finstack_core::dates::{create_date, Date};
use finstack_core::factor_model::credit_hierarchy::{
    AdderVolSource, CreditFactorModel, CreditHierarchySpec, FactorVolModel, GenericFactorSpec,
    HierarchyDimension, IssuerBetaMode, IssuerBetaOverride, IssuerBetaPolicy, IssuerTags,
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

    // M-4: verify tag_taxonomy contains the expected dimension and observed values.
    let taxonomy = &model.diagnostics.tag_taxonomy;
    assert!(
        taxonomy.contains_key("rating"),
        "tag_taxonomy must contain dimension key 'rating'"
    );
    let rating_values = &taxonomy["rating"];
    assert_eq!(
        *rating_values,
        BTreeSet::from(["IG".to_owned(), "HY".to_owned()]),
        "rating dimension must observe exactly IG and HY"
    );
    // Single-level hierarchy: only 'rating' should appear as a key.
    assert_eq!(
        taxonomy.len(),
        1,
        "single-level Rating hierarchy must produce exactly one taxonomy key"
    );
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

    // M-4: verify tag_taxonomy for a two-level Rating × Region hierarchy.
    let taxonomy = &model.diagnostics.tag_taxonomy;
    assert!(
        taxonomy.contains_key("rating"),
        "tag_taxonomy must contain dimension key 'rating'"
    );
    assert!(
        taxonomy.contains_key("region"),
        "tag_taxonomy must contain dimension key 'region'"
    );
    assert_eq!(
        taxonomy["rating"],
        BTreeSet::from(["IG".to_owned(), "HY".to_owned()]),
        "rating dimension must observe exactly IG and HY"
    );
    assert_eq!(
        taxonomy["region"],
        BTreeSet::from(["EU".to_owned(), "NA".to_owned(), "APAC".to_owned()]),
        "region dimension must observe exactly EU, NA, and APAC"
    );
}

// ---------------------------------------------------------------------------
// I-2: sparse bucket emits None for empty dates (factor variance excludes gap)
// ---------------------------------------------------------------------------

/// Fixture with 2 issuers, each the sole member of its Rating bucket.
/// On date index 5 (0-based), ISSUER-IG has no observation (spread = None).
/// That date should produce a `None` factor observation for the IG bucket,
/// which must be excluded from the annualized variance calculation.
#[test]
fn sparse_bucket_emits_none_for_empty_dates() {
    let n = 12usize; // 12-month panel
    let as_of = d(2024, Month::December, 31);
    let dates = monthly_dates(n, as_of);

    let generic_values: Vec<f64> = (0..n).map(|i| 0.5 * (i as f64).sin()).collect();

    // Two issuers: IG (sole member of its bucket) and HY (sole member of its bucket).
    // IG is missing on date index 5.
    let mut spreads: BTreeMap<IssuerId, Vec<Option<f64>>> = BTreeMap::new();
    let mut issuer_tags_map: BTreeMap<IssuerId, IssuerTags> = BTreeMap::new();
    let mut asof_spreads: BTreeMap<IssuerId, f64> = BTreeMap::new();

    let ig_id = IssuerId::new("ISSUER-IG");
    let hy_id = IssuerId::new("ISSUER-HY");

    // IG series: present on all dates except index 5.
    let ig_series: Vec<Option<f64>> = (0..n)
        .map(|i| {
            if i == 5 {
                None
            } else {
                Some(100.0 + 0.8 * generic_values[i] + 0.05 * (i as f64).cos())
            }
        })
        .collect();
    asof_spreads.insert(ig_id.clone(), ig_series[n - 1].unwrap());
    spreads.insert(ig_id.clone(), ig_series);
    let mut ig_tags_map = BTreeMap::new();
    ig_tags_map.insert("rating".to_owned(), "IG".to_owned());
    issuer_tags_map.insert(ig_id.clone(), IssuerTags(ig_tags_map));

    // HY series: fully observed.
    let hy_series: Vec<Option<f64>> = (0..n)
        .map(|i| Some(200.0 + 1.2 * generic_values[i] + 0.03 * (i as f64).sin()))
        .collect();
    asof_spreads.insert(hy_id.clone(), hy_series[n - 1].unwrap());
    spreads.insert(hy_id.clone(), hy_series);
    let mut hy_tags_map = BTreeMap::new();
    hy_tags_map.insert("rating".to_owned(), "HY".to_owned());
    issuer_tags_map.insert(hy_id.clone(), IssuerTags(hy_tags_map));

    let inputs = CreditCalibrationInputs {
        history_panel: HistoryPanel { dates, spreads },
        issuer_tags: IssuerTagPanel {
            tags: issuer_tags_map,
        },
        generic_factor: GenericFactorSeries {
            spec: GenericFactorSpec {
                name: "CDX".to_owned(),
                series_id: "cdx".to_owned(),
            },
            values: generic_values,
        },
        as_of,
        asof_spreads,
        idiosyncratic_overrides: BTreeMap::new(),
    };

    // Use GloballyOff so betas=1 and the IG factor series = issuer's residual mean.
    let cfg = CreditCalibrationConfig {
        min_bucket_size_per_level: BucketSizeThresholds { per_level: vec![1] },
        ..config_with(
            IssuerBetaPolicy::GloballyOff,
            vec![HierarchyDimension::Rating],
        )
    };

    let model = CreditCalibrator::new(cfg)
        .calibrate(inputs)
        .expect("sparse-panel calibration succeeds");
    model.validate().expect("model validates");

    // The IG factor history is in returns space (dates[1..]). A missing spread
    // at index 5 makes both return[4] (spread[5]-spread[4]) and return[5]
    // (spread[6]-spread[5]) uncomputable. The empty-bucket entries are stored
    // as 0.0 (the dense-compatible sentinel) so that FactorHistories can
    // round-trip through JSON without serde type errors.
    let fh = model
        .factor_histories
        .as_ref()
        .expect("factor_histories present");
    let ig_factor_id = finstack_core::factor_model::FactorId::new("credit::level0::Rating::IG");
    let ig_history = fh
        .values
        .get(&ig_factor_id)
        .expect("IG factor history present");

    // No NaN values must appear in the stored history (ensuring JSON round-trip).
    assert!(
        ig_history.iter().all(|v| v.is_finite()),
        "IG factor history must contain no NaN or Inf (JSON round-trip requirement)"
    );

    // Exactly 2 zero-sentinel entries must appear (the two return periods that
    // straddle the missing spread at index 5).
    let zero_count = ig_history.iter().filter(|&&v| v == 0.0).count();
    assert_eq!(
        zero_count, 2,
        "IG factor history must contain exactly 2 zero sentinels (empty-bucket dates)"
    );

    // The factor variance is computed before flattening, over only the observed
    // (Some) values. It must be strictly positive (the IG series is non-constant).
    let vol_entry = model
        .vol_state
        .factors
        .get(&ig_factor_id)
        .expect("IG factor vol present");
    let FactorVolModel::Sample { variance } = vol_entry;
    assert!(
        *variance > 0.0,
        "IG factor variance must be positive (computed over non-zero-sentinel dates)"
    );

    // Round-trip serialization must succeed without error.
    let json = serde_json::to_string(&model).expect("serialize succeeds");
    let model2: CreditFactorModel = serde_json::from_str(&json).expect("deserialize succeeds");
    assert_eq!(model.as_of, model2.as_of, "round-trip preserves as_of");
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

// ---------------------------------------------------------------------------
// PR-5a Test 1: caller override wins over IssuerBeta history
// ---------------------------------------------------------------------------

/// An idiosyncratic override supplied for an `IssuerBeta` issuer must win over
/// the vol computed from that issuer's residual history, and the source must
/// record `CallerSupplied`.
#[test]
fn idiosyncratic_override_wins_over_history() {
    // Use Dynamic policy with low min_history so ISSUER-A gets IssuerBeta mode.
    let policy = IssuerBetaPolicy::Dynamic {
        min_history: 12,
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

    let fixture = fixture_panel();
    let override_vol = 0.9999_f64;
    let mut overrides = BTreeMap::new();
    overrides.insert(IssuerId::new("ISSUER-A"), override_vol);

    let inputs = CreditCalibrationInputs {
        idiosyncratic_overrides: overrides,
        ..fixture.into_inputs()
    };

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
        "ISSUER-A must be IssuerBeta"
    );
    assert!(
        (row_a.adder_vol_annualized - override_vol).abs() < 1e-12,
        "adder_vol_annualized must equal override; got {}",
        row_a.adder_vol_annualized
    );
    assert!(
        matches!(row_a.adder_vol_source, AdderVolSource::CallerSupplied),
        "adder_vol_source must be CallerSupplied; got {:?}",
        row_a.adder_vol_source
    );
}

// ---------------------------------------------------------------------------
// PR-5a Test 2: caller override wins over BucketOnly peer proxy
// ---------------------------------------------------------------------------

/// An idiosyncratic override supplied for a `BucketOnly` issuer must win over
/// the peer-proxy fallback, and the source must record `CallerSupplied`.
#[test]
fn idiosyncratic_override_wins_over_bucket_only_peer_proxy() {
    // GloballyOff → all issuers are BucketOnly.
    let cfg = CreditCalibrationConfig {
        min_bucket_size_per_level: BucketSizeThresholds {
            per_level: vec![1, 1],
        },
        ..config_with(
            IssuerBetaPolicy::GloballyOff,
            vec![HierarchyDimension::Rating, HierarchyDimension::Region],
        )
    };

    let fixture = fixture_panel();
    let override_vol = 0.7777_f64;
    let mut overrides = BTreeMap::new();
    overrides.insert(IssuerId::new("ISSUER-D"), override_vol);

    let inputs = CreditCalibrationInputs {
        idiosyncratic_overrides: overrides,
        ..fixture.into_inputs()
    };

    let model = CreditCalibrator::new(cfg)
        .calibrate(inputs)
        .expect("calibration succeeds");

    let row_d = model
        .issuer_betas
        .iter()
        .find(|r| r.issuer_id.as_str() == "ISSUER-D")
        .expect("ISSUER-D row present");

    assert!(
        matches!(row_d.mode, IssuerBetaMode::BucketOnly),
        "ISSUER-D must be BucketOnly"
    );
    assert!(
        (row_d.adder_vol_annualized - override_vol).abs() < 1e-12,
        "adder_vol_annualized must equal override; got {}",
        row_d.adder_vol_annualized
    );
    assert!(
        matches!(row_d.adder_vol_source, AdderVolSource::CallerSupplied),
        "adder_vol_source must be CallerSupplied; got {:?}",
        row_d.adder_vol_source
    );
}

// ---------------------------------------------------------------------------
// PR-5a Test 3: BucketOnly uses peer proxy at deepest level
// ---------------------------------------------------------------------------

/// Fixture: 1 BucketOnly issuer X with tags `{rating: IG, region: EU}` plus
/// 2 IssuerBeta peers also tagged `{rating: IG, region: EU}`.
/// X's adder vol must equal the mean of those 2 peers' vols, and the
/// `peer_bucket` must be `"IG.EU"` (the deepest level = level-1 path).
#[test]
fn bucket_only_uses_peer_proxy_at_deepest_level() {
    let n = 24usize;
    let as_of = d(2024, Month::March, 31);
    let dates = monthly_dates(n, as_of);

    let generic_values: Vec<f64> = (0..n).map(|i| 0.5 * (i as f64).sin()).collect();

    let mut spreads: BTreeMap<IssuerId, Vec<Option<f64>>> = BTreeMap::new();
    let mut issuer_tags_map: BTreeMap<IssuerId, IssuerTags> = BTreeMap::new();
    let mut asof_spreads: BTreeMap<IssuerId, f64> = BTreeMap::new();

    // 2 IssuerBeta peers in IG.EU bucket.
    for (idx, id) in ["PEER-1", "PEER-2"].iter().enumerate() {
        let issuer_id = IssuerId::new(*id);
        let series: Vec<Option<f64>> = (0..n)
            .map(|i| {
                Some(
                    100.0
                        + (idx as f64) * 20.0
                        + 0.8 * generic_values[i]
                        + 0.1 * ((idx as f64) + (i as f64) * 0.3).sin(),
                )
            })
            .collect();
        asof_spreads.insert(issuer_id.clone(), series[n - 1].unwrap());
        spreads.insert(issuer_id.clone(), series);
        issuer_tags_map.insert(issuer_id, tags_for("IG", "EU"));
    }

    // BucketOnly issuer X in the same IG.EU bucket.
    let x_id = IssuerId::new("ISSUER-X");
    let x_series: Vec<Option<f64>> = (0..n)
        .map(|i| Some(150.0 + 0.9 * generic_values[i] + 0.05 * ((i as f64) * 0.7).cos()))
        .collect();
    asof_spreads.insert(x_id.clone(), x_series[n - 1].unwrap());
    spreads.insert(x_id.clone(), x_series);
    issuer_tags_map.insert(x_id.clone(), tags_for("IG", "EU"));

    // Policy: peers are IssuerBeta, X is BucketOnly via ForceIssuerBeta +
    // ForceBucketOnly overrides.
    let mut overrides = BTreeMap::new();
    overrides.insert(IssuerId::new("PEER-1"), IssuerBetaOverride::ForceIssuerBeta);
    overrides.insert(IssuerId::new("PEER-2"), IssuerBetaOverride::ForceIssuerBeta);
    overrides.insert(x_id.clone(), IssuerBetaOverride::ForceBucketOnly);

    let policy = IssuerBetaPolicy::Dynamic {
        min_history: 12,
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

    let inputs = CreditCalibrationInputs {
        history_panel: HistoryPanel { dates, spreads },
        issuer_tags: IssuerTagPanel {
            tags: issuer_tags_map,
        },
        generic_factor: GenericFactorSeries {
            spec: GenericFactorSpec {
                name: "CDX".to_owned(),
                series_id: "cdx".to_owned(),
            },
            values: generic_values,
        },
        as_of,
        asof_spreads,
        idiosyncratic_overrides: BTreeMap::new(),
    };

    let model = CreditCalibrator::new(cfg)
        .calibrate(inputs)
        .expect("calibration succeeds");

    // Get peer vols from the model.
    let peer1_vol = model
        .issuer_betas
        .iter()
        .find(|r| r.issuer_id.as_str() == "PEER-1")
        .map(|r| r.adder_vol_annualized)
        .expect("PEER-1 row");
    let peer2_vol = model
        .issuer_betas
        .iter()
        .find(|r| r.issuer_id.as_str() == "PEER-2")
        .map(|r| r.adder_vol_annualized)
        .expect("PEER-2 row");

    let expected_mean = (peer1_vol + peer2_vol) / 2.0;

    let row_x = model
        .issuer_betas
        .iter()
        .find(|r| r.issuer_id.as_str() == "ISSUER-X")
        .expect("ISSUER-X row present");

    assert!(
        matches!(row_x.mode, IssuerBetaMode::BucketOnly),
        "ISSUER-X must be BucketOnly"
    );
    assert!(
        (row_x.adder_vol_annualized - expected_mean).abs() < 1e-9,
        "adder_vol must equal mean of IG.EU peers ({expected_mean}); got {}",
        row_x.adder_vol_annualized
    );
    // peer_bucket must be "IG.EU" (the deepest level path where peers exist).
    assert!(
        matches!(
            &row_x.adder_vol_source,
            AdderVolSource::BucketPeerProxy { peer_bucket }
            if peer_bucket == "IG.EU"
        ),
        "adder_vol_source must be BucketPeerProxy {{ peer_bucket: \"IG.EU\" }}; got {:?}",
        row_x.adder_vol_source
    );
}

// ---------------------------------------------------------------------------
// PR-5a Test 4: peer proxy falls back to parent bucket level
// ---------------------------------------------------------------------------

/// Fixture: BucketOnly issuer X tagged `{rating: IG, region: APAC}` but there
/// are no IG.APAC IssuerBeta peers. There ARE IG.EU IssuerBeta peers.
/// X must proxy from IG level (level-0, the coarsest), not IG.APAC.
/// `peer_bucket = "IG"`.
#[test]
fn bucket_peer_proxy_falls_back_to_parent() {
    let n = 24usize;
    let as_of = d(2024, Month::March, 31);
    let dates = monthly_dates(n, as_of);

    let generic_values: Vec<f64> = (0..n).map(|i| 0.5 * (i as f64).sin()).collect();

    let mut spreads: BTreeMap<IssuerId, Vec<Option<f64>>> = BTreeMap::new();
    let mut issuer_tags_map: BTreeMap<IssuerId, IssuerTags> = BTreeMap::new();
    let mut asof_spreads: BTreeMap<IssuerId, f64> = BTreeMap::new();

    // 2 IssuerBeta peers in IG.EU bucket (different region from X).
    for (idx, id) in ["PEER-EU-1", "PEER-EU-2"].iter().enumerate() {
        let issuer_id = IssuerId::new(*id);
        let series: Vec<Option<f64>> = (0..n)
            .map(|i| {
                Some(
                    100.0
                        + (idx as f64) * 20.0
                        + 0.8 * generic_values[i]
                        + 0.1 * ((idx as f64) + (i as f64) * 0.3).sin(),
                )
            })
            .collect();
        asof_spreads.insert(issuer_id.clone(), series[n - 1].unwrap());
        spreads.insert(issuer_id.clone(), series);
        issuer_tags_map.insert(issuer_id, tags_for("IG", "EU"));
    }

    // BucketOnly issuer X in IG.APAC — no IG.APAC IssuerBeta peers.
    let x_id = IssuerId::new("ISSUER-X");
    let x_series: Vec<Option<f64>> = (0..n)
        .map(|i| Some(150.0 + 0.9 * generic_values[i] + 0.05 * ((i as f64) * 0.7).cos()))
        .collect();
    asof_spreads.insert(x_id.clone(), x_series[n - 1].unwrap());
    spreads.insert(x_id.clone(), x_series);
    issuer_tags_map.insert(x_id.clone(), tags_for("IG", "APAC"));

    let mut overrides = BTreeMap::new();
    overrides.insert(
        IssuerId::new("PEER-EU-1"),
        IssuerBetaOverride::ForceIssuerBeta,
    );
    overrides.insert(
        IssuerId::new("PEER-EU-2"),
        IssuerBetaOverride::ForceIssuerBeta,
    );
    overrides.insert(x_id.clone(), IssuerBetaOverride::ForceBucketOnly);

    let policy = IssuerBetaPolicy::Dynamic {
        min_history: 12,
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

    let inputs = CreditCalibrationInputs {
        history_panel: HistoryPanel { dates, spreads },
        issuer_tags: IssuerTagPanel {
            tags: issuer_tags_map,
        },
        generic_factor: GenericFactorSeries {
            spec: GenericFactorSpec {
                name: "CDX".to_owned(),
                series_id: "cdx".to_owned(),
            },
            values: generic_values,
        },
        as_of,
        asof_spreads,
        idiosyncratic_overrides: BTreeMap::new(),
    };

    let model = CreditCalibrator::new(cfg)
        .calibrate(inputs)
        .expect("calibration succeeds");

    let peer1_vol = model
        .issuer_betas
        .iter()
        .find(|r| r.issuer_id.as_str() == "PEER-EU-1")
        .map(|r| r.adder_vol_annualized)
        .expect("PEER-EU-1 row");
    let peer2_vol = model
        .issuer_betas
        .iter()
        .find(|r| r.issuer_id.as_str() == "PEER-EU-2")
        .map(|r| r.adder_vol_annualized)
        .expect("PEER-EU-2 row");

    let expected_mean = (peer1_vol + peer2_vol) / 2.0;

    let row_x = model
        .issuer_betas
        .iter()
        .find(|r| r.issuer_id.as_str() == "ISSUER-X")
        .expect("ISSUER-X row present");

    assert!(
        (row_x.adder_vol_annualized - expected_mean).abs() < 1e-9,
        "adder_vol must equal mean of IG-level peers ({expected_mean}); got {}",
        row_x.adder_vol_annualized
    );
    // Level-1 bucket IG.APAC has no peers → fell back to level-0 bucket "IG".
    assert!(
        matches!(
            &row_x.adder_vol_source,
            AdderVolSource::BucketPeerProxy { peer_bucket }
            if peer_bucket == "IG"
        ),
        "adder_vol_source must be BucketPeerProxy {{ peer_bucket: \"IG\" }}; got {:?}",
        row_x.adder_vol_source
    );
}

// ---------------------------------------------------------------------------
// PR-5a Test 5: peer proxy cascade falls back to global mean
// ---------------------------------------------------------------------------

/// Fixture: BucketOnly issuer X tagged `{rating: HY, region: APAC}` but there
/// are NO HY.APAC or HY IssuerBeta peers anywhere. There are IG IssuerBeta
/// peers in a completely different rating bucket. X's vol must equal the global
/// mean of all IssuerBeta vols. Source = `Default`.
#[test]
fn peer_proxy_cascade_falls_back_to_global() {
    let n = 24usize;
    let as_of = d(2024, Month::March, 31);
    let dates = monthly_dates(n, as_of);

    let generic_values: Vec<f64> = (0..n).map(|i| 0.5 * (i as f64).sin()).collect();

    let mut spreads: BTreeMap<IssuerId, Vec<Option<f64>>> = BTreeMap::new();
    let mut issuer_tags_map: BTreeMap<IssuerId, IssuerTags> = BTreeMap::new();
    let mut asof_spreads: BTreeMap<IssuerId, f64> = BTreeMap::new();

    // 2 IssuerBeta peers in IG.EU bucket (different rating bucket from X).
    for (idx, id) in ["IG-PEER-1", "IG-PEER-2"].iter().enumerate() {
        let issuer_id = IssuerId::new(*id);
        let series: Vec<Option<f64>> = (0..n)
            .map(|i| {
                Some(
                    100.0
                        + (idx as f64) * 20.0
                        + 0.8 * generic_values[i]
                        + 0.1 * ((idx as f64) + (i as f64) * 0.3).sin(),
                )
            })
            .collect();
        asof_spreads.insert(issuer_id.clone(), series[n - 1].unwrap());
        spreads.insert(issuer_id.clone(), series);
        issuer_tags_map.insert(issuer_id, tags_for("IG", "EU"));
    }

    // BucketOnly issuer X in HY.APAC — no HY IssuerBeta peers at any level.
    let x_id = IssuerId::new("ISSUER-X");
    let x_series: Vec<Option<f64>> = (0..n)
        .map(|i| Some(250.0 + 1.2 * generic_values[i] + 0.08 * ((i as f64) * 0.4).cos()))
        .collect();
    asof_spreads.insert(x_id.clone(), x_series[n - 1].unwrap());
    spreads.insert(x_id.clone(), x_series);
    issuer_tags_map.insert(x_id.clone(), tags_for("HY", "APAC"));

    let mut overrides = BTreeMap::new();
    overrides.insert(
        IssuerId::new("IG-PEER-1"),
        IssuerBetaOverride::ForceIssuerBeta,
    );
    overrides.insert(
        IssuerId::new("IG-PEER-2"),
        IssuerBetaOverride::ForceIssuerBeta,
    );
    overrides.insert(x_id.clone(), IssuerBetaOverride::ForceBucketOnly);

    let policy = IssuerBetaPolicy::Dynamic {
        min_history: 12,
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

    let inputs = CreditCalibrationInputs {
        history_panel: HistoryPanel { dates, spreads },
        issuer_tags: IssuerTagPanel {
            tags: issuer_tags_map,
        },
        generic_factor: GenericFactorSeries {
            spec: GenericFactorSpec {
                name: "CDX".to_owned(),
                series_id: "cdx".to_owned(),
            },
            values: generic_values,
        },
        as_of,
        asof_spreads,
        idiosyncratic_overrides: BTreeMap::new(),
    };

    let model = CreditCalibrator::new(cfg)
        .calibrate(inputs)
        .expect("calibration succeeds");

    // Compute the global mean of IssuerBeta from-history vols.
    let ig_peer_vols: Vec<f64> = ["IG-PEER-1", "IG-PEER-2"]
        .iter()
        .map(|id| {
            model
                .issuer_betas
                .iter()
                .find(|r| r.issuer_id.as_str() == *id)
                .map(|r| r.adder_vol_annualized)
                .unwrap_or(0.0)
        })
        .collect();
    let global_mean = ig_peer_vols.iter().sum::<f64>() / (ig_peer_vols.len() as f64);

    let row_x = model
        .issuer_betas
        .iter()
        .find(|r| r.issuer_id.as_str() == "ISSUER-X")
        .expect("ISSUER-X row present");

    assert!(
        (row_x.adder_vol_annualized - global_mean).abs() < 1e-9,
        "adder_vol must equal global mean ({global_mean}); got {}",
        row_x.adder_vol_annualized
    );
    assert!(
        matches!(row_x.adder_vol_source, AdderVolSource::Default),
        "adder_vol_source must be Default (global fallback); got {:?}",
        row_x.adder_vol_source
    );
}

// ---------------------------------------------------------------------------
// PR-5a Test 6: all-BucketOnly model uses 0.0 vol with Default source
// ---------------------------------------------------------------------------

/// When every issuer is `BucketOnly` (GloballyOff policy), there are no
/// `IssuerBeta` peers anywhere. Every issuer must get `adder_vol = 0.0` and
/// `AdderVolSource::Default`.
#[test]
fn peer_proxy_with_no_issuer_beta_anywhere_uses_zero() {
    // GloballyOff → all issuers BucketOnly, no FromHistory vols anywhere.
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

    for row in &model.issuer_betas {
        assert!(
            matches!(row.mode, IssuerBetaMode::BucketOnly),
            "mode must be BucketOnly"
        );
        assert!(
            row.adder_vol_annualized.abs() < 1e-12,
            "adder_vol must be 0.0 when no IssuerBeta peers exist; got {} for {:?}",
            row.adder_vol_annualized,
            row.issuer_id.as_str()
        );
        assert!(
            matches!(row.adder_vol_source, AdderVolSource::Default),
            "adder_vol_source must be Default; got {:?} for {:?}",
            row.adder_vol_source,
            row.issuer_id.as_str()
        );
    }
}
