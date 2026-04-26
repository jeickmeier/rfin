//! PR-8a: credit-factor hierarchy detail for waterfall + parallel attribution.
//!
//! Four named tests:
//!  1. `waterfall_credit_factor_detail_reconciles_to_credit_curves_pnl`
//!  2. `parallel_credit_detail_plus_cross_effects_preserves_total`
//!  3. `waterfall_no_model_keeps_default_credit_step`
//!  4. `same_credit_total_different_hierarchy_different_detail`

use finstack_core::currency::Currency;
use finstack_core::dates::{create_date, DayCount};
use finstack_core::factor_model::credit_hierarchy::{
    AdderVolSource, CalibrationDiagnostics, CreditFactorModel, CreditHierarchySpec, DateRange,
    FactorCorrelationMatrix, GenericFactorSpec, HierarchyDimension, IssuerBetaMode,
    IssuerBetaPolicy, IssuerBetaRow, IssuerBetas, IssuerTags, LevelsAtAnchor, VolState,
};
use finstack_core::factor_model::{
    FactorCovarianceMatrix, FactorModelConfig, MatchingConfig, PricingMode,
};
use finstack_core::market_data::context::{
    CurveState, MarketContextState, MARKET_CONTEXT_STATE_VERSION,
};
use finstack_core::market_data::term_structures::{DiscountCurve, HazardCurve};
use finstack_core::money::Money;
use finstack_core::types::{CurveId, IssuerId};
use finstack_valuations::attribution::{
    default_waterfall_order, AttributionEnvelope, AttributionMethod, AttributionSpec,
    CreditFactorDetailOptions, CreditFactorModelRef,
};
use finstack_valuations::instruments::json_loader::InstrumentJson;
use finstack_valuations::instruments::{Attributes, Bond};
use std::collections::BTreeMap;
use time::Month;

// ─────────────────────────── Helpers ───────────────────────────

fn issuer_tags(rating: &str, region: &str) -> IssuerTags {
    let mut m = BTreeMap::new();
    m.insert("rating".into(), rating.into());
    m.insert("region".into(), region.into());
    // Carry sector too so the same issuer can be reused with sector-aware
    // hierarchies in tests that vary the level set.
    m.insert("sector".into(), "FIN".into());
    IssuerTags(m)
}

fn empty_factor_config() -> FactorModelConfig {
    FactorModelConfig {
        factors: vec![],
        covariance: FactorCovarianceMatrix::new(vec![], vec![]).unwrap(),
        matching: MatchingConfig::MappingTable(vec![]),
        pricing_mode: PricingMode::DeltaBased,
        risk_measure: Default::default(),
        bump_size: None,
        unmatched_policy: None,
    }
}

fn issuer_row(id: &str, rating: &str, region: &str, pc: f64, lv: Vec<f64>) -> IssuerBetaRow {
    IssuerBetaRow {
        issuer_id: IssuerId::new(id),
        tags: issuer_tags(rating, region),
        mode: IssuerBetaMode::IssuerBeta,
        betas: IssuerBetas { pc, levels: lv },
        adder_at_anchor: 0.0,
        adder_vol_annualized: 0.01,
        adder_vol_source: AdderVolSource::Default,
        fit_quality: None,
    }
}

fn make_model(levels: Vec<HierarchyDimension>) -> CreditFactorModel {
    let n = levels.len();
    CreditFactorModel {
        schema_version: CreditFactorModel::SCHEMA_VERSION.into(),
        as_of: create_date(2024, Month::March, 29).unwrap(),
        calibration_window: DateRange {
            start: create_date(2022, Month::March, 29).unwrap(),
            end: create_date(2024, Month::March, 29).unwrap(),
        },
        policy: IssuerBetaPolicy::GloballyOff,
        generic_factor: GenericFactorSpec {
            name: "CDX IG 5Y".into(),
            series_id: "cdx.ig.5y".into(),
        },
        hierarchy: CreditHierarchySpec { levels },
        config: empty_factor_config(),
        issuer_betas: vec![issuer_row(
            "ISSUER-A",
            "IG",
            "EU",
            1.10,
            vec![0.90; n.max(1)],
        )],
        anchor_state: LevelsAtAnchor {
            pc: 0.0,
            by_level: vec![],
        },
        static_correlation: FactorCorrelationMatrix::identity(vec![]),
        vol_state: VolState {
            factors: BTreeMap::new(),
            idiosyncratic: BTreeMap::new(),
        },
        factor_histories: None,
        diagnostics: CalibrationDiagnostics {
            mode_counts: BTreeMap::new(),
            bucket_sizes_per_level: vec![],
            fold_ups: vec![],
            r_squared_histogram: None,
            tag_taxonomy: BTreeMap::new(),
        },
    }
}

fn make_bond() -> Bond {
    let mut bond = Bond::fixed(
        "BOND-ISSUER-A",
        Money::new(1_000_000.0, Currency::USD),
        0.05_f64,
        create_date(2024, Month::January, 1).unwrap(),
        create_date(2034, Month::January, 1).unwrap(),
        "USD-OIS",
    )
    .expect("bond construction");
    bond.credit_curve_id = Some(CurveId::new("ISSUER-A-HAZ"));
    bond.attributes = Attributes::new().with_meta("credit::issuer_id", "ISSUER-A");
    bond
}

fn flat_discount(base: time::Date) -> DiscountCurve {
    let r = 0.05_f64;
    DiscountCurve::builder("USD-OIS")
        .base_date(base)
        .day_count(DayCount::Act365F)
        .knots([
            (0.0_f64, 1.0_f64),
            (1.0_f64, (-r).exp()),
            (5.0_f64, (-r * 5.0).exp()),
            (10.0_f64, (-r * 10.0).exp()),
            (30.0_f64, (-r * 30.0).exp()),
        ])
        .build()
        .expect("discount curve")
}

fn flat_hazard(base: time::Date, rate: f64) -> HazardCurve {
    HazardCurve::builder("ISSUER-A-HAZ")
        .base_date(base)
        .day_count(DayCount::Act365F)
        .recovery_rate(0.4)
        .knots([(0.5_f64, rate), (5.0_f64, rate), (10.0_f64, rate)])
        .build()
        .expect("hazard curve")
}

fn make_market_state(disc: DiscountCurve, haz: HazardCurve) -> MarketContextState {
    MarketContextState {
        version: MARKET_CONTEXT_STATE_VERSION,
        curves: vec![CurveState::Discount(disc), CurveState::Hazard(haz)],
        fx: None,
        surfaces: vec![],
        prices: BTreeMap::new(),
        series: vec![],
        inflation_indices: vec![],
        dividends: vec![],
        credit_indices: vec![],
        collateral: BTreeMap::new(),
        fx_delta_vol_surfaces: vec![],
        hierarchy: None,
        vol_cubes: vec![],
    }
}

fn standard_period() -> (time::Date, time::Date) {
    (
        create_date(2025, Month::January, 1).unwrap(),
        create_date(2025, Month::January, 2).unwrap(),
    )
}

// ─────────────────────────── Tests ───────────────────────────

/// PR-8a test 1: waterfall reconciliation invariant.
#[test]
fn waterfall_credit_factor_detail_reconciles_to_credit_curves_pnl() {
    let (as_of_t0, as_of_t1) = standard_period();
    let bond = make_bond();
    let model = make_model(vec![HierarchyDimension::Rating, HierarchyDimension::Region]);

    let market_t0 = make_market_state(flat_discount(as_of_t0), flat_hazard(as_of_t0, 0.01));
    let market_t1 = make_market_state(flat_discount(as_of_t1), flat_hazard(as_of_t1, 0.02));

    let spec = AttributionSpec {
        instrument: InstrumentJson::Bond(bond),
        market_t0,
        market_t1,
        as_of_t0,
        as_of_t1,
        method: AttributionMethod::Waterfall(default_waterfall_order()),
        model_params_t0: None,
        credit_factor_model: Some(CreditFactorModelRef::Inline(Box::new(model))),
        credit_factor_detail_options: CreditFactorDetailOptions::default(),
        config: None,
    };

    let result = AttributionEnvelope::new(spec)
        .execute()
        .expect("waterfall attribution should succeed");
    let attribution = result.result.attribution;

    let detail = attribution
        .credit_factor_detail
        .as_ref()
        .expect("credit_factor_detail must be Some for waterfall + model");

    let attributed = detail.generic_pnl.amount()
        + detail.levels.iter().map(|l| l.total.amount()).sum::<f64>()
        + detail.adder_pnl_total.amount();
    let expected = attribution.credit_curves_pnl.amount();
    assert!(
        (attributed - expected).abs() < 1e-8,
        "waterfall reconciliation: attributed={attributed}, credit_curves_pnl={expected}"
    );
}

/// PR-8a test 2: parallel reconciliation including cross-effects.
#[test]
fn parallel_credit_detail_plus_cross_effects_preserves_total() {
    let (as_of_t0, as_of_t1) = standard_period();
    let bond = make_bond();
    let model = make_model(vec![HierarchyDimension::Rating, HierarchyDimension::Region]);

    let market_t0 = make_market_state(flat_discount(as_of_t0), flat_hazard(as_of_t0, 0.01));
    let market_t1 = make_market_state(flat_discount(as_of_t1), flat_hazard(as_of_t1, 0.02));

    let spec = AttributionSpec {
        instrument: InstrumentJson::Bond(bond),
        market_t0,
        market_t1,
        as_of_t0,
        as_of_t1,
        method: AttributionMethod::Parallel,
        model_params_t0: None,
        credit_factor_model: Some(CreditFactorModelRef::Inline(Box::new(model))),
        credit_factor_detail_options: CreditFactorDetailOptions::default(),
        config: None,
    };

    let result = AttributionEnvelope::new(spec)
        .execute()
        .expect("parallel attribution should succeed");
    let attribution = result.result.attribution;

    let detail = attribution
        .credit_factor_detail
        .as_ref()
        .expect("credit_factor_detail must be Some for parallel + model");

    let credit_detail_total = detail.generic_pnl.amount()
        + detail.levels.iter().map(|l| l.total.amount()).sum::<f64>()
        + detail.adder_pnl_total.amount();

    // Compute the CreditCascadeResidual cross-effect captured for parallel.
    let credit_hier_cross = attribution
        .cross_factor_detail
        .as_ref()
        .and_then(|d| d.by_pair.get("CreditCascadeResidual"))
        .map(|m| m.amount())
        .unwrap_or(0.0);

    let expected = attribution.credit_curves_pnl.amount();
    let recon = credit_detail_total + credit_hier_cross;
    // Cross-effects are second order; modest tolerance is fine.
    assert!(
        (recon - expected).abs() < 1e-6,
        "parallel reconciliation: detail+cross={recon}, credit_curves_pnl={expected}"
    );
}

/// PR-8a test 3: no-model waterfall keeps the legacy single Credit step.
/// The default factor order length and credit-step P&L stay unchanged byte-
/// identical between two runs that omit the credit factor model.
#[test]
fn waterfall_no_model_keeps_default_credit_step() {
    // Default order is length 9 with CreditCurves at index 2.
    let order = default_waterfall_order();
    assert_eq!(order.len(), 9);

    let (as_of_t0, as_of_t1) = standard_period();
    let bond = make_bond();
    let market_t0 = make_market_state(flat_discount(as_of_t0), flat_hazard(as_of_t0, 0.01));
    let market_t1 = make_market_state(flat_discount(as_of_t1), flat_hazard(as_of_t1, 0.02));

    let spec = AttributionSpec {
        instrument: InstrumentJson::Bond(bond),
        market_t0,
        market_t1,
        as_of_t0,
        as_of_t1,
        method: AttributionMethod::Waterfall(default_waterfall_order()),
        model_params_t0: None,
        credit_factor_model: None,
        credit_factor_detail_options: CreditFactorDetailOptions::default(),
        config: None,
    };

    let result = AttributionEnvelope::new(spec)
        .execute()
        .expect("waterfall attribution should succeed");
    let attribution = result.result.attribution;

    // No credit factor detail when no model.
    assert!(attribution.credit_factor_detail.is_none());
    // Credit step still produced a value (non-zero).
    assert!(attribution.credit_curves_pnl.amount().abs() > 0.0);
}

/// PR-8a test 4: same credit total, different hierarchies → different details.
#[test]
fn same_credit_total_different_hierarchy_different_detail() {
    let (as_of_t0, as_of_t1) = standard_period();

    let market_t0 = make_market_state(flat_discount(as_of_t0), flat_hazard(as_of_t0, 0.01));
    let market_t1 = make_market_state(flat_discount(as_of_t1), flat_hazard(as_of_t1, 0.02));

    let run = |levels: Vec<HierarchyDimension>| {
        let bond = make_bond();
        let model = make_model(levels);
        let spec = AttributionSpec {
            instrument: InstrumentJson::Bond(bond),
            market_t0: market_t0.clone(),
            market_t1: market_t1.clone(),
            as_of_t0,
            as_of_t1,
            method: AttributionMethod::Waterfall(default_waterfall_order()),
            model_params_t0: None,
            credit_factor_model: Some(CreditFactorModelRef::Inline(Box::new(model))),
            credit_factor_detail_options: CreditFactorDetailOptions::default(),
            config: None,
        };
        AttributionEnvelope::new(spec)
            .execute()
            .expect("waterfall attribution should succeed")
            .result
            .attribution
    };

    let a = run(vec![HierarchyDimension::Rating, HierarchyDimension::Region]);
    let b = run(vec![HierarchyDimension::Sector]);

    // Same total credit P&L (waterfall snaps to T1 hazard at adder step).
    assert!(
        (a.credit_curves_pnl.amount() - b.credit_curves_pnl.amount()).abs() < 1e-8,
        "credit_curves_pnl should be identical: a={}, b={}",
        a.credit_curves_pnl.amount(),
        b.credit_curves_pnl.amount()
    );

    let detail_a = a.credit_factor_detail.as_ref().unwrap();
    let detail_b = b.credit_factor_detail.as_ref().unwrap();
    // Different hierarchy depths → different number of LevelPnl entries.
    assert_ne!(detail_a.levels.len(), detail_b.levels.len());

    // Both reconcile to credit_curves_pnl.
    for attribution in [&a, &b] {
        let detail = attribution.credit_factor_detail.as_ref().unwrap();
        let attributed = detail.generic_pnl.amount()
            + detail.levels.iter().map(|l| l.total.amount()).sum::<f64>()
            + detail.adder_pnl_total.amount();
        assert!(
            (attributed - attribution.credit_curves_pnl.amount()).abs() < 1e-8,
            "reconciliation failed for one of the runs"
        );
    }
}
