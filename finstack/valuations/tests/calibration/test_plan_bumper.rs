//! Tests for calibration plan bumping utilities (`PlanBumper`).

use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount};
use finstack_core::types::CurveId;
use finstack_valuations::calibration::api::schema::{
    CalibrationPlan, CalibrationStep, DiscountCurveParams, StepParams,
};
use finstack_valuations::calibration::bumps::{BumpRequest, PlanBumper};
use finstack_valuations::market::conventions::ids::IndexId;
use finstack_valuations::market::quotes::ids::{Pillar, QuoteId};
use finstack_valuations::market::quotes::market_quote::MarketQuote;
use finstack_valuations::market::quotes::rates::RateQuote;
use std::collections::HashMap;

fn d(y: i32, m: time::Month, day: u8) -> Date {
    Date::from_calendar_date(y, m, day).expect("valid date")
}

fn deposit_quote(id: &str, pillar: Pillar, rate: f64) -> MarketQuote {
    MarketQuote::Rates(RateQuote::Deposit {
        id: QuoteId::new(id),
        index: IndexId::new("USD-SOFR-1M"),
        pillar,
        rate,
    })
}

fn discount_step(id: &str, quote_set: &str, base_date: Date) -> CalibrationStep {
    CalibrationStep {
        id: id.to_string(),
        quote_set: quote_set.to_string(),
        params: StepParams::Discount(DiscountCurveParams {
            curve_id: CurveId::new("USD-OIS"),
            currency: Currency::USD,
            base_date,
            method: Default::default(),
            interpolation: Default::default(),
            extrapolation: Default::default(),
            pricing_discount_id: None,
            pricing_forward_id: None,
            conventions: Default::default(),
        }),
    }
}

#[test]
fn plan_bumper_parallel_bumps_all_quote_sets() -> finstack_core::Result<()> {
    let base = d(2025, time::Month::January, 1);

    let mut quote_sets: HashMap<String, Vec<MarketQuote>> = HashMap::new();
    quote_sets.insert(
        "qs1".to_string(),
        vec![
            deposit_quote("DEP-1Y", Pillar::Date(d(2026, time::Month::January, 1)), 0.05),
            deposit_quote("DEP-6M", Pillar::Date(d(2025, time::Month::July, 1)), 0.04),
        ],
    );
    quote_sets.insert(
        "qs2".to_string(),
        vec![deposit_quote(
            "DEP-3M",
            Pillar::Date(d(2025, time::Month::April, 1)),
            0.03,
        )],
    );

    let mut plan = CalibrationPlan {
        id: "PLAN".to_string(),
        description: None,
        quote_sets,
        steps: vec![discount_step("disc", "qs1", base)],
        settings: Default::default(),
    };

    // +10bp
    PlanBumper::bump(&mut plan, &BumpRequest::Parallel(10.0))?;

    let qs1 = plan.quote_sets.get("qs1").unwrap();
    let qs2 = plan.quote_sets.get("qs2").unwrap();

    let r1 = match &qs1[0] {
        MarketQuote::Rates(RateQuote::Deposit { rate, .. }) => *rate,
        _ => panic!("expected deposit"),
    };
    let r2 = match &qs1[1] {
        MarketQuote::Rates(RateQuote::Deposit { rate, .. }) => *rate,
        _ => panic!("expected deposit"),
    };
    let r3 = match &qs2[0] {
        MarketQuote::Rates(RateQuote::Deposit { rate, .. }) => *rate,
        _ => panic!("expected deposit"),
    };

    // bp_to_decimal(10bp) = 0.001
    assert!((r1 - 0.051).abs() < 1e-12);
    assert!((r2 - 0.041).abs() < 1e-12);
    assert!((r3 - 0.031).abs() < 1e-12);

    Ok(())
}

#[test]
fn plan_bumper_tenor_bumps_apply_per_step_base_date() -> finstack_core::Result<()> {
    // One quote set referenced by two steps with different base dates.
    // The same quote can be matched/bumped twice (once per referencing step).
    let base1 = d(2025, time::Month::January, 1);
    let base2 = d(2025, time::Month::July, 1);

    let quote = deposit_quote("DEP-END", Pillar::Date(d(2026, time::Month::January, 1)), 0.05);

    let mut quote_sets: HashMap<String, Vec<MarketQuote>> = HashMap::new();
    quote_sets.insert("qs".to_string(), vec![quote]);

    let mut plan = CalibrationPlan {
        id: "PLAN".to_string(),
        description: None,
        quote_sets,
        steps: vec![
            discount_step("disc1", "qs", base1),
            discount_step("disc2", "qs", base2),
        ],
        settings: Default::default(),
    };

    // Target 1.0y from base1 (2025-01-01 -> 2026-01-01) and 0.5y from base2.
    let bump = BumpRequest::Tenors(vec![(1.0, 10.0), (0.5, 5.0)]);
    PlanBumper::bump(&mut plan, &bump)?;

    let bumped_rate = match &plan.quote_sets["qs"][0] {
        MarketQuote::Rates(RateQuote::Deposit { rate, .. }) => *rate,
        _ => panic!("expected deposit"),
    };

    // Applies twice: 10bp + 5bp = 15bp total -> 0.0015
    assert!((bumped_rate - 0.0515).abs() < 1e-12);

    Ok(())
}

#[test]
fn plan_bumper_missing_quote_set_is_error() {
    let base = d(2025, time::Month::January, 1);
    let plan = CalibrationPlan {
        id: "PLAN".to_string(),
        description: None,
        quote_sets: HashMap::new(),
        steps: vec![discount_step("disc", "does_not_exist", base)],
        settings: Default::default(),
    };

    let err = PlanBumper::apply(plan, &BumpRequest::Tenors(vec![(1.0, 1.0)]))
        .expect_err("missing quote_set should error");
    match err {
        finstack_core::Error::Input(finstack_core::error::InputError::NotFound { .. }) => {}
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn plan_bumper_apply_returns_new_plan() -> finstack_core::Result<()> {
    let base = d(2025, time::Month::January, 1);
    let mut quote_sets: HashMap<String, Vec<MarketQuote>> = HashMap::new();
    quote_sets.insert(
        "qs".to_string(),
        vec![deposit_quote(
            "DEP-1Y",
            Pillar::Date(d(2026, time::Month::January, 1)),
            0.05,
        )],
    );

    let plan = CalibrationPlan {
        id: "PLAN".to_string(),
        description: None,
        quote_sets,
        steps: vec![discount_step("disc", "qs", base)],
        settings: Default::default(),
    };

    let bumped = PlanBumper::apply(plan.clone(), &BumpRequest::Parallel(1.0))?;

    let original_rate = match &plan.quote_sets["qs"][0] {
        MarketQuote::Rates(RateQuote::Deposit { rate, .. }) => *rate,
        _ => panic!("expected deposit"),
    };
    let bumped_rate = match &bumped.quote_sets["qs"][0] {
        MarketQuote::Rates(RateQuote::Deposit { rate, .. }) => *rate,
        _ => panic!("expected deposit"),
    };

    assert!((original_rate - 0.05).abs() < 1e-12);
    assert!((bumped_rate - 0.0501).abs() < 1e-12); // +1bp

    Ok(())
}


