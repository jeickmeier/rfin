//! Calibration repricing tests (v2) with market-standard tolerance requirements.
//!
//! The goal is to ensure that curves produced by v2 calibration steps can reprice
//! instruments constructed *outside* the solver to reasonable tolerances.

use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::math::interp::ExtrapolationPolicy;
use finstack_core::money::Money;
use finstack_core::types::{Currency, CurveId};
use finstack_valuations::calibration::api::engine;
use finstack_valuations::calibration::api::schema::{
    CalibrationEnvelope, CalibrationMethod, CalibrationPlan, CalibrationStep, DiscountCurveParams,
    ForwardCurveParams, StepParams,
};
use finstack_valuations::calibration::CalibrationConfig;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::instruments::ForwardRateAgreement;
use finstack_valuations::market::build::context::BuildCtx;
use finstack_valuations::market::build::rates::build_rate_instrument;
use finstack_valuations::market::conventions::ids::IndexId;
use finstack_valuations::market::quotes::ids::{Pillar, QuoteId};
use finstack_valuations::market::quotes::market_quote::MarketQuote;
use finstack_valuations::market::quotes::rates::RateQuote;
use finstack_core::collections::HashMap;
use time::Month;

use super::tolerances;

const NOTIONAL: f64 = 1_000_000.0; // $1M notional

/// FRA repricing tolerance per $1M notional.
const FRA_TOLERANCE_DOLLARS: f64 = tolerances::FRA_REPRICE_ABS_TOL_DOLLARS;

fn run_plan(envelope: &CalibrationEnvelope) -> MarketContext {
    let out = engine::execute(envelope).expect("calibration should succeed");
    MarketContext::try_from(out.result.final_market).expect("restore context")
}

#[test]
fn discount_curve_deposit_repricing() {
    // Use a business day as base_date to avoid holiday adjustment complications.
    let base_date = Date::from_calendar_date(2025, Month::January, 2).unwrap();
    let currency = Currency::USD;

    // Market standard deposits are quoted by tenor (from spot).
    let deposit_quotes: Vec<RateQuote> = vec![
        RateQuote::Deposit {
            id: QuoteId::new("DEP-1M"),
            index: IndexId::new("USD-Deposit"),
            pillar: Pillar::Tenor(finstack_core::dates::Tenor::parse("1M").unwrap()),
            rate: 0.045,
        },
        RateQuote::Deposit {
            id: QuoteId::new("DEP-3M"),
            index: IndexId::new("USD-Deposit"),
            pillar: Pillar::Tenor(finstack_core::dates::Tenor::parse("3M").unwrap()),
            rate: 0.046,
        },
        RateQuote::Deposit {
            id: QuoteId::new("DEP-6M"),
            index: IndexId::new("USD-Deposit"),
            pillar: Pillar::Tenor(finstack_core::dates::Tenor::parse("6M").unwrap()),
            rate: 0.047,
        },
    ];

    let mut quote_sets: HashMap<String, Vec<MarketQuote>> = HashMap::default();
    quote_sets.insert(
        "mm".to_string(),
        deposit_quotes
            .iter()
            .cloned()
            .map(MarketQuote::Rates)
            .collect(),
    );

    let settings = CalibrationConfig {
        solver: finstack_valuations::calibration::solver::SolverConfig::brent_default()
            .with_tolerance(1e-12)
            .with_max_iterations(200),
        ..Default::default()
    };

    let plan = CalibrationPlan {
        id: "plan".to_string(),
        description: None,
        quote_sets,
        settings,
        steps: vec![CalibrationStep {
            id: "disc".to_string(),
            quote_set: "mm".to_string(),
            params: StepParams::Discount(DiscountCurveParams {
                curve_id: CurveId::from("USD-OIS"),
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
    };

    let envelope = CalibrationEnvelope {
        schema: "finstack.calibration/2".to_string(),
        plan,
        initial_market: None,
    };

    let ctx = run_plan(&envelope);

    let build_ctx = BuildCtx {
        as_of: base_date,
        notional: NOTIONAL,
        curve_ids: [("discount".to_string(), "USD-OIS".to_string())]
            .into_iter()
            .collect(),
        attributes: Default::default(),
    };

    for q in &deposit_quotes {
        let inst = build_rate_instrument(q, &build_ctx).expect("build deposit instrument");
        let pv = inst.value(&ctx, base_date).unwrap();
        assert!(
            pv.amount().abs() <= tolerances::REPRICE_PV_ABS_TOL_DOLLARS,
            "deposit should reprice within ${}. PV=${:.6}",
            tolerances::REPRICE_PV_ABS_TOL_DOLLARS,
            pv.amount(),
        );
    }
}

#[test]
fn forward_curve_fra_repricing() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let currency = Currency::USD;

    // Discount quotes (minimal)
    let disc_quotes: Vec<RateQuote> = vec![
        RateQuote::Deposit {
            id: QuoteId::new(format!("DEP-{:?}", base_date + time::Duration::days(30))),
            index: IndexId::new("USD-Deposit"),
            pillar: Pillar::Date(base_date + time::Duration::days(30)),
            rate: 0.0450,
        },
        RateQuote::Deposit {
            id: QuoteId::new(format!("DEP-{:?}", base_date + time::Duration::days(90))),
            index: IndexId::new("USD-Deposit"),
            pillar: Pillar::Date(base_date + time::Duration::days(90)),
            rate: 0.0460,
        },
    ];

    // Forward quotes (FRAs)
    let fra_quotes: Vec<RateQuote> = vec![
        RateQuote::Fra {
            id: QuoteId::new(format!(
                "FRA-{:?}-{:?}",
                base_date + time::Duration::days(90),
                base_date + time::Duration::days(180)
            )),
            index: IndexId::new("USD-LIBOR-3M"),
            start: Pillar::Date(base_date + time::Duration::days(90)),
            end: Pillar::Date(base_date + time::Duration::days(180)),
            rate: 0.0470,
        },
        RateQuote::Fra {
            id: QuoteId::new(format!(
                "FRA-{:?}-{:?}",
                base_date + time::Duration::days(180),
                base_date + time::Duration::days(270)
            )),
            index: IndexId::new("USD-LIBOR-3M"),
            start: Pillar::Date(base_date + time::Duration::days(180)),
            end: Pillar::Date(base_date + time::Duration::days(270)),
            rate: 0.0480,
        },
    ];

    let mut quote_sets: HashMap<String, Vec<MarketQuote>> = HashMap::default();
    quote_sets.insert(
        "disc".to_string(),
        disc_quotes
            .iter()
            .cloned()
            .map(MarketQuote::Rates)
            .collect(),
    );
    quote_sets.insert(
        "fra".to_string(),
        fra_quotes.iter().cloned().map(MarketQuote::Rates).collect(),
    );

    let settings = CalibrationConfig {
        solver: finstack_valuations::calibration::solver::SolverConfig::brent_default()
            .with_tolerance(1e-12)
            .with_max_iterations(200),
        ..Default::default()
    };

    let plan = CalibrationPlan {
        id: "plan".to_string(),
        description: None,
        quote_sets,
        settings,
        steps: vec![
            CalibrationStep {
                id: "disc".to_string(),
                quote_set: "disc".to_string(),
                params: StepParams::Discount(DiscountCurveParams {
                    curve_id: CurveId::from("USD-OIS"),
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
            CalibrationStep {
                id: "fwd".to_string(),
                quote_set: "fra".to_string(),
                params: StepParams::Forward(ForwardCurveParams {
                    curve_id: CurveId::from("USD-SOFR-3M"),
                    currency,
                    base_date,
                    tenor_years: 0.25,
                    discount_curve_id: CurveId::from("USD-OIS"),
                    method: CalibrationMethod::Bootstrap,
                    interpolation: Default::default(),
                    conventions: Default::default(),
                }),
            },
        ],
    };

    let envelope = CalibrationEnvelope {
        schema: "finstack.calibration/2".to_string(),
        plan,
        initial_market: None,
    };

    let ctx = run_plan(&envelope);

    for q in &fra_quotes {
        let (start, end, rate) = match q {
            RateQuote::Fra {
                start: Pillar::Date(s),
                end: Pillar::Date(e),
                rate,
                ..
            } => (*s, *e, *rate),
            _ => continue,
        };

        let day_count = finstack_core::dates::DayCount::Act360;

        // Heuristic fixing date: T-2 if possible.
        let fixing_date = if start >= base_date + time::Duration::days(2) {
            start - time::Duration::days(2)
        } else {
            base_date
        };

        let fra = ForwardRateAgreement::builder()
            .id(format!("FRA-{}-{}", start, end).into())
            .notional(Money::new(NOTIONAL, currency))
            .fixing_date(fixing_date)
            .start_date(start)
            .end_date(end)
            .fixed_rate(rate)
            .day_count(day_count)
            .reset_lag(2)
            .discount_curve_id("USD-OIS".into())
            .forward_id("USD-SOFR-3M".into())
            .pay_fixed(false)
            .build()
            .unwrap();

        let pv = fra.value(&ctx, base_date).unwrap();
        assert!(
            pv.amount().abs() <= FRA_TOLERANCE_DOLLARS,
            "fra should reprice within ${}. PV=${:.2}",
            FRA_TOLERANCE_DOLLARS,
            pv.amount()
        );
    }
}
