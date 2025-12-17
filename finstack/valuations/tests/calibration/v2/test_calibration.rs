//! Determinism and smoke tests for calibration v2.

use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Tenor};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::Seniority;
use finstack_core::math::interp::ExtrapolationPolicy;
use finstack_core::prelude::DateExt;
use finstack_core::types::{Currency, CurveId};
use finstack_valuations::calibration::v2::api::engine;
use finstack_valuations::calibration::v2::api::schema::{
    CalibrationEnvelopeV2, CalibrationMethod, CalibrationPlanV2, CalibrationStepV2,
    DiscountCurveParams, HazardCurveParams, StepParams,
};
use finstack_valuations::calibration::v2::domain::quotes::{
    CreditQuote, InstrumentConventions, MarketQuote, RatesQuote,
};
use std::collections::HashMap;
use time::Month;

fn build_discount_quotes(base_date: Date) -> Vec<MarketQuote> {
    vec![
        MarketQuote::Rates(RatesQuote::Deposit {
            maturity: base_date + time::Duration::days(1),
            rate: 0.05,
            conventions: InstrumentConventions::default()
                .with_day_count(DayCount::Act360)
                .with_settlement_days(0),
        }),
        MarketQuote::Rates(RatesQuote::Swap {
            maturity: base_date.add_months(12),
            rate: 0.051,
            is_ois: true,
            conventions: InstrumentConventions::default(),
            fixed_leg_conventions: InstrumentConventions::default()
                .with_payment_frequency(Tenor::annual())
                .with_day_count(DayCount::Act360),
            float_leg_conventions: InstrumentConventions::default()
                .with_payment_frequency(Tenor::annual())
                .with_day_count(DayCount::Act360)
                .with_index("USD-OIS"),
        }),
        MarketQuote::Rates(RatesQuote::Swap {
            maturity: base_date.add_months(24),
            rate: 0.052,
            is_ois: true,
            conventions: InstrumentConventions::default(),
            fixed_leg_conventions: InstrumentConventions::default()
                .with_payment_frequency(Tenor::annual())
                .with_day_count(DayCount::Act360),
            float_leg_conventions: InstrumentConventions::default()
                .with_payment_frequency(Tenor::annual())
                .with_day_count(DayCount::Act360)
                .with_index("USD-OIS"),
        }),
    ]
}

fn build_credit_quotes() -> Vec<MarketQuote> {
    let conventions = InstrumentConventions::default()
        .with_day_count(DayCount::Act360)
        .with_payment_frequency(Tenor::quarterly())
        .with_settlement_days(0)
        .with_calendar_id("usny")
        .with_business_day_convention(BusinessDayConvention::ModifiedFollowing);

    vec![
        MarketQuote::Credit(CreditQuote::CDS {
            entity: "TEST-ENTITY".to_string(),
            maturity: Date::from_calendar_date(2026, Month::March, 20).unwrap(),
            spread_bp: 100.0,
            recovery_rate: 0.40,
            currency: Currency::USD,
            conventions: conventions.clone(),
        }),
        MarketQuote::Credit(CreditQuote::CDS {
            entity: "TEST-ENTITY".to_string(),
            maturity: Date::from_calendar_date(2028, Month::March, 20).unwrap(),
            spread_bp: 150.0,
            recovery_rate: 0.40,
            currency: Currency::USD,
            conventions: conventions.clone(),
        }),
        MarketQuote::Credit(CreditQuote::CDS {
            entity: "TEST-ENTITY".to_string(),
            maturity: Date::from_calendar_date(2030, Month::March, 20).unwrap(),
            spread_bp: 200.0,
            recovery_rate: 0.40,
            currency: Currency::USD,
            conventions,
        }),
    ]
}

#[test]
fn hazard_curve_calibration_is_deterministic_across_runs() {
    // Use an IMM-style base date (20th) to align with canonical CDS construction.
    let base_date = Date::from_calendar_date(2025, Month::March, 20).unwrap();
    let currency = Currency::USD;

    let mut quote_sets: HashMap<String, Vec<MarketQuote>> = HashMap::new();
    quote_sets.insert("disc".to_string(), build_discount_quotes(base_date));
    quote_sets.insert("credit".to_string(), build_credit_quotes());

    let hazard_id: CurveId = "TEST-ENTITY-SENIOR".into();

    let plan = CalibrationPlanV2 {
        id: "plan".to_string(),
        description: None,
        quote_sets,
        settings: Default::default(),
        steps: vec![
            CalibrationStepV2 {
                id: "disc".to_string(),
                quote_set: "disc".to_string(),
                params: StepParams::Discount(DiscountCurveParams {
                    curve_id: "USD-OIS".into(),
                    currency,
                    base_date,
                    method: CalibrationMethod::Bootstrap,
                    interpolation: Default::default(),
                    extrapolation: ExtrapolationPolicy::FlatForward,
                    pricing_discount_id: None,
                    pricing_forward_id: None,
                    conventions: Default::default(),
                }),
            },
            CalibrationStepV2 {
                id: "haz".to_string(),
                quote_set: "credit".to_string(),
                params: StepParams::Hazard(HazardCurveParams {
                    curve_id: hazard_id.clone(),
                    entity: "TEST-ENTITY".to_string(),
                    seniority: Seniority::Senior,
                    currency,
                    base_date,
                    discount_curve_id: "USD-OIS".into(),
                    recovery_rate: 0.40,
                    notional: 1.0,
                    method: CalibrationMethod::Bootstrap,
                    interpolation: Default::default(),
                }),
            },
        ],
    };

    let envelope = CalibrationEnvelopeV2 {
        schema: "finstack.calibration/2".to_string(),
        plan,
        initial_market: Some((&MarketContext::new()).into()),
    };

    let mut first: Option<Vec<(f64, f64)>> = None;
    for _ in 0..20 {
        let result = engine::execute(&envelope).expect("execute");
        let ctx = MarketContext::try_from(result.result.final_market).expect("restore context");
        let curve = ctx.get_hazard(hazard_id.as_str()).expect("hazard curve");

        let knots: Vec<(f64, f64)> = curve.knot_points().collect();
        match &first {
            None => first = Some(knots),
            Some(k0) => assert_eq!(knots, *k0, "hazard knots should be identical across runs"),
        }
    }
}

#[test]
fn discount_curve_bootstrap_is_order_independent() {
    let base_date = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let currency = Currency::USD;

    let quotes_sorted = build_discount_quotes(base_date);
    let mut quotes_shuffled = quotes_sorted.clone();
    quotes_shuffled.reverse();

    let plan_for = |quote_set: Vec<MarketQuote>| CalibrationEnvelopeV2 {
        schema: "finstack.calibration/2".to_string(),
        plan: CalibrationPlanV2 {
            id: "plan".to_string(),
            description: None,
            quote_sets: {
                let mut q = HashMap::new();
                q.insert("disc".to_string(), quote_set);
                q
            },
            settings: finstack_valuations::calibration::CalibrationConfig {
                tolerance: 1e-12,
                max_iterations: 200,
                ..Default::default()
            },
            steps: vec![CalibrationStepV2 {
                id: "disc".to_string(),
                quote_set: "disc".to_string(),
                params: StepParams::Discount(DiscountCurveParams {
                    curve_id: "USD-OIS".into(),
                    currency,
                    base_date,
                    method: CalibrationMethod::Bootstrap,
                    interpolation: Default::default(),
                    extrapolation: ExtrapolationPolicy::FlatForward,
                    pricing_discount_id: None,
                    pricing_forward_id: None,
                    conventions: Default::default(),
                }),
            }],
        },
        initial_market: Some((&MarketContext::new()).into()),
    };

    let out_sorted = engine::execute(&plan_for(quotes_sorted)).expect("execute (sorted)");
    let out_shuffled = engine::execute(&plan_for(quotes_shuffled)).expect("execute (shuffled)");

    let ctx_sorted = MarketContext::try_from(out_sorted.result.final_market).expect("restore ctx");
    let ctx_shuffled =
        MarketContext::try_from(out_shuffled.result.final_market).expect("restore ctx");

    let curve_sorted = ctx_sorted.get_discount("USD-OIS").expect("discount curve");
    let curve_shuffled = ctx_shuffled.get_discount("USD-OIS").expect("discount curve");

    assert_eq!(
        curve_sorted
            .knots()
            .iter()
            .copied()
            .zip(curve_sorted.dfs().iter().copied())
            .collect::<Vec<_>>(),
        curve_shuffled
            .knots()
            .iter()
            .copied()
            .zip(curve_shuffled.dfs().iter().copied())
            .collect::<Vec<_>>(),
        "discount curve knots should be identical under quote shuffling"
    );

    let rep_sorted = out_sorted
        .result
        .step_reports
        .get("disc")
        .expect("step report");
    let rep_shuffled = out_shuffled
        .result
        .step_reports
        .get("disc")
        .expect("step report");
    assert_eq!(
        rep_sorted.residuals, rep_shuffled.residuals,
        "residuals should be identical under quote shuffling"
    );
}

#[test]
fn discount_curve_global_solve_smoke_v2() {
    let base_date = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let currency = Currency::USD;

    let quotes = vec![
        MarketQuote::Rates(RatesQuote::Deposit {
            maturity: Date::from_calendar_date(2025, Month::July, 15).unwrap(),
            rate: 0.03,
            conventions: InstrumentConventions::default()
                .with_day_count(DayCount::Act360)
                .with_settlement_days(0),
        }),
        MarketQuote::Rates(RatesQuote::Deposit {
            maturity: Date::from_calendar_date(2026, Month::January, 15).unwrap(),
            rate: 0.031,
            conventions: InstrumentConventions::default()
                .with_day_count(DayCount::Act360)
                .with_settlement_days(0),
        }),
        MarketQuote::Rates(RatesQuote::Deposit {
            maturity: Date::from_calendar_date(2026, Month::July, 15).unwrap(),
            rate: 0.0315,
            conventions: InstrumentConventions::default()
                .with_day_count(DayCount::Act360)
                .with_settlement_days(0),
        }),
    ];

    let mut quote_sets: HashMap<String, Vec<MarketQuote>> = HashMap::new();
    quote_sets.insert("ois".to_string(), quotes);

    let plan = CalibrationPlanV2 {
        id: "plan".to_string(),
        description: None,
        quote_sets,
        settings: finstack_valuations::calibration::CalibrationConfig {
            tolerance: 1e-10,
            max_iterations: 200,
            ..Default::default()
        },
        steps: vec![CalibrationStepV2 {
            id: "disc".to_string(),
            quote_set: "ois".to_string(),
            params: StepParams::Discount(DiscountCurveParams {
                curve_id: "USD-OIS".into(),
                currency,
                base_date,
                method: CalibrationMethod::Global,
                interpolation: Default::default(),
                extrapolation: ExtrapolationPolicy::FlatForward,
                pricing_discount_id: None,
                pricing_forward_id: None,
                conventions: Default::default(),
            }),
        }],
    };

    let envelope = CalibrationEnvelopeV2 {
        schema: "finstack.calibration/2".to_string(),
        plan,
        initial_market: None,
    };

    let result = engine::execute(&envelope).expect("execute");
    assert!(result.result.report.success, "global fit should succeed for deposits");

    let ctx = MarketContext::try_from(result.result.final_market).expect("restore context");
    let disc = ctx.get_discount("USD-OIS").expect("discount curve");
    assert!(disc.knots().len() >= 2, "expected at least anchor + 1 pillar");
}
