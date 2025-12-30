//! Hazard curve calibration tests (v2).

use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::market_data::term_structures::Seniority;
use finstack_core::math::interp::InterpStyle;
use finstack_core::types::{Currency, CurveId};
use finstack_core::HashMap;
use finstack_valuations::calibration::api::engine;
use finstack_valuations::calibration::api::schema::{
    CalibrationConfig, CalibrationEnvelope, CalibrationMethod, CalibrationPlan, CalibrationStep,
    HazardCurveParams, StepParams,
};
use finstack_valuations::calibration::ResidualWeightingScheme;
use finstack_valuations::market::conventions::ids::{CdsConventionKey, CdsDocClause};
use finstack_valuations::market::quotes::cds::CdsQuote;
use finstack_valuations::market::quotes::ids::{Pillar, QuoteId};
use finstack_valuations::market::quotes::market_quote::MarketQuote;
use time::Month;

fn create_test_discount_curve(base: Date) -> DiscountCurve {
    DiscountCurve::builder("TEST-DISC")
        .base_date(base)
        .knots(vec![
            (0.0, 1.0),
            (1.0, 0.98),
            (3.0, 0.94),
            (5.0, 0.88),
            (10.0, 0.75),
        ])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap()
}

fn hazard_total_variation(curve: &finstack_core::market_data::term_structures::HazardCurve) -> f64 {
    let mut total = 0.0;
    let mut prev: Option<f64> = None;
    for (_t, lambda) in curve.knot_points() {
        if let Some(last) = prev {
            total += (lambda - last).abs();
        }
        prev = Some(lambda);
    }
    total
}

#[test]
fn hazard_calibration_positive_rates() {
    // Use ISDA-friendly dates (IMM 20th) because v2 hazard bootstrapping builds
    // canonical CDS instruments under ISDA conventions.
    let base = Date::from_calendar_date(2025, Month::March, 20).unwrap();
    let currency = Currency::USD;

    let disc = create_test_discount_curve(base);
    let initial_market = MarketContext::new().insert_discount(disc);

    let quotes = vec![
        MarketQuote::Cds(CdsQuote::CdsParSpread {
            id: QuoteId::new(format!(
                "CDS-{:?}",
                Date::from_calendar_date(2026, Month::March, 20).unwrap()
            )),
            entity: "ACME-Corp".to_string(),
            pillar: Pillar::Date(Date::from_calendar_date(2026, Month::March, 20).unwrap()),
            spread_bp: 100.0,
            recovery_rate: 0.40,
            convention: CdsConventionKey {
                currency,
                doc_clause: CdsDocClause::IsdaNa,
            },
        }),
        MarketQuote::Cds(CdsQuote::CdsParSpread {
            id: QuoteId::new(format!(
                "CDS-{:?}",
                Date::from_calendar_date(2028, Month::March, 20).unwrap()
            )),
            entity: "ACME-Corp".to_string(),
            pillar: Pillar::Date(Date::from_calendar_date(2028, Month::March, 20).unwrap()),
            spread_bp: 150.0,
            recovery_rate: 0.40,
            convention: CdsConventionKey {
                currency,
                doc_clause: CdsDocClause::IsdaNa,
            },
        }),
        MarketQuote::Cds(CdsQuote::CdsParSpread {
            id: QuoteId::new(format!(
                "CDS-{:?}",
                Date::from_calendar_date(2030, Month::March, 20).unwrap()
            )),
            entity: "ACME-Corp".to_string(),
            pillar: Pillar::Date(Date::from_calendar_date(2030, Month::March, 20).unwrap()),
            spread_bp: 200.0,
            recovery_rate: 0.40,
            convention: CdsConventionKey {
                currency,
                doc_clause: CdsDocClause::IsdaNa,
            },
        }),
    ];

    let mut quote_sets: HashMap<String, Vec<MarketQuote>> = HashMap::default();
    quote_sets.insert("credit".to_string(), quotes);

    let hazard_id: CurveId = "ACME-Corp-SENIOR".into();

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
                entity: "ACME-Corp".to_string(),
                seniority: Seniority::Senior,
                currency,
                base_date: base,
                discount_curve_id: "TEST-DISC".into(),
                recovery_rate: 0.40,
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

    let result = engine::execute(&envelope).expect("execute");
    assert!(result.result.report.success);

    let ctx = MarketContext::try_from(result.result.final_market).expect("restore context");
    let curve = ctx.get_hazard(hazard_id.as_str()).expect("hazard curve");

    for (_t, lambda) in curve.knot_points() {
        assert!(lambda > 0.0, "hazard rate should be positive, got {lambda}");
    }
}

#[test]
fn hazard_calibration_rejects_zero_spread() {
    let base = Date::from_calendar_date(2025, Month::March, 20).unwrap();
    let currency = Currency::USD;

    let disc = create_test_discount_curve(base);
    let initial_market = MarketContext::new().insert_discount(disc);

    let quotes = vec![MarketQuote::Cds(CdsQuote::CdsParSpread {
        id: QuoteId::new(format!(
            "CDS-{:?}",
            Date::from_calendar_date(2026, Month::March, 20).unwrap()
        )),
        entity: "ZERO-SPREAD".to_string(),
        pillar: Pillar::Date(Date::from_calendar_date(2026, Month::March, 20).unwrap()),
        spread_bp: 0.0,
        recovery_rate: 0.40,
        convention: CdsConventionKey {
            currency,
            doc_clause: CdsDocClause::IsdaNa,
        },
    })];

    let mut quote_sets: HashMap<String, Vec<MarketQuote>> = HashMap::default();
    quote_sets.insert("credit".to_string(), quotes);

    let plan = CalibrationPlan {
        id: "plan".to_string(),
        description: None,
        quote_sets,
        settings: Default::default(),
        steps: vec![CalibrationStep {
            id: "haz".to_string(),
            quote_set: "credit".to_string(),
            params: StepParams::Hazard(HazardCurveParams {
                curve_id: "ZERO-SPREAD-SENIOR".into(),
                entity: "ZERO-SPREAD".to_string(),
                seniority: Seniority::Senior,
                currency,
                base_date: base,
                discount_curve_id: "TEST-DISC".into(),
                recovery_rate: 0.40,
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

    let err = engine::execute(&envelope).expect_err("zero spread should be invalid");
    assert!(matches!(
        err,
        finstack_core::Error::Validation(_)
            | finstack_core::Error::Input(_)
            | finstack_core::Error::Calibration { .. }
    ));
}

#[test]
fn hazard_calibration_global_solve_sqrt_time_is_not_rougher_than_bootstrap() {
    let base = Date::from_calendar_date(2025, Month::March, 20).unwrap();
    let currency = Currency::USD;

    let disc = create_test_discount_curve(base);
    let initial_market = MarketContext::new().insert_discount(disc);

    let quotes = vec![
        MarketQuote::Cds(CdsQuote::CdsParSpread {
            id: QuoteId::new(format!(
                "CDS-{:?}",
                Date::from_calendar_date(2026, Month::March, 20).unwrap()
            )),
            entity: "ACME-Corp".to_string(),
            pillar: Pillar::Date(Date::from_calendar_date(2026, Month::March, 20).unwrap()),
            spread_bp: 110.0,
            recovery_rate: 0.40,
            convention: CdsConventionKey {
                currency,
                doc_clause: CdsDocClause::IsdaNa,
            },
        }),
        MarketQuote::Cds(CdsQuote::CdsParSpread {
            id: QuoteId::new(format!(
                "CDS-{:?}",
                Date::from_calendar_date(2028, Month::March, 20).unwrap()
            )),
            entity: "ACME-Corp".to_string(),
            pillar: Pillar::Date(Date::from_calendar_date(2028, Month::March, 20).unwrap()),
            spread_bp: 170.0,
            recovery_rate: 0.40,
            convention: CdsConventionKey {
                currency,
                doc_clause: CdsDocClause::IsdaNa,
            },
        }),
        MarketQuote::Cds(CdsQuote::CdsParSpread {
            id: QuoteId::new(format!(
                "CDS-{:?}",
                Date::from_calendar_date(2030, Month::March, 20).unwrap()
            )),
            entity: "ACME-Corp".to_string(),
            pillar: Pillar::Date(Date::from_calendar_date(2030, Month::March, 20).unwrap()),
            spread_bp: 210.0,
            recovery_rate: 0.40,
            convention: CdsConventionKey {
                currency,
                doc_clause: CdsDocClause::IsdaNa,
            },
        }),
        MarketQuote::Cds(CdsQuote::CdsParSpread {
            id: QuoteId::new(format!(
                "CDS-{:?}",
                Date::from_calendar_date(2032, Month::March, 20).unwrap()
            )),
            entity: "ACME-Corp".to_string(),
            pillar: Pillar::Date(Date::from_calendar_date(2032, Month::March, 20).unwrap()),
            spread_bp: 190.0,
            recovery_rate: 0.40,
            convention: CdsConventionKey {
                currency,
                doc_clause: CdsDocClause::IsdaNa,
            },
        }),
    ];

    let mut quote_sets: HashMap<String, Vec<MarketQuote>> = HashMap::default();
    quote_sets.insert("credit".to_string(), quotes.clone());

    let hazard_id_boot: CurveId = "ACME-Corp-BOOT".into();
    let hazard_id_global: CurveId = "ACME-Corp-GLOBAL".into();

    let bootstrap_plan = CalibrationPlan {
        id: "plan-bootstrap".to_string(),
        description: None,
        quote_sets: quote_sets.clone(),
        settings: CalibrationConfig::default(),
        steps: vec![CalibrationStep {
            id: "haz".to_string(),
            quote_set: "credit".to_string(),
            params: StepParams::Hazard(HazardCurveParams {
                curve_id: hazard_id_boot.clone(),
                entity: "ACME-Corp".to_string(),
                seniority: Seniority::Senior,
                currency,
                base_date: base,
                discount_curve_id: "TEST-DISC".into(),
                recovery_rate: 0.40,
                notional: 1.0,
                method: CalibrationMethod::Bootstrap,
                interpolation: Default::default(),
                par_interp: finstack_core::market_data::term_structures::ParInterp::Linear,
                doc_clause: None,
            }),
        }],
    };

    let bootstrap_env = CalibrationEnvelope {
        schema: "finstack.calibration/2".to_string(),
        plan: bootstrap_plan,
        initial_market: Some((&initial_market).into()),
    };

    let bootstrap_result = engine::execute(&bootstrap_env).expect("bootstrap execute");
    let bootstrap_report = bootstrap_result
        .result
        .step_reports
        .get("haz")
        .expect("bootstrap report");
    assert!(bootstrap_report.success);

    let bootstrap_ctx =
        MarketContext::try_from(bootstrap_result.result.final_market).expect("restore context");
    let bootstrap_curve = bootstrap_ctx
        .get_hazard(hazard_id_boot.as_str())
        .expect("bootstrap curve");

    let mut global_settings = CalibrationConfig::default();
    global_settings.discount_curve.weighting_scheme = ResidualWeightingScheme::SqrtTime;
    global_settings.calibration_method = CalibrationMethod::GlobalSolve {
        use_analytical_jacobian: false,
    };

    let global_plan = CalibrationPlan {
        id: "plan-global".to_string(),
        description: None,
        quote_sets,
        settings: global_settings.clone(),
        steps: vec![CalibrationStep {
            id: "haz".to_string(),
            quote_set: "credit".to_string(),
            params: StepParams::Hazard(HazardCurveParams {
                curve_id: hazard_id_global.clone(),
                entity: "ACME-Corp".to_string(),
                seniority: Seniority::Senior,
                currency,
                base_date: base,
                discount_curve_id: "TEST-DISC".into(),
                recovery_rate: 0.40,
                notional: 1.0,
                method: CalibrationMethod::GlobalSolve {
                    use_analytical_jacobian: false,
                },
                interpolation: Default::default(),
                par_interp: finstack_core::market_data::term_structures::ParInterp::Linear,
                doc_clause: None,
            }),
        }],
    };

    let global_env = CalibrationEnvelope {
        schema: "finstack.calibration/2".to_string(),
        plan: global_plan,
        initial_market: Some((&initial_market).into()),
    };

    let global_result = engine::execute(&global_env).expect("global execute");
    let global_report = global_result
        .result
        .step_reports
        .get("haz")
        .expect("global report");
    assert!(global_report.success);
    assert!(
        global_report.max_residual <= global_settings.discount_curve.validation_tolerance,
        "max_residual {} exceeds tolerance {}",
        global_report.max_residual,
        global_settings.discount_curve.validation_tolerance
    );

    let global_ctx =
        MarketContext::try_from(global_result.result.final_market).expect("restore context");
    let global_curve = global_ctx
        .get_hazard(hazard_id_global.as_str())
        .expect("global curve");

    let bootstrap_tv = hazard_total_variation(&bootstrap_curve);
    let global_tv = hazard_total_variation(&global_curve);

    assert!(
        global_tv <= bootstrap_tv + 1e-6,
        "expected global solve to be no rougher (global {:.6e}, bootstrap {:.6e})",
        global_tv,
        bootstrap_tv
    );
}
