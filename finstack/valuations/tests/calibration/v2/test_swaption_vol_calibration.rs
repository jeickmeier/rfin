//! Integration test for swaption volatility calibration (v2).

use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::types::{Currency, CurveId};
use finstack_valuations::calibration::api::engine;
use finstack_valuations::calibration::api::schema::{
    CalibrationEnvelopeV2, CalibrationPlanV2, CalibrationStepV2, StepParams,
    SurfaceExtrapolationPolicy, SwaptionVolConvention, SwaptionVolParams,
};
use finstack_valuations::calibration::quotes::{MarketQuote, VolQuote};
use finstack_valuations::calibration::CalibrationConfig;
use std::collections::HashMap;
use time::Month;

fn create_test_discount_curve(base_date: Date) -> DiscountCurve {
    DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots(vec![
            (0.0, 1.0),   // Today
            (0.25, 0.99), // 3M: ~4% rate
            (1.0, 0.96),  // 1Y: ~4% rate
            (2.0, 0.92),  // 2Y: ~4% rate
            (5.0, 0.80),  // 5Y: ~4% rate
            (10.0, 0.64), // 10Y: ~4% rate
        ])
        .build()
        .unwrap()
}

fn create_test_swaption_quotes() -> Vec<MarketQuote> {
    vec![
        // 1Y x 1Y bucket (5 strikes)
        MarketQuote::Vol(VolQuote::SwaptionVol {
            expiry: Date::from_calendar_date(2026, Month::January, 1).unwrap(),
            tenor: Date::from_calendar_date(2027, Month::January, 1).unwrap(),
            strike: 0.035,
            vol: 120.0,
            quote_type: "OTM-100".to_string(),
            conventions: Default::default(),
            fixed_leg_conventions: Default::default(),
            float_leg_conventions: Default::default(),
        }),
        MarketQuote::Vol(VolQuote::SwaptionVol {
            expiry: Date::from_calendar_date(2026, Month::January, 1).unwrap(),
            tenor: Date::from_calendar_date(2027, Month::January, 1).unwrap(),
            strike: 0.040,
            vol: 100.0,
            quote_type: "ATM-50".to_string(),
            conventions: Default::default(),
            fixed_leg_conventions: Default::default(),
            float_leg_conventions: Default::default(),
        }),
        MarketQuote::Vol(VolQuote::SwaptionVol {
            expiry: Date::from_calendar_date(2026, Month::January, 1).unwrap(),
            tenor: Date::from_calendar_date(2027, Month::January, 1).unwrap(),
            strike: 0.043,
            vol: 90.0,
            quote_type: "ATM".to_string(),
            conventions: Default::default(),
            fixed_leg_conventions: Default::default(),
            float_leg_conventions: Default::default(),
        }),
        MarketQuote::Vol(VolQuote::SwaptionVol {
            expiry: Date::from_calendar_date(2026, Month::January, 1).unwrap(),
            tenor: Date::from_calendar_date(2027, Month::January, 1).unwrap(),
            strike: 0.046,
            vol: 100.0,
            quote_type: "ATM+50".to_string(),
            conventions: Default::default(),
            fixed_leg_conventions: Default::default(),
            float_leg_conventions: Default::default(),
        }),
        MarketQuote::Vol(VolQuote::SwaptionVol {
            expiry: Date::from_calendar_date(2026, Month::January, 1).unwrap(),
            tenor: Date::from_calendar_date(2027, Month::January, 1).unwrap(),
            strike: 0.050,
            vol: 120.0,
            quote_type: "OTM+100".to_string(),
            conventions: Default::default(),
            fixed_leg_conventions: Default::default(),
            float_leg_conventions: Default::default(),
        }),
        // 1Y x 5Y bucket (5 strikes)
        MarketQuote::Vol(VolQuote::SwaptionVol {
            expiry: Date::from_calendar_date(2026, Month::January, 1).unwrap(),
            tenor: Date::from_calendar_date(2031, Month::January, 1).unwrap(),
            strike: 0.038,
            vol: 85.0,
            quote_type: "OTM-100".to_string(),
            conventions: Default::default(),
            fixed_leg_conventions: Default::default(),
            float_leg_conventions: Default::default(),
        }),
        MarketQuote::Vol(VolQuote::SwaptionVol {
            expiry: Date::from_calendar_date(2026, Month::January, 1).unwrap(),
            tenor: Date::from_calendar_date(2031, Month::January, 1).unwrap(),
            strike: 0.042,
            vol: 75.0,
            quote_type: "ATM-50".to_string(),
            conventions: Default::default(),
            fixed_leg_conventions: Default::default(),
            float_leg_conventions: Default::default(),
        }),
        MarketQuote::Vol(VolQuote::SwaptionVol {
            expiry: Date::from_calendar_date(2026, Month::January, 1).unwrap(),
            tenor: Date::from_calendar_date(2031, Month::January, 1).unwrap(),
            strike: 0.045,
            vol: 70.0,
            quote_type: "ATM".to_string(),
            conventions: Default::default(),
            fixed_leg_conventions: Default::default(),
            float_leg_conventions: Default::default(),
        }),
        MarketQuote::Vol(VolQuote::SwaptionVol {
            expiry: Date::from_calendar_date(2026, Month::January, 1).unwrap(),
            tenor: Date::from_calendar_date(2031, Month::January, 1).unwrap(),
            strike: 0.048,
            vol: 75.0,
            quote_type: "ATM+50".to_string(),
            conventions: Default::default(),
            fixed_leg_conventions: Default::default(),
            float_leg_conventions: Default::default(),
        }),
        MarketQuote::Vol(VolQuote::SwaptionVol {
            expiry: Date::from_calendar_date(2026, Month::January, 1).unwrap(),
            tenor: Date::from_calendar_date(2031, Month::January, 1).unwrap(),
            strike: 0.052,
            vol: 85.0,
            quote_type: "OTM+100".to_string(),
            conventions: Default::default(),
            fixed_leg_conventions: Default::default(),
            float_leg_conventions: Default::default(),
        }),
    ]
}

#[test]
fn swaption_vol_step_builds_and_inserts_surface() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let currency = Currency::USD;

    let initial_market =
        MarketContext::new().insert_discount(create_test_discount_curve(base_date));

    let mut quote_sets: HashMap<String, Vec<MarketQuote>> = HashMap::new();
    quote_sets.insert("swpt".to_string(), create_test_swaption_quotes());

    let settings = CalibrationConfig {
        solver: finstack_valuations::calibration::solver::SolverConfig::brent_default()
            .with_tolerance(1e-10)
            .with_max_iterations(200),
        ..Default::default()
    };

    let plan = CalibrationPlanV2 {
        id: "plan".to_string(),
        description: None,
        quote_sets,
        settings,
        steps: vec![CalibrationStepV2 {
            id: "swpt".to_string(),
            quote_set: "swpt".to_string(),
            params: StepParams::SwaptionVol(SwaptionVolParams {
                surface_id: "USD-SWPT".to_string(),
                base_date,
                discount_curve_id: CurveId::from("USD-OIS"),
                forward_id: None,
                currency,
                vol_convention: SwaptionVolConvention::Normal,
                atm_convention: Default::default(),
                sabr_beta: 0.0,
                target_expiries: vec![1.0],
                target_tenors: vec![1.0, 5.0],
                sabr_interpolation: Default::default(),
                calendar_id: None,
                fixed_day_count: None,
                vol_tolerance: None,
                sabr_tolerance: None,
                sabr_extrapolation: SurfaceExtrapolationPolicy::Error,
                allow_sabr_missing_bucket_fallback: false,
            }),
        }],
    };

    let envelope = CalibrationEnvelopeV2 {
        schema: "finstack.calibration/2".to_string(),
        plan,
        initial_market: Some((&initial_market).into()),
    };

    let result = engine::execute(&envelope).expect("execute");
    assert!(result.result.report.success);
    let step = result.result.step_reports.get("swpt").expect("step report");
    assert!(step.success);
    assert!(
        !step.residuals.is_empty(),
        "expected residuals for calibrated buckets"
    );

    let ctx = MarketContext::try_from(result.result.final_market).expect("restore context");
    let surface = ctx.surface("USD-SWPT").expect("surface inserted");

    // Surface axes are (expiry, tenor) for swaption calibration.
    let v_1y_1y = surface.value_clamped(1.0, 1.0);
    let v_1y_5y = surface.value_clamped(1.0, 5.0);
    assert!(v_1y_1y.is_finite() && v_1y_1y > 0.0);
    assert!(v_1y_5y.is_finite() && v_1y_5y > 0.0);
}

#[test]
fn swaption_vol_out_of_bounds_targets_error_by_default() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let currency = Currency::USD;

    let initial_market =
        MarketContext::new().insert_discount(create_test_discount_curve(base_date));

    let mut quote_sets: HashMap<String, Vec<MarketQuote>> = HashMap::new();
    quote_sets.insert("swpt".to_string(), create_test_swaption_quotes());

    let plan = CalibrationPlanV2 {
        id: "plan".to_string(),
        description: None,
        quote_sets,
        settings: Default::default(),
        steps: vec![CalibrationStepV2 {
            id: "swpt".to_string(),
            quote_set: "swpt".to_string(),
            params: StepParams::SwaptionVol(SwaptionVolParams {
                surface_id: "USD-SWPT".to_string(),
                base_date,
                discount_curve_id: CurveId::from("USD-OIS"),
                forward_id: None,
                currency,
                vol_convention: SwaptionVolConvention::Normal,
                atm_convention: Default::default(),
                sabr_beta: 0.0,
                target_expiries: vec![0.5],
                target_tenors: vec![1.0, 5.0],
                sabr_interpolation: Default::default(),
                calendar_id: None,
                fixed_day_count: None,
                vol_tolerance: None,
                sabr_tolerance: None,
                sabr_extrapolation: SurfaceExtrapolationPolicy::Error,
                allow_sabr_missing_bucket_fallback: false,
            }),
        }],
    };

    let envelope = CalibrationEnvelopeV2 {
        schema: "finstack.calibration/2".to_string(),
        plan,
        initial_market: Some((&initial_market).into()),
    };

    let err = engine::execute(&envelope).expect_err("out-of-bounds targets should error");
    let msg = err.to_string();
    assert!(msg.contains("out of bounds"));
    assert!(msg.contains("sabr_extrapolation"));
}

#[test]
fn swaption_vol_out_of_bounds_targets_can_clamp_when_configured() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let currency = Currency::USD;

    let initial_market =
        MarketContext::new().insert_discount(create_test_discount_curve(base_date));

    let mut quote_sets: HashMap<String, Vec<MarketQuote>> = HashMap::new();
    quote_sets.insert("swpt".to_string(), create_test_swaption_quotes());

    let plan = CalibrationPlanV2 {
        id: "plan".to_string(),
        description: None,
        quote_sets,
        settings: Default::default(),
        steps: vec![CalibrationStepV2 {
            id: "swpt".to_string(),
            quote_set: "swpt".to_string(),
            params: StepParams::SwaptionVol(SwaptionVolParams {
                surface_id: "USD-SWPT".to_string(),
                base_date,
                discount_curve_id: CurveId::from("USD-OIS"),
                forward_id: None,
                currency,
                vol_convention: SwaptionVolConvention::Normal,
                atm_convention: Default::default(),
                sabr_beta: 0.0,
                target_expiries: vec![0.5, 1.0],
                target_tenors: vec![1.0, 5.0],
                sabr_interpolation: Default::default(),
                calendar_id: None,
                fixed_day_count: None,
                vol_tolerance: None,
                sabr_tolerance: None,
                sabr_extrapolation: SurfaceExtrapolationPolicy::Clamp,
                allow_sabr_missing_bucket_fallback: false,
            }),
        }],
    };

    let envelope = CalibrationEnvelopeV2 {
        schema: "finstack.calibration/2".to_string(),
        plan,
        initial_market: Some((&initial_market).into()),
    };

    let result = engine::execute(&envelope).expect("execute");
    let step = result.result.step_reports.get("swpt").expect("step report");
    assert!(step.success);
    assert_eq!(
        step.metadata
            .get("clamped_target_points")
            .map(|v| v.as_str()),
        Some("2")
    );
}
