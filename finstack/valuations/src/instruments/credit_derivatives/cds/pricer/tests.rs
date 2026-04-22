#![allow(clippy::expect_used, clippy::panic)]

use super::helpers::{
    approx_default_density, date_from_hazard_time, df_asof_to, disc_t, haz_t,
    midpoint_default_date, restructuring_adjustment_factor, settlement_date, sp_cond_to,
};
use super::*;
use crate::calibration::api::engine;
use crate::calibration::api::schema::{
    CalibrationEnvelope, CalibrationPlan, CalibrationStep, HazardCurveParams, StepParams,
};
use crate::calibration::CalibrationMethod;
use crate::constants::{credit, time as time_constants, ONE_BASIS_POINT};
use crate::instruments::credit_derivatives::cds::{CreditDefaultSwap, PayReceive};
use crate::market::conventions::ids::{CdsConventionKey, CdsDocClause};
use crate::market::quotes::cds::CdsQuote;
use crate::market::quotes::ids::{Pillar, QuoteId};
use crate::market::quotes::market_quote::MarketQuote;
use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DateExt, DayCount, Tenor};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::{DiscountCurve, HazardCurve, Seniority};
use finstack_core::money::Money;
use finstack_core::types::CurveId;
use finstack_core::HashMap;
use rust_decimal::Decimal;
use time::macros::date;
use time::Duration;

fn create_test_cds(
    id: impl Into<String>,
    start_date: Date,
    end_date: Date,
    spread_bp: f64,
    recovery_rate: f64,
) -> CreditDefaultSwap {
    CreditDefaultSwap::new_isda(
        finstack_core::types::InstrumentId::new(id),
        Money::new(10_000_000.0, Currency::USD),
        PayReceive::PayFixed,
        crate::instruments::credit_derivatives::cds::CDSConvention::IsdaNa,
        Decimal::try_from(spread_bp).expect("valid spread_bp"),
        start_date,
        end_date,
        recovery_rate,
        finstack_core::types::CurveId::new("USD-OIS"),
        finstack_core::types::CurveId::new("TEST-CREDIT"),
    )
    .expect("test CDS creation should succeed")
}

fn create_test_curves() -> (DiscountCurve, HazardCurve) {
    let disc = DiscountCurve::builder("USD-OIS")
        .base_date(Date::from_calendar_date(2025, time::Month::January, 1).expect("valid date"))
        .knots(vec![(0.0, 1.0), (1.0, 0.95), (5.0, 0.80), (10.0, 0.65)])
        .build()
        .expect("should succeed");

    let credit = HazardCurve::builder("TEST-CREDIT")
        .base_date(Date::from_calendar_date(2025, time::Month::January, 1).expect("valid date"))
        .recovery_rate(0.40)
        .knots(vec![(1.0, 0.02), (3.0, 0.03), (5.0, 0.04), (10.0, 0.05)])
        .par_spreads(vec![
            (1.0, 100.0),
            (3.0, 150.0),
            (5.0, 200.0),
            (10.0, 250.0),
        ])
        .build()
        .expect("should succeed");

    (disc, credit)
}

#[test]
fn test_enhanced_protection_leg() {
    let (disc, credit) = create_test_curves();
    let as_of = Date::from_calendar_date(2025, time::Month::January, 1).expect("valid date");
    let cds = create_test_cds("TEST-CDS", as_of, as_of.add_months(60), 100.0, 0.40);
    let pricer = CDSPricer::new();
    let protection_pv = pricer
        .pv_protection_leg(&cds, &disc, &credit, as_of)
        .expect("should succeed");
    assert!(protection_pv.amount() > 0.0);
}

#[test]
fn test_accrual_on_default() {
    let (disc, credit) = create_test_curves();
    let as_of = Date::from_calendar_date(2025, time::Month::January, 1).expect("valid date");
    let cds = create_test_cds("TEST-CDS", as_of, as_of.add_months(60), 100.0, 0.40);
    let pricer_with = CDSPricer::new();
    let pricer_without = CDSPricer::with_config(CDSPricerConfig {
        include_accrual: false,
        integration_method: IntegrationMethod::Midpoint,
        ..Default::default()
    });
    let pv_with = pricer_with
        .pv_premium_leg(&cds, &disc, &credit, as_of)
        .expect("should succeed");
    let pv_without = pricer_without
        .pv_premium_leg(&cds, &disc, &credit, as_of)
        .expect("should succeed");
    assert!(pv_with.amount() > pv_without.amount());
}

#[test]
fn premium_leg_scales_linearly_with_notional_when_accrual_on_default_enabled() {
    let (disc, credit) = create_test_curves();
    let as_of = Date::from_calendar_date(2025, time::Month::February, 15).expect("valid date");
    let pricer = CDSPricer::new();

    let mut cds_unit = create_test_cds(
        "TEST-CDS-UNIT",
        date!(2024 - 12 - 20),
        date!(2028 - 03 - 20),
        100.0,
        0.40,
    );
    cds_unit.notional = Money::new(1.0, Currency::USD);

    let mut cds_large = cds_unit.clone();
    cds_large.id = finstack_core::types::InstrumentId::new("TEST-CDS-LARGE");
    cds_large.notional = Money::new(1_000_000.0, Currency::USD);

    let pv_unit = pricer
        .pv_premium_leg_raw(&cds_unit, &disc, &credit, as_of)
        .expect("unit notional premium leg");
    let pv_large = pricer
        .pv_premium_leg_raw(&cds_large, &disc, &credit, as_of)
        .expect("large notional premium leg");

    let scaled_unit = pv_unit * cds_large.notional.amount();
    assert!(
        (pv_large - scaled_unit).abs() < 1e-8,
        "premium leg PV should scale with notional, unit={pv_unit}, large={pv_large}, scaled_unit={scaled_unit}"
    );
}

#[test]
fn test_par_spread_calculation() {
    let (disc, credit) = create_test_curves();
    let as_of = Date::from_calendar_date(2025, time::Month::January, 1).expect("valid date");
    let cds = create_test_cds("TEST-CDS", as_of, as_of.add_months(60), 0.0, 0.40);
    let pricer = CDSPricer::new();
    let par_spread = pricer
        .par_spread(&cds, &disc, &credit, as_of)
        .expect("should succeed");
    assert!(par_spread > 0.0 && par_spread < 2000.0);
    let mut cds_at_par = cds.clone();
    cds_at_par.premium.spread_bp = Decimal::try_from(par_spread).expect("valid par_spread");
    let npv = pricer
        .npv(&cds_at_par, &disc, &credit, as_of)
        .expect("should succeed");
    // A CDS at par spread should have near-zero NPV. Tolerance of $5000
    // (~5bp on $10M) accounts for the accrual-on-default midpoint approximation
    // and discrete quarterly premium schedule vs. continuous protection leg.
    assert!(
        npv.amount().abs() < 5000.0,
        "CDS at par spread should have near-zero NPV, got {}",
        npv.amount()
    );
}

#[test]
fn test_settlement_delay_reduces_protection_pv() {
    let (disc, credit) = create_test_curves();
    let as_of = Date::from_calendar_date(2025, time::Month::January, 1).expect("valid date");
    let mut cds0 = create_test_cds("CDS-0D", as_of, as_of.add_months(60), 100.0, 0.40);
    let mut cds20 = cds0.clone();
    cds0.protection.settlement_delay = 0;
    cds20.protection.settlement_delay = 20;
    let pricer = CDSPricer::with_config(CDSPricerConfig {
        integration_method: IntegrationMethod::GaussianQuadrature,
        ..Default::default()
    });
    let pv0 = pricer
        .pv_protection_leg(&cds0, &disc, &credit, as_of)
        .expect("should succeed")
        .amount();
    let pv20 = pricer
        .pv_protection_leg(&cds20, &disc, &credit, as_of)
        .expect("should succeed")
        .amount();
    assert!(pv20 < pv0);
}

#[test]
fn test_isda_standard_model_ignores_step_tuning() {
    let as_of = Date::from_calendar_date(2025, time::Month::January, 1).expect("valid date");
    let cds = create_test_cds("CDS-STEP", as_of, as_of.add_months(120), 100.0, 0.40);

    let disc = DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .knots(vec![
            (0.0, 1.0),
            (0.25, 0.995),
            (0.5, 0.985),
            (1.0, 0.955),
            (3.0, 0.860),
            (7.0, 0.705),
            (10.0, 0.575),
        ])
        .build()
        .expect("should succeed");

    let credit = HazardCurve::builder("TEST-CREDIT")
        .base_date(as_of)
        .recovery_rate(0.40)
        .knots(vec![
            (0.25, 0.01),
            (0.5, 0.08),
            (1.0, 0.12),
            (3.0, 0.18),
            (7.0, 0.22),
            (10.0, 0.25),
        ])
        .build()
        .expect("should succeed");

    let coarse = CDSPricer::with_config(CDSPricerConfig {
        integration_method: IntegrationMethod::IsdaStandardModel,
        steps_per_year: 1,
        min_steps_per_year: 1,
        adaptive_steps: false,
        ..Default::default()
    });
    let fine = CDSPricer::with_config(CDSPricerConfig {
        integration_method: IntegrationMethod::IsdaStandardModel,
        steps_per_year: 3650,
        min_steps_per_year: 3650,
        adaptive_steps: false,
        ..Default::default()
    });

    let pv_coarse = coarse
        .pv_protection_leg_raw(&cds, &disc, &credit, as_of)
        .expect("coarse pricer should succeed");
    let pv_fine = fine
        .pv_protection_leg_raw(&cds, &disc, &credit, as_of)
        .expect("fine pricer should succeed");

    assert!(
        (pv_coarse - pv_fine).abs() < 1e-8,
        "ISDA standard model protection PV should not depend on step tuning; coarse={pv_coarse}, fine={pv_fine}",
    );
}

#[test]
fn test_legacy_bootstrapper_matches_canonical_hazard_target_conventions() {
    let base = Date::from_calendar_date(2025, time::Month::March, 20).expect("valid date");
    let currency = Currency::USD;
    let recovery_rate = 0.40;
    let cds_spreads = vec![(1.0, 100.0), (3.0, 150.0)];

    let disc = DiscountCurve::builder("TEST-DISC")
        .base_date(base)
        .knots(vec![
            (0.0, 1.0),
            (1.0, 0.98),
            (3.0, 0.94),
            (5.0, 0.88),
            (10.0, 0.75),
        ])
        .build()
        .expect("discount curve");

    let legacy_curve = CDSBootstrapper::new()
        .bootstrap_hazard_curve(&cds_spreads, recovery_rate, &disc, base)
        .expect("legacy bootstrap should succeed");

    let initial_market = MarketContext::new().insert(disc);
    let quotes = cds_spreads
        .iter()
        .map(|(tenor_years, spread_bp)| {
            MarketQuote::Cds(CdsQuote::CdsParSpread {
                id: QuoteId::new(format!("CDS-{tenor_years:.1}Y")),
                entity: "BOOTSTRAP-CONSISTENCY".to_string(),
                pillar: Pillar::Tenor(Tenor::from_years(*tenor_years, legacy_curve.day_count())),
                spread_bp: *spread_bp,
                recovery_rate,
                convention: CdsConventionKey {
                    currency,
                    doc_clause: CdsDocClause::IsdaNa,
                },
            })
        })
        .collect();

    let mut quote_sets: HashMap<String, Vec<MarketQuote>> = HashMap::default();
    quote_sets.insert("credit".to_string(), quotes);

    let hazard_id: CurveId = "BOOTSTRAP-CONSISTENCY-SENIOR".into();
    let plan = CalibrationPlan {
        id: "plan".to_string(),
        description: None,
        quote_sets,
        settings: Default::default(),
        steps: vec![CalibrationStep {
            id: "haz".to_string(),
            quote_set: "credit".to_string(),
            params: StepParams::Hazard(HazardCurveParams {
                curve_id: hazard_id.clone(),
                entity: "BOOTSTRAP-CONSISTENCY".to_string(),
                seniority: Seniority::Senior,
                currency,
                base_date: base,
                discount_curve_id: "TEST-DISC".into(),
                recovery_rate,
                notional: 1.0,
                method: CalibrationMethod::Bootstrap,
                interpolation: Default::default(),
                par_interp: finstack_core::market_data::term_structures::ParInterp::Linear,
                doc_clause: None,
            }),
        }],
    };

    let envelope = CalibrationEnvelope {
        schema: "finstack.calibration/2".to_string(),
        plan,
        initial_market: Some((&initial_market).into()),
    };

    let result = engine::execute(&envelope).expect("canonical execute");
    let canonical_ctx =
        MarketContext::try_from(result.result.final_market).expect("restore context");
    let canonical_curve = canonical_ctx
        .get_hazard(hazard_id.as_str())
        .expect("canonical hazard curve");

    assert_eq!(
        legacy_curve.day_count(),
        canonical_curve.day_count(),
        "legacy bootstrap should use the same day count as canonical hazard calibration"
    );

    let legacy_knots: Vec<f64> = legacy_curve.knot_points().map(|(t, _)| t).collect();
    let canonical_knots: Vec<f64> = canonical_curve.knot_points().map(|(t, _)| t).collect();
    assert_eq!(
        legacy_knots.len(),
        canonical_knots.len(),
        "legacy and canonical curves should have the same number of knots"
    );
    for (legacy_t, canonical_t) in legacy_knots.iter().zip(canonical_knots.iter()) {
        assert!(
            (legacy_t - canonical_t).abs() <= 1e-12,
            "legacy bootstrap should use canonical pillar times; legacy={legacy_t}, canonical={canonical_t}"
        );
    }
}

#[test]
fn test_par_spread_full_premium_option_runs() {
    let (disc, credit) = create_test_curves();
    let as_of = Date::from_calendar_date(2025, time::Month::January, 1).expect("valid date");
    let cds = create_test_cds("CDS-PAR", as_of, as_of.add_months(60), 0.0, 0.40);
    let pricer_ra = CDSPricer::new();
    let pricer_full = CDSPricer::with_config(CDSPricerConfig {
        par_spread_uses_full_premium: true,
        ..Default::default()
    });
    let s1 = pricer_ra
        .par_spread(&cds, &disc, &credit, as_of)
        .expect("should succeed");
    let s2 = pricer_full
        .par_spread(&cds, &disc, &credit, as_of)
        .expect("should succeed");
    assert!(s1.is_finite() && s2.is_finite());
}

// ─── Restructuring clause / doc_clause tests ───────────────────────

#[test]
fn test_xr14_regression_matches_baseline() {
    // Xr14 (no restructuring) should produce the same output as a CDS without
    // any explicit doc_clause, since the default convention (IsdaNa) maps to Xr14.
    let (disc, credit) = create_test_curves();
    let as_of = Date::from_calendar_date(2025, time::Month::January, 1).expect("valid date");

    let cds_baseline = create_test_cds("CDS-BASELINE", as_of, as_of.add_months(60), 100.0, 0.40);

    let mut cds_xr14 = create_test_cds("CDS-XR14", as_of, as_of.add_months(60), 100.0, 0.40);
    cds_xr14.doc_clause = Some(CdsDocClause::Xr14);

    let pricer = CDSPricer::new();

    let pv_baseline = pricer
        .pv_protection_leg_raw(&cds_baseline, &disc, &credit, as_of)
        .expect("should succeed");
    let pv_xr14 = pricer
        .pv_protection_leg_raw(&cds_xr14, &disc, &credit, as_of)
        .expect("should succeed");

    // Both should be identical since IsdaNa convention defaults to Xr14
    assert!(
        (pv_baseline - pv_xr14).abs() < 1e-10,
        "Xr14 should match baseline (IsdaNa default). Baseline={}, Xr14={}",
        pv_baseline,
        pv_xr14,
    );
}

#[test]
fn test_default_pricer_disables_restructuring_uplift() {
    let (disc, credit) = create_test_curves();
    let as_of = Date::from_calendar_date(2025, time::Month::January, 1).expect("valid date");

    let mut cds_xr14 = create_test_cds("CDS-XR14", as_of, as_of.add_months(60), 100.0, 0.40);
    cds_xr14.doc_clause = Some(CdsDocClause::Xr14);

    let mut cds_cr14 = create_test_cds("CDS-CR14", as_of, as_of.add_months(60), 100.0, 0.40);
    cds_cr14.doc_clause = Some(CdsDocClause::Cr14);

    let pricer = CDSPricer::new();

    let pv_xr14 = pricer
        .pv_protection_leg_raw(&cds_xr14, &disc, &credit, as_of)
        .expect("should succeed");
    let pv_cr14 = pricer
        .pv_protection_leg_raw(&cds_cr14, &disc, &credit, as_of)
        .expect("should succeed");

    assert!(
        (pv_cr14 - pv_xr14).abs() < 1e-10,
        "Default pricer should not apply restructuring uplift. Cr14={}, Xr14={}",
        pv_cr14,
        pv_xr14,
    );
}

#[test]
fn test_cr14_higher_protection_than_xr14_when_approximation_enabled() {
    let (disc, credit) = create_test_curves();
    let as_of = Date::from_calendar_date(2025, time::Month::January, 1).expect("valid date");

    let mut cds_xr14 = create_test_cds("CDS-XR14", as_of, as_of.add_months(60), 100.0, 0.40);
    cds_xr14.doc_clause = Some(CdsDocClause::Xr14);

    let mut cds_cr14 = create_test_cds("CDS-CR14", as_of, as_of.add_months(60), 100.0, 0.40);
    cds_cr14.doc_clause = Some(CdsDocClause::Cr14);

    let pricer = CDSPricer::with_config(CDSPricerConfig {
        enable_restructuring_approximation: true,
        ..Default::default()
    });

    let pv_xr14 = pricer
        .pv_protection_leg_raw(&cds_xr14, &disc, &credit, as_of)
        .expect("should succeed");
    let pv_cr14 = pricer
        .pv_protection_leg_raw(&cds_cr14, &disc, &credit, as_of)
        .expect("should succeed");

    assert!(
        pv_cr14 > pv_xr14,
        "Cr14 protection should exceed Xr14 when approximation is enabled. Cr14={}, Xr14={}",
        pv_cr14,
        pv_xr14,
    );
}

#[test]
fn test_restructuring_ordering_xr14_mr14_mm14_cr14() {
    // Protection PV should increase with broader restructuring coverage:
    // Xr14 <= Mr14 <= Mm14 <= Cr14
    let (disc, credit) = create_test_curves();
    let as_of = Date::from_calendar_date(2025, time::Month::January, 1).expect("valid date");

    let clauses = [
        CdsDocClause::Xr14,
        CdsDocClause::Mr14,
        CdsDocClause::Mm14,
        CdsDocClause::Cr14,
    ];

    let pricer = CDSPricer::with_config(CDSPricerConfig {
        enable_restructuring_approximation: true,
        ..Default::default()
    });
    let mut pvs = Vec::new();

    for clause in &clauses {
        let mut cds = create_test_cds("CDS-TEST", as_of, as_of.add_months(60), 100.0, 0.40);
        cds.doc_clause = Some(*clause);
        let pv = pricer
            .pv_protection_leg_raw(&cds, &disc, &credit, as_of)
            .expect("should succeed");
        pvs.push(pv);
    }

    for i in 0..pvs.len() - 1 {
        assert!(
            pvs[i] <= pvs[i + 1],
            "Protection PV should increase with broader restructuring: {:?}={} should be <= {:?}={}",
            clauses[i], pvs[i], clauses[i + 1], pvs[i + 1],
        );
    }
}

#[test]
fn test_doc_clause_effective_defaults() {
    let as_of = Date::from_calendar_date(2025, time::Month::January, 1).expect("valid date");

    // No explicit doc_clause with IsdaNa convention -> Xr14
    let cds_na = create_test_cds("CDS-NA", as_of, as_of.add_months(60), 100.0, 0.40);
    assert_eq!(cds_na.doc_clause_effective(), CdsDocClause::Xr14);

    // Explicit Cr14 should override convention default
    let mut cds_cr14 = create_test_cds("CDS-CR14", as_of, as_of.add_months(60), 100.0, 0.40);
    cds_cr14.doc_clause = Some(CdsDocClause::Cr14);
    assert_eq!(cds_cr14.doc_clause_effective(), CdsDocClause::Cr14);

    // Meta-clause IsdaEu should resolve to Mm14
    let mut cds_eu = create_test_cds("CDS-EU", as_of, as_of.add_months(60), 100.0, 0.40);
    cds_eu.doc_clause = Some(CdsDocClause::IsdaEu);
    assert_eq!(cds_eu.doc_clause_effective(), CdsDocClause::Mm14);
}

#[test]
fn test_max_deliverable_maturity_mapping() {
    assert_eq!(max_deliverable_maturity(CdsDocClause::Cr14), None);
    assert_eq!(max_deliverable_maturity(CdsDocClause::Mr14), Some(30));
    assert_eq!(max_deliverable_maturity(CdsDocClause::Mm14), Some(60));
    assert_eq!(max_deliverable_maturity(CdsDocClause::Xr14), Some(0));
    // Meta-clauses delegate
    assert_eq!(max_deliverable_maturity(CdsDocClause::IsdaNa), Some(0));
    assert_eq!(max_deliverable_maturity(CdsDocClause::IsdaEu), Some(60));
}

#[test]
fn test_doc_clause_serde_roundtrip() {
    let as_of = Date::from_calendar_date(2025, time::Month::January, 1).expect("valid date");

    // With doc_clause set
    let mut cds_with = create_test_cds("CDS-SERDE", as_of, as_of.add_months(60), 100.0, 0.40);
    cds_with.doc_clause = Some(CdsDocClause::Cr14);
    let json = serde_json::to_string(&cds_with).expect("serialize should succeed");
    assert!(
        json.contains("doc_clause"),
        "JSON should contain doc_clause field"
    );
    let deser: CreditDefaultSwap = serde_json::from_str(&json).expect("deserialize should succeed");
    assert_eq!(deser.doc_clause, Some(CdsDocClause::Cr14));

    // Without doc_clause (None) - should not appear in JSON (skip_serializing_if)
    let cds_without = create_test_cds("CDS-SERDE-NONE", as_of, as_of.add_months(60), 100.0, 0.40);
    let json_without = serde_json::to_string(&cds_without).expect("serialize should succeed");
    assert!(
        !json_without.contains("doc_clause"),
        "JSON should NOT contain doc_clause when None"
    );
    let deser_without: CreditDefaultSwap =
        serde_json::from_str(&json_without).expect("deserialize should succeed");
    assert_eq!(deser_without.doc_clause, None);
}

#[test]
fn test_doc_clause_default_when_omitted() {
    // Existing construction without doc_clause should still work
    let as_of = Date::from_calendar_date(2025, time::Month::January, 1).expect("valid date");
    let cds = create_test_cds("CDS-COMPAT", as_of, as_of.add_months(60), 100.0, 0.40);
    assert_eq!(cds.doc_clause, None);

    // Builder pattern should also work without doc_clause
    let cds_built = CreditDefaultSwap::builder()
        .id(finstack_core::types::InstrumentId::new("CDS-BUILDER"))
        .notional(Money::new(10_000_000.0, Currency::USD))
        .side(PayReceive::PayFixed)
        .convention(crate::instruments::credit_derivatives::cds::CDSConvention::IsdaNa)
        .premium(
            crate::instruments::common_impl::parameters::legs::PremiumLegSpec {
                start: as_of,
                end: as_of.add_months(60),
                frequency: finstack_core::dates::Tenor::quarterly(),
                stub: finstack_core::dates::StubKind::ShortFront,
                bdc: finstack_core::dates::BusinessDayConvention::ModifiedFollowing,
                calendar_id: Some("nyse".to_string()),
                day_count: finstack_core::dates::DayCount::Act360,
                spread_bp: Decimal::try_from(100.0).expect("valid"),
                discount_curve_id: finstack_core::types::CurveId::new("USD-OIS"),
            },
        )
        .protection(
            crate::instruments::common_impl::parameters::legs::ProtectionLegSpec {
                credit_curve_id: finstack_core::types::CurveId::new("TEST-CREDIT"),
                recovery_rate: 0.40,
                settlement_delay: 3,
            },
        )
        .build()
        .expect("builder should succeed without doc_clause");
    assert_eq!(cds_built.doc_clause, None);
    assert_eq!(cds_built.doc_clause_effective(), CdsDocClause::Xr14);
}

#[test]
fn test_doc_clause_serde_deserializes_without_field() {
    // Simulate old serialized data by serializing a CDS, stripping the
    // doc_clause field from JSON, and verifying it still deserializes.
    let as_of = Date::from_calendar_date(2025, time::Month::January, 1).expect("valid date");
    let cds = create_test_cds("CDS-OLD", as_of, as_of.add_months(60), 100.0, 0.40);
    let json = serde_json::to_string(&cds).expect("serialize should succeed");

    // The JSON should not contain "doc_clause" since it is None
    assert!(
        !json.contains("doc_clause"),
        "Baseline CDS JSON should not contain doc_clause"
    );

    // Deserialize and verify omitted fields use defaults
    let deser: CreditDefaultSwap =
        serde_json::from_str(&json).expect("Should deserialize old JSON without doc_clause field");
    assert_eq!(deser.doc_clause, None);
    assert_eq!(deser.doc_clause_effective(), CdsDocClause::Xr14);
}

#[test]
fn test_integration_method_recommendations_cover_boundaries() {
    assert_eq!(
        IntegrationMethod::recommended(0.5, false),
        IntegrationMethod::Midpoint
    );
    assert_eq!(
        IntegrationMethod::recommended(1.99, false),
        IntegrationMethod::Midpoint
    );
    assert_eq!(
        IntegrationMethod::recommended(2.0, false),
        IntegrationMethod::IsdaStandardModel
    );
    assert_eq!(
        IntegrationMethod::recommended(10.0, false),
        IntegrationMethod::IsdaStandardModel
    );
    assert_eq!(
        IntegrationMethod::recommended(10.01, false),
        IntegrationMethod::AdaptiveSimpson
    );
    assert_eq!(
        IntegrationMethod::recommended(0.25, true),
        IntegrationMethod::GaussianQuadrature
    );
    assert_eq!(
        IntegrationMethod::recommended(30.0, true),
        IntegrationMethod::GaussianQuadrature
    );
}

#[test]
fn test_pricer_config_factories_helpers_and_validation_paths() {
    let standard = CDSPricerConfig::isda_standard();
    assert_eq!(
        standard.integration_method,
        IntegrationMethod::IsdaStandardModel
    );
    assert!(standard.use_isda_coupon_dates);
    assert!(standard.adaptive_steps);
    assert_eq!(
        standard.business_days_per_year,
        time_constants::BUSINESS_DAYS_PER_YEAR_US
    );

    let europe = CDSPricerConfig::isda_europe();
    assert_eq!(
        europe.business_days_per_year,
        time_constants::BUSINESS_DAYS_PER_YEAR_UK
    );
    assert_eq!(europe.integration_method, standard.integration_method);

    let asia = CDSPricerConfig::isda_asia();
    assert_eq!(
        asia.business_days_per_year,
        time_constants::BUSINESS_DAYS_PER_YEAR_JP
    );
    assert_eq!(asia.integration_method, standard.integration_method);

    let simplified = CDSPricerConfig::simplified();
    assert_eq!(simplified.integration_method, IntegrationMethod::Midpoint);
    assert!(!simplified.use_isda_coupon_dates);
    assert!(!simplified.adaptive_steps);
    assert_eq!(simplified.validated_gl_order(), 4);

    let mut invalid_gl = standard.clone();
    invalid_gl.gl_order = 3;
    assert_eq!(invalid_gl.validated_gl_order(), 8);

    let mut adaptive = simplified.clone();
    adaptive.adaptive_steps = true;
    adaptive.min_steps_per_year = 20;
    assert_eq!(adaptive.effective_steps(1.1), 20);
    assert_eq!(adaptive.effective_steps(3.0), 36);
    assert_eq!(simplified.effective_steps(30.0), simplified.steps_per_year);

    let pricer = CDSPricer::try_with_config(standard.clone()).expect("valid config");
    assert_eq!(
        pricer.config().business_days_per_year,
        standard.business_days_per_year
    );

    let invalid_cases = {
        let mut cases = Vec::new();

        let mut cfg = standard.clone();
        cfg.tolerance = 0.0;
        cases.push((cfg, "tolerance"));

        let mut cfg = standard.clone();
        cfg.steps_per_year = 0;
        cases.push((cfg, "steps_per_year"));

        let mut cfg = standard.clone();
        cfg.min_steps_per_year = 0;
        cases.push((cfg, "min_steps_per_year"));

        let mut cfg = standard.clone();
        cfg.bootstrap_max_iterations = 0;
        cases.push((cfg, "bootstrap_max_iterations"));

        let mut cfg = standard.clone();
        cfg.bootstrap_tolerance = 0.0;
        cases.push((cfg, "bootstrap_tolerance"));

        let mut cfg = standard.clone();
        cfg.business_days_per_year = 0.0;
        cases.push((cfg, "business_days_per_year"));

        let mut cfg = standard.clone();
        cfg.adaptive_max_depth = 0;
        cases.push((cfg, "adaptive_max_depth"));

        cases
    };

    for (cfg, needle) in invalid_cases {
        let err = cfg.validate().expect_err("config should be rejected");
        assert!(
            err.to_string().contains(needle),
            "expected validation error mentioning {needle}, got {err}"
        );
    }

    let mut bad_for_pricer = standard.clone();
    bad_for_pricer.steps_per_year = 0;
    assert!(
        CDSPricer::try_with_config(bad_for_pricer).is_err(),
        "try_with_config should reject invalid settings"
    );
}

#[test]
fn test_max_deliverable_maturity_covers_remaining_meta_clauses_and_custom() {
    assert_eq!(max_deliverable_maturity(CdsDocClause::Custom), Some(0));
    assert_eq!(max_deliverable_maturity(CdsDocClause::IsdaAs), Some(0));
    assert_eq!(max_deliverable_maturity(CdsDocClause::IsdaAu), Some(0));
    assert_eq!(max_deliverable_maturity(CdsDocClause::IsdaNz), Some(0));
}

#[test]
fn test_bootstrap_hazard_curve_rejects_empty_quotes_and_handles_distressed_spreads() {
    let as_of = Date::from_calendar_date(2025, time::Month::January, 1).expect("valid date");
    let disc = DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .knots(vec![(0.0, 1.0), (1.0, 0.97), (3.0, 0.90), (5.0, 0.82)])
        .build()
        .expect("discount curve");

    let bootstrapper = CDSBootstrapper::new();
    let err = bootstrapper
        .bootstrap_hazard_curve(&[], 0.40, &disc, as_of)
        .expect_err("empty spread set should be rejected");
    assert!(matches!(
        err,
        finstack_core::Error::Input(finstack_core::InputError::TooFewPoints)
    ));

    let distressed = bootstrapper
        .bootstrap_hazard_curve(
            &[(1.0, 1_200.0), (3.0, 1_600.0), (5.0, 2_000.0)],
            0.25,
            &disc,
            as_of,
        )
        .expect("distressed spreads should still bootstrap");

    assert_eq!(distressed.base_date(), as_of);
    assert_eq!(distressed.recovery_rate(), 0.25);
    assert!(distressed.knot_points().count() >= 3);
}

#[test]
fn test_schedule_generation_respects_isda_flag_and_calendar_availability() {
    let start = Date::from_calendar_date(2025, time::Month::July, 1).expect("valid date");
    let end = Date::from_calendar_date(2026, time::Month::July, 1).expect("valid date");
    let cds = create_test_cds("CDS-SCHED", start, end, 100.0, 0.40);

    let simplified = CDSPricer::with_config(CDSPricerConfig::simplified());
    let regular_schedule = simplified
        .generate_schedule(&cds, start)
        .expect("regular schedule");
    let expected_regular = crate::cashflow::builder::build_dates(
        cds.premium.start,
        cds.premium.end,
        cds.premium.frequency,
        cds.premium.stub,
        cds.premium.bdc,
        false,
        0,
        cds.premium
            .calendar_id
            .as_deref()
            .unwrap_or(crate::cashflow::builder::calendar::WEEKENDS_ONLY_ID),
    )
    .expect("expected regular schedule")
    .dates;
    assert_eq!(regular_schedule, expected_regular);

    let isda = CDSPricer::new();
    let adjusted_schedule = isda
        .generate_isda_schedule(&cds)
        .expect("adjusted ISDA schedule");
    assert_ne!(
        regular_schedule, adjusted_schedule,
        "non-ISDA schedule should differ from IMM-style ISDA schedule"
    );

    let mut cds_no_calendar = cds.clone();
    cds_no_calendar.premium.calendar_id = None;
    let unadjusted_schedule = isda
        .generate_isda_schedule(&cds_no_calendar)
        .expect("unadjusted ISDA schedule");

    let sep_20 = Date::from_calendar_date(2025, time::Month::September, 20).expect("valid date");
    let sep_22 = Date::from_calendar_date(2025, time::Month::September, 22).expect("valid date");
    assert!(
        unadjusted_schedule.contains(&sep_20),
        "calendar-less ISDA schedule should keep weekend IMM dates"
    );
    assert!(
        adjusted_schedule.contains(&sep_22),
        "calendar-aware ISDA schedule should adjust weekend IMM dates"
    );
    assert!(
        !adjusted_schedule.contains(&sep_20),
        "calendar-aware ISDA schedule should not keep the unadjusted weekend date"
    );
}

#[test]
fn test_premium_leg_per_bp_matches_risky_annuity_without_accrual_on_default() {
    let (disc, credit) = create_test_curves();
    let as_of = Date::from_calendar_date(2025, time::Month::January, 1).expect("valid date");
    let cds = create_test_cds("CDS-PER-BP", as_of, as_of.add_months(60), 100.0, 0.40);

    let without_aod = CDSPricer::with_config(CDSPricerConfig {
        include_accrual: false,
        ..Default::default()
    });
    let risky_annuity = without_aod
        .risky_annuity(&cds, &disc, &credit, as_of)
        .expect("risky annuity");
    let per_bp_without_aod = without_aod
        .premium_leg_pv_per_bp(&cds, &disc, &credit, as_of)
        .expect("premium leg per bp");
    assert!(
        (per_bp_without_aod - risky_annuity * ONE_BASIS_POINT).abs() < 1e-14,
        "premium leg per bp without AoD should equal risky annuity × 1bp"
    );

    let with_aod = CDSPricer::new();
    let per_bp_with_aod = with_aod
        .premium_leg_pv_per_bp(&cds, &disc, &credit, as_of)
        .expect("premium leg per bp with AoD");
    assert!(
        per_bp_with_aod > per_bp_without_aod,
        "including AoD should increase premium leg PV per bp"
    );
}

#[test]
fn test_full_premium_par_spread_is_below_risky_annuity_par_spread() {
    let as_of = Date::from_calendar_date(2025, time::Month::January, 1).expect("valid date");
    let disc = DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .knots(vec![(0.0, 1.0), (1.0, 0.975), (5.0, 0.86), (10.0, 0.72)])
        .build()
        .expect("discount curve");
    let credit = HazardCurve::builder("TEST-CREDIT")
        .base_date(as_of)
        .recovery_rate(0.40)
        .knots(vec![(0.25, 0.08), (1.0, 0.12), (3.0, 0.16), (5.0, 0.20)])
        .build()
        .expect("hazard curve");
    let cds = create_test_cds("CDS-PAR-FULL", as_of, as_of.add_months(60), 100.0, 0.40);

    let isda = CDSPricer::new();
    let full_premium = CDSPricer::with_config(CDSPricerConfig {
        par_spread_uses_full_premium: true,
        ..Default::default()
    });

    let isda_spread = isda
        .par_spread(&cds, &disc, &credit, as_of)
        .expect("ISDA par spread");
    let full_spread = full_premium
        .par_spread(&cds, &disc, &credit, as_of)
        .expect("full-premium par spread");

    assert!(isda_spread.is_finite() && full_spread.is_finite());
    assert!(
        full_spread < isda_spread,
        "including AoD in the denominator should reduce the par spread"
    );
}

#[test]
fn test_npv_with_upfront_combines_dated_and_market_quote_adjustments() {
    let (disc, credit) = create_test_curves();
    let as_of = Date::from_calendar_date(2025, time::Month::January, 1).expect("valid date");
    let mut cds = create_test_cds("CDS-UPFRONT", as_of, as_of.add_months(60), 100.0, 0.40);
    let pricer = CDSPricer::new();

    let base_npv = pricer
        .npv(&cds, &disc, &credit, as_of)
        .expect("base npv")
        .amount();

    let dated_upfront_date = as_of.add_months(6);
    let dated_upfront_amount = 150_000.0;
    let quote_adjustment = Money::new(25_000.0, Currency::USD);
    cds.upfront = Some((
        dated_upfront_date,
        Money::new(dated_upfront_amount, Currency::USD),
    ));
    cds.pricing_overrides.market_quotes.upfront_payment = Some(quote_adjustment);

    let dated_df = disc
        .df_between_dates(as_of, dated_upfront_date)
        .expect("discount factor");
    let expected = base_npv - dated_upfront_amount * dated_df - quote_adjustment.amount();
    let npv_with_upfront = pricer
        .npv_with_upfront(&cds, &disc, &credit, as_of)
        .expect("npv with upfront")
        .amount();
    assert!(
        (npv_with_upfront - expected).abs() < 1e-8,
        "dated upfront and direct PV adjustment should combine additively"
    );

    let market = MarketContext::new()
        .insert(disc.clone())
        .insert(credit.clone());
    let npv_market = pricer
        .npv_market(&cds, &market, as_of)
        .expect("market npv")
        .amount();
    assert!(
        (npv_market - npv_with_upfront).abs() < 1e-12,
        "npv_market should match direct-curve npv_with_upfront"
    );
}

#[test]
fn test_time_and_settlement_helpers_match_curve_and_calendar_conventions() {
    let (disc, credit) = create_test_curves();
    let base_date = disc.base_date();
    let one_year = base_date.add_months(12);

    let expected_disc_t = disc
        .day_count()
        .year_fraction(
            base_date,
            one_year,
            finstack_core::dates::DayCountContext::default(),
        )
        .expect("discount year fraction");
    assert!(
        (disc_t(&disc, one_year).expect("disc_t") - expected_disc_t).abs() < 1e-12,
        "disc_t should respect the discount curve day-count"
    );

    let expected_haz_t = credit
        .day_count()
        .year_fraction(
            credit.base_date(),
            one_year,
            finstack_core::dates::DayCountContext::default(),
        )
        .expect("hazard year fraction");
    assert!(
        (haz_t(&credit, one_year).expect("haz_t") - expected_haz_t).abs() < 1e-12,
        "haz_t should respect the hazard curve day-count"
    );

    assert_eq!(
        date_from_hazard_time(&credit, -1.0),
        credit.base_date(),
        "negative hazard times should clamp to the curve base date"
    );
    let days_per_year: f64 = match credit.day_count() {
        DayCount::Act360 => 360.0,
        DayCount::Act365F => 365.0,
        DayCount::Act365L | DayCount::ActAct | DayCount::ActActIsma => 365.25,
        DayCount::Thirty360 | DayCount::ThirtyE360 => 360.0,
        DayCount::Bus252 => 252.0,
        _ => 365.25,
    };
    let hazard_time = 1.25_f64;
    let expected_date =
        credit.base_date() + Duration::days((hazard_time * days_per_year).round() as i64);
    assert_eq!(date_from_hazard_time(&credit, hazard_time), expected_date);

    let fallback_settlement = settlement_date(base_date, 3, None, 252.0).expect("fallback");
    assert_eq!(
        fallback_settlement,
        base_date + Duration::days(4),
        "3 business days at 252 bdays/year should round to 4 calendar days"
    );

    let nyse = finstack_core::dates::fx::resolve_calendar(Some("nyse")).expect("nyse calendar");
    let friday =
        Date::from_calendar_date(2025, time::Month::January, 3).expect("valid Friday date");
    let monday =
        Date::from_calendar_date(2025, time::Month::January, 6).expect("valid Monday date");
    assert_eq!(
        settlement_date(friday, 1, Some(nyse.as_holiday_calendar()), 252.0)
            .expect("calendar settlement"),
        monday,
        "calendar-aware settlement should advance by business days"
    );

    let midpoint = midpoint_default_date(&credit, base_date, one_year).expect("midpoint");
    let midpoint_time = 0.5
        * (haz_t(&credit, base_date).expect("haz_t start")
            + haz_t(&credit, one_year).expect("haz_t end"));
    assert_eq!(midpoint, date_from_hazard_time(&credit, midpoint_time));
}

#[test]
fn test_discount_survival_and_default_density_helpers_cover_boundary_cases() {
    let (disc, credit) = create_test_curves();
    let as_of = disc.base_date();
    let one_year = as_of.add_months(12);

    assert_eq!(
        df_asof_to(&disc, as_of, one_year).expect("df"),
        disc.df_between_dates(as_of, one_year)
            .expect("df between dates")
    );

    let t_asof = haz_t(&credit, as_of).expect("haz_t as_of");
    let t_one_year = haz_t(&credit, one_year).expect("haz_t future");
    let expected_conditional_survival = credit.sp(t_one_year) / credit.sp(t_asof);
    assert!(
        (sp_cond_to(&credit, as_of, one_year).expect("conditional survival")
            - expected_conditional_survival)
            .abs()
            < 1e-12
    );

    let mut late_as_of = as_of;
    while credit.sp(haz_t(&credit, late_as_of).expect("haz_t late"))
        > credit::SURVIVAL_PROBABILITY_FLOOR
    {
        late_as_of = late_as_of.add_months(600);
    }
    assert_eq!(
        sp_cond_to(&credit, late_as_of, late_as_of.add_months(12))
            .expect("conditional survival after effective default"),
        0.0,
        "conditional survival should floor to zero after effective default"
    );

    let t_start = 0.5;
    let t_end = 1.5;
    let h = 0.1;
    let center_density = approx_default_density(&credit, 1.0, h, t_start, t_end);
    let expected_center_density = -((credit.sp(1.0 + h) - credit.sp(1.0 - h)) / (2.0 * h));
    assert!(
        (center_density - expected_center_density.max(0.0)).abs() < 1e-12,
        "interior default density should use central differences"
    );
    assert!(approx_default_density(&credit, t_start, 0.0, t_start, t_end) >= 0.0);
    assert!(approx_default_density(&credit, t_end, 0.0, t_start, t_end) >= 0.0);
}

#[test]
fn test_restructuring_adjustment_factor_scales_with_clause_and_remaining_tenor() {
    let as_of = Date::from_calendar_date(2025, time::Month::January, 1).expect("valid date");
    let short_cds = create_test_cds("CDS-1Y", as_of, as_of.add_months(12), 100.0, 0.40);
    let long_cds = create_test_cds("CDS-10Y", as_of, as_of.add_months(120), 100.0, 0.40);

    assert_eq!(
        restructuring_adjustment_factor(CdsDocClause::Xr14, &short_cds),
        1.0
    );
    assert_eq!(
        restructuring_adjustment_factor(CdsDocClause::Custom, &short_cds),
        1.0
    );
    assert_eq!(
        restructuring_adjustment_factor(CdsDocClause::Mr14, &short_cds),
        1.02
    );
    assert_eq!(
        restructuring_adjustment_factor(CdsDocClause::Mm14, &short_cds),
        1.03
    );
    assert_eq!(
        restructuring_adjustment_factor(CdsDocClause::Cr14, &short_cds),
        1.05
    );

    let mr14_long = restructuring_adjustment_factor(CdsDocClause::Mr14, &long_cds);
    let mm14_long = restructuring_adjustment_factor(CdsDocClause::Mm14, &long_cds);
    let cr14_long = restructuring_adjustment_factor(CdsDocClause::Cr14, &long_cds);
    assert!(
        mr14_long > 1.0 && mr14_long < 1.02,
        "modified restructuring should be partially scaled for long tenors"
    );
    assert!(
        mm14_long > mr14_long && mm14_long < 1.03,
        "modified-modified restructuring should sit between MR14 and its full uplift"
    );
    assert_eq!(cr14_long, 1.05);
}

#[test]
fn test_bootstrap_convention_defaults_and_representative_keys_match_regions() {
    let default_convention = BootstrapConvention::default();
    assert!(default_convention.use_imm_dates);
    assert_eq!(
        default_convention.representative_convention_key(),
        CdsConventionKey {
            currency: Currency::USD,
            doc_clause: CdsDocClause::IsdaNa,
        }
    );

    let eu = BootstrapConvention {
        convention: crate::instruments::credit_derivatives::cds::CDSConvention::IsdaEu,
        use_imm_dates: true,
    };
    assert_eq!(
        eu.representative_convention_key(),
        CdsConventionKey {
            currency: Currency::EUR,
            doc_clause: CdsDocClause::IsdaEu,
        }
    );

    let asia = BootstrapConvention {
        convention: crate::instruments::credit_derivatives::cds::CDSConvention::IsdaAs,
        use_imm_dates: true,
    };
    assert_eq!(
        asia.representative_convention_key(),
        CdsConventionKey {
            currency: Currency::JPY,
            doc_clause: CdsDocClause::IsdaAs,
        }
    );

    let custom = BootstrapConvention {
        convention: crate::instruments::credit_derivatives::cds::CDSConvention::Custom,
        use_imm_dates: false,
    };
    assert_eq!(
        custom.representative_convention_key(),
        CdsConventionKey {
            currency: Currency::USD,
            doc_clause: CdsDocClause::Custom,
        }
    );

    let bootstrapper = CDSBootstrapper::default();
    assert_eq!(
        bootstrapper.config.integration_method,
        CDSPricerConfig::default().integration_method
    );
    assert_eq!(
        bootstrapper.convention.representative_convention_key(),
        default_convention.representative_convention_key()
    );
}

// ── Forward-starting CDS tests ──────────────────────────────────────

/// Helper: create a forward-starting CDS with a specified protection effective date.
fn create_forward_start_cds(
    id: impl Into<String>,
    start_date: Date,
    end_date: Date,
    spread_bp: f64,
    recovery_rate: f64,
    protection_effective_date: Option<Date>,
) -> CreditDefaultSwap {
    let mut cds = create_test_cds(id, start_date, end_date, spread_bp, recovery_rate);
    cds.protection_effective_date = protection_effective_date;
    cds.validate().expect("forward-start CDS should validate");
    cds
}

#[test]
fn test_forward_start_none_matches_spot_cds() {
    let (disc, credit) = create_test_curves();
    let as_of = Date::from_calendar_date(2025, time::Month::January, 1).expect("valid date");
    let end = as_of.add_months(60);

    let spot_cds = create_test_cds("CDS-SPOT", as_of, end, 100.0, 0.40);
    let fwd_none = create_forward_start_cds("CDS-FWD-NONE", as_of, end, 100.0, 0.40, None);

    let pricer = CDSPricer::new();

    let spot_prot = pricer
        .pv_protection_leg_raw(&spot_cds, &disc, &credit, as_of)
        .expect("should succeed");
    let fwd_prot = pricer
        .pv_protection_leg_raw(&fwd_none, &disc, &credit, as_of)
        .expect("should succeed");

    let spot_prem = pricer
        .pv_premium_leg_raw(&spot_cds, &disc, &credit, as_of)
        .expect("should succeed");
    let fwd_prem = pricer
        .pv_premium_leg_raw(&fwd_none, &disc, &credit, as_of)
        .expect("should succeed");

    assert!(
        (spot_prot - fwd_prot).abs() < 1e-10,
        "None protection_effective_date should match spot: spot={spot_prot}, fwd={fwd_prot}",
    );
    assert!(
        (spot_prem - fwd_prem).abs() < 1e-10,
        "None protection_effective_date should match spot premium: spot={spot_prem}, fwd={fwd_prem}",
    );
}

#[test]
fn test_forward_start_lower_protection_pv_same_premium_pv() {
    let (disc, credit) = create_test_curves();
    let as_of = Date::from_calendar_date(2025, time::Month::January, 1).expect("valid date");
    let end = as_of.add_months(60);
    let fwd_date = as_of.add_months(24);

    let spot_cds = create_test_cds("CDS-SPOT", as_of, end, 100.0, 0.40);
    let fwd_cds = create_forward_start_cds("CDS-FWD", as_of, end, 100.0, 0.40, Some(fwd_date));

    let pricer = CDSPricer::new();

    let spot_prot = pricer
        .pv_protection_leg_raw(&spot_cds, &disc, &credit, as_of)
        .expect("should succeed");
    let fwd_prot = pricer
        .pv_protection_leg_raw(&fwd_cds, &disc, &credit, as_of)
        .expect("should succeed");

    assert!(
        fwd_prot < spot_prot,
        "Forward-start protection PV ({fwd_prot}) should be less than spot ({spot_prot})",
    );
    assert!(
        fwd_prot > 0.0,
        "Forward-start protection PV should still be positive"
    );

    let spot_prem = pricer
        .pv_premium_leg_raw(&spot_cds, &disc, &credit, as_of)
        .expect("should succeed");
    let fwd_prem = pricer
        .pv_premium_leg_raw(&fwd_cds, &disc, &credit, as_of)
        .expect("should succeed");

    assert!(
        (spot_prem - fwd_prem).abs() < 1e-10,
        "Premium leg should be identical: spot={spot_prem}, fwd={fwd_prem}",
    );
}

#[test]
fn test_forward_start_protection_at_end_near_zero() {
    let (disc, credit) = create_test_curves();
    let as_of = Date::from_calendar_date(2025, time::Month::January, 1).expect("valid date");
    let end = as_of.add_months(60);

    let fwd_cds = create_forward_start_cds("CDS-FWD-END", as_of, end, 100.0, 0.40, Some(end));

    let pricer = CDSPricer::new();
    let prot_pv = pricer
        .pv_protection_leg_raw(&fwd_cds, &disc, &credit, as_of)
        .expect("should succeed");

    assert!(
        prot_pv.abs() < 1.0,
        "Protection PV should be ~0 when effective_date = end, got {prot_pv}",
    );
}

#[test]
fn test_forward_start_invalid_before_premium_start() {
    let as_of = Date::from_calendar_date(2025, time::Month::January, 1).expect("valid date");
    let end = as_of.add_months(60);
    let before_start = Date::from_calendar_date(2024, time::Month::June, 1).expect("valid date");

    let mut cds = create_test_cds("CDS-BAD", as_of, end, 100.0, 0.40);
    cds.protection_effective_date = Some(before_start);
    let result = cds.validate();
    assert!(
        result.is_err(),
        "protection_effective_date before premium start should fail"
    );
}

#[test]
fn test_forward_start_invalid_after_premium_end() {
    let as_of = Date::from_calendar_date(2025, time::Month::January, 1).expect("valid date");
    let end = as_of.add_months(60);
    let after_end = end.add_months(12);

    let mut cds = create_test_cds("CDS-BAD", as_of, end, 100.0, 0.40);
    cds.protection_effective_date = Some(after_end);
    let result = cds.validate();
    assert!(
        result.is_err(),
        "protection_effective_date after premium end should fail"
    );
}

#[test]
fn test_protection_start_helper() {
    let as_of = Date::from_calendar_date(2025, time::Month::January, 1).expect("valid date");
    let end = as_of.add_months(60);
    let fwd_date = as_of.add_months(24);

    let spot = create_test_cds("CDS-SPOT", as_of, end, 100.0, 0.40);
    assert_eq!(spot.protection_start(), as_of);

    let mut fwd = spot.clone();
    fwd.protection_effective_date = Some(fwd_date);
    assert_eq!(fwd.protection_start(), fwd_date);
}
