//! Calibration repricing tests (v2) with market-standard tolerance requirements.
//!
//! The goal is to ensure that curves produced by v2 calibration steps can reprice
//! instruments constructed *outside* the solver to reasonable tolerances.

use finstack_core::dates::{Date, DayCount, Tenor};
use finstack_core::market_data::context::MarketContext;
use finstack_core::math::interp::{ExtrapolationPolicy, InterpStyle};
use finstack_core::money::Money;
use finstack_core::types::{Currency, CurveId};
use finstack_valuations::calibration::CalibrationConfig;
use finstack_valuations::calibration::v2::api::engine;
use finstack_valuations::calibration::v2::api::schema::{
    CalibrationEnvelopeV2, CalibrationMethod, CalibrationPlanV2, CalibrationStepV2,
    DiscountCurveParams, ForwardCurveParams, StepParams,
};
use finstack_valuations::calibration::v2::domain::pricing::CalibrationPricer;
use finstack_valuations::calibration::v2::domain::quotes::{InstrumentConventions, MarketQuote, RatesQuote};
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::instruments::irs::InterestRateSwap;
use finstack_valuations::instruments::{Deposit, ForwardRateAgreement};
use finstack_valuations::metrics::MetricCalculator;
use std::collections::HashMap;
use time::Month;

const NOTIONAL: f64 = 1_000_000.0; // $1M notional

/// OIS swap repricing tolerance in basis points for externally-constructed swaps.
const OIS_SWAP_TOLERANCE_BP: f64 = 1.5;

/// FRA repricing tolerance per $1M notional.
const FRA_TOLERANCE_DOLLARS: f64 = 150.0;

fn calculate_swap_dv01(swap: &InterestRateSwap, ctx: &MarketContext, as_of: Date) -> f64 {
    use finstack_valuations::metrics::MetricContext;

    let base_pv = swap.value(ctx, as_of).unwrap();
    let mut metric_ctx = MetricContext::new(
        std::sync::Arc::new(swap.clone()),
        std::sync::Arc::new(ctx.clone()),
        as_of,
        base_pv,
    );

    use finstack_valuations::metrics::{Dv01CalculatorConfig, UnifiedDv01Calculator};
    let dv01_calc =
        UnifiedDv01Calculator::<InterestRateSwap>::new(Dv01CalculatorConfig::parallel_combined());
    dv01_calc.calculate(&mut metric_ctx).unwrap()
}

fn swap_tolerance_from_dv01(dv01: f64, tolerance_bp: f64) -> f64 {
    (tolerance_bp * dv01.abs()).max(50.0) // Minimum $50 tolerance
}

fn run_plan(envelope: &CalibrationEnvelopeV2) -> MarketContext {
    let out = engine::execute(envelope).expect("calibration should succeed");
    MarketContext::try_from(out.result.final_market).expect("restore context")
}

#[test]
fn discount_curve_swap_repricing() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let currency = Currency::USD;

    let quotes: Vec<RatesQuote> = vec![
        RatesQuote::Deposit {
            maturity: base_date + time::Duration::days(30),
            rate: 0.0450,
            conventions: InstrumentConventions::default()
                .with_day_count(DayCount::Act360)
                .with_settlement_days(0),
        },
        RatesQuote::Deposit {
            maturity: base_date + time::Duration::days(90),
            rate: 0.0460,
            conventions: InstrumentConventions::default()
                .with_day_count(DayCount::Act360)
                .with_settlement_days(0),
        },
        RatesQuote::Swap {
            maturity: base_date + time::Duration::days(365),
            rate: 0.0470,
            is_ois: true,
            conventions: Default::default(),
            fixed_leg_conventions: InstrumentConventions::default()
                .with_payment_frequency(Tenor::semi_annual())
                .with_day_count(DayCount::Thirty360),
            float_leg_conventions: InstrumentConventions::default()
                .with_payment_frequency(Tenor::daily())
                .with_day_count(DayCount::Act360)
                .with_index("USD-OIS"),
        },
        RatesQuote::Swap {
            maturity: base_date + time::Duration::days(365 * 2),
            rate: 0.0480,
            is_ois: true,
            conventions: Default::default(),
            fixed_leg_conventions: InstrumentConventions::default()
                .with_payment_frequency(Tenor::semi_annual())
                .with_day_count(DayCount::Thirty360),
            float_leg_conventions: InstrumentConventions::default()
                .with_payment_frequency(Tenor::daily())
                .with_day_count(DayCount::Act360)
                .with_index("USD-OIS"),
        },
        RatesQuote::Swap {
            maturity: base_date + time::Duration::days(365 * 5),
            rate: 0.0490,
            is_ois: true,
            conventions: Default::default(),
            fixed_leg_conventions: InstrumentConventions::default()
                .with_payment_frequency(Tenor::semi_annual())
                .with_day_count(DayCount::Thirty360),
            float_leg_conventions: InstrumentConventions::default()
                .with_payment_frequency(Tenor::daily())
                .with_day_count(DayCount::Act360)
                .with_index("USD-OIS"),
        },
    ];

    let mut quote_sets: HashMap<String, Vec<MarketQuote>> = HashMap::new();
    quote_sets.insert(
        "ois".to_string(),
        quotes.iter().cloned().map(MarketQuote::Rates).collect(),
    );

    let settings = CalibrationConfig {
        tolerance: 1e-12,
        max_iterations: 200,
        ..Default::default()
    };

    let plan = CalibrationPlanV2 {
        id: "plan".to_string(),
        description: None,
        quote_sets,
        settings,
        steps: vec![CalibrationStepV2 {
            id: "disc".to_string(),
            quote_set: "ois".to_string(),
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

    let envelope = CalibrationEnvelopeV2 {
        schema: "finstack.calibration/2".to_string(),
        plan,
        initial_market: Some((&MarketContext::new()).into()),
    };

    let ctx = run_plan(&envelope);

    // External repricing of swaps constructed with the v2 pricer.
    let pricer = CalibrationPricer::new(base_date, CurveId::from("USD-OIS"))
        .with_market_conventions(currency)
        .with_use_settlement_start(true);

    for q in &quotes {
        if matches!(q, RatesQuote::Swap { .. }) {
            let swap = pricer
                .create_ois_swap(q, Money::new(NOTIONAL, currency), currency)
                .expect("swap construction");

            let pv = swap.value(&ctx, base_date).unwrap();
            let dv01 = calculate_swap_dv01(&swap, &ctx, base_date);
            let tolerance = swap_tolerance_from_dv01(dv01, OIS_SWAP_TOLERANCE_BP);

            assert!(
                pv.amount().abs() <= tolerance,
                "swap should reprice within {}bp × DV01. PV=${:.2}, DV01=${:.2}, tol=${:.2}",
                OIS_SWAP_TOLERANCE_BP,
                pv.amount(),
                dv01,
                tolerance
            );
        }
    }
}

#[test]
fn discount_curve_swap_repricing_respects_requested_interp() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let currency = Currency::USD;

    // Use semi-annual fixed coupons so the swap annuity depends on interpolated DFs
    // between quote pillars (market-standard behavior).
    let quotes: Vec<RatesQuote> = vec![
        RatesQuote::Deposit {
            maturity: base_date + time::Duration::days(30),
            rate: 0.0450,
            conventions: InstrumentConventions::default()
                .with_day_count(DayCount::Act360)
                .with_settlement_days(0),
        },
        RatesQuote::Deposit {
            maturity: base_date + time::Duration::days(90),
            rate: 0.0460,
            conventions: InstrumentConventions::default()
                .with_day_count(DayCount::Act360)
                .with_settlement_days(0),
        },
        RatesQuote::Swap {
            maturity: base_date + time::Duration::days(365),
            rate: 0.0470,
            is_ois: true,
            conventions: Default::default(),
            fixed_leg_conventions: InstrumentConventions::default()
                .with_payment_frequency(Tenor::semi_annual())
                .with_day_count(DayCount::Thirty360),
            float_leg_conventions: InstrumentConventions::default()
                .with_payment_frequency(Tenor::daily())
                .with_day_count(DayCount::Act360)
                .with_index("USD-OIS"),
        },
        RatesQuote::Swap {
            maturity: base_date + time::Duration::days(365 * 2),
            rate: 0.0480,
            is_ois: true,
            conventions: Default::default(),
            fixed_leg_conventions: InstrumentConventions::default()
                .with_payment_frequency(Tenor::semi_annual())
                .with_day_count(DayCount::Thirty360),
            float_leg_conventions: InstrumentConventions::default()
                .with_payment_frequency(Tenor::daily())
                .with_day_count(DayCount::Act360)
                .with_index("USD-OIS"),
        },
        RatesQuote::Swap {
            maturity: base_date + time::Duration::days(365 * 5),
            rate: 0.0490,
            is_ois: true,
            conventions: Default::default(),
            fixed_leg_conventions: InstrumentConventions::default()
                .with_payment_frequency(Tenor::semi_annual())
                .with_day_count(DayCount::Thirty360),
            float_leg_conventions: InstrumentConventions::default()
                .with_payment_frequency(Tenor::daily())
                .with_day_count(DayCount::Act360)
                .with_index("USD-OIS"),
        },
    ];

    let mut quote_sets: HashMap<String, Vec<MarketQuote>> = HashMap::new();
    quote_sets.insert(
        "ois".to_string(),
        quotes.iter().cloned().map(MarketQuote::Rates).collect(),
    );

    let settings = CalibrationConfig {
        tolerance: 1e-12,
        max_iterations: 200,
        ..Default::default()
    };

    let plan = CalibrationPlanV2 {
        id: "plan".to_string(),
        description: None,
        quote_sets,
        settings,
        steps: vec![CalibrationStepV2 {
            id: "disc".to_string(),
            quote_set: "ois".to_string(),
            params: StepParams::Discount(DiscountCurveParams {
                curve_id: CurveId::from("USD-OIS"),
                currency,
                base_date,
                method: CalibrationMethod::Bootstrap,
                interpolation: InterpStyle::PiecewiseQuadraticForward,
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
        initial_market: Some((&MarketContext::new()).into()),
    };

    let ctx = run_plan(&envelope);

    // External repricing of swaps constructed with the v2 pricer.
    let pricer = CalibrationPricer::new(base_date, CurveId::from("USD-OIS"))
        .with_market_conventions(currency)
        .with_use_settlement_start(true);

    for q in &quotes {
        if matches!(q, RatesQuote::Swap { .. }) {
            let swap = pricer
                .create_ois_swap(q, Money::new(NOTIONAL, currency), currency)
                .expect("swap construction");

            let pv = swap.value(&ctx, base_date).unwrap();
            let dv01 = calculate_swap_dv01(&swap, &ctx, base_date);
            let tolerance = swap_tolerance_from_dv01(dv01, OIS_SWAP_TOLERANCE_BP);

            assert!(
                pv.amount().abs() <= tolerance,
                "swap should reprice within {}bp × DV01 under requested interpolation. PV=${:.2}, DV01=${:.2}, tol=${:.2}",
                OIS_SWAP_TOLERANCE_BP,
                pv.amount(),
                dv01,
                tolerance
            );
        }
    }
}

#[test]
fn discount_curve_deposit_repricing() {
    // Use a business day as base_date to avoid holiday adjustment complications.
    let base_date = Date::from_calendar_date(2025, Month::January, 2).unwrap();
    let currency = Currency::USD;

    let deposit_quotes: Vec<RatesQuote> = vec![
        RatesQuote::Deposit {
            maturity: base_date + time::Duration::days(30),
            rate: 0.045,
            conventions: InstrumentConventions::default()
                .with_day_count(DayCount::Act360)
                .with_settlement_days(0),
        },
        RatesQuote::Deposit {
            maturity: base_date + time::Duration::days(90),
            rate: 0.046,
            conventions: InstrumentConventions::default()
                .with_day_count(DayCount::Act360)
                .with_settlement_days(0),
        },
        RatesQuote::Deposit {
            maturity: base_date + time::Duration::days(180),
            rate: 0.047,
            conventions: InstrumentConventions::default()
                .with_day_count(DayCount::Act360)
                .with_settlement_days(0),
        },
    ];

    let mut quote_sets: HashMap<String, Vec<MarketQuote>> = HashMap::new();
    quote_sets.insert(
        "mm".to_string(),
        deposit_quotes
            .iter()
            .cloned()
            .map(MarketQuote::Rates)
            .collect(),
    );

    let settings = CalibrationConfig {
        tolerance: 1e-12,
        max_iterations: 200,
        ..Default::default()
    };

    let plan = CalibrationPlanV2 {
        id: "plan".to_string(),
        description: None,
        quote_sets,
        settings,
        steps: vec![CalibrationStepV2 {
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

    let envelope = CalibrationEnvelopeV2 {
        schema: "finstack.calibration/2".to_string(),
        plan,
        initial_market: None,
    };

    let ctx = run_plan(&envelope);

    for q in &deposit_quotes {
        let (maturity, rate, conventions) = match q {
            RatesQuote::Deposit {
                maturity,
                rate,
                conventions,
            } => (*maturity, *rate, conventions),
            _ => continue,
        };

        let day_count = conventions.day_count.unwrap_or_else(|| {
            InstrumentConventions::default_money_market_day_count(currency)
        });

        let dep = Deposit {
            id: format!("DEP-{}", maturity).into(),
            notional: Money::new(NOTIONAL, currency),
            start: base_date,
            end: maturity,
            day_count,
            quote_rate: Some(rate),
            discount_curve_id: "USD-OIS".into(),
            attributes: Default::default(),
            spot_lag_days: conventions.settlement_days,
            bdc: None,
            calendar_id: None,
        };

        let pv = dep.value(&ctx, base_date).unwrap();
        assert!(
            pv.amount().abs() <= 1.0,
            "deposit should reprice within $1. PV=${:.2}",
            pv.amount()
        );
    }
}

#[test]
fn forward_curve_fra_repricing() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let currency = Currency::USD;

    // Discount quotes (minimal)
    let disc_quotes: Vec<RatesQuote> = vec![
        RatesQuote::Deposit {
            maturity: base_date + time::Duration::days(30),
            rate: 0.0450,
            conventions: InstrumentConventions::default()
                .with_day_count(DayCount::Act360)
                .with_settlement_days(0),
        },
        RatesQuote::Deposit {
            maturity: base_date + time::Duration::days(90),
            rate: 0.0460,
            conventions: InstrumentConventions::default()
                .with_day_count(DayCount::Act360)
                .with_settlement_days(0),
        },
    ];

    // Forward quotes (FRAs)
    let fra_quotes: Vec<RatesQuote> = vec![
        RatesQuote::FRA {
            start: base_date + time::Duration::days(90),
            end: base_date + time::Duration::days(180),
            rate: 0.0470,
            conventions: InstrumentConventions::default().with_day_count(DayCount::Act360),
        },
        RatesQuote::FRA {
            start: base_date + time::Duration::days(180),
            end: base_date + time::Duration::days(270),
            rate: 0.0480,
            conventions: InstrumentConventions::default().with_day_count(DayCount::Act360),
        },
    ];

    let mut quote_sets: HashMap<String, Vec<MarketQuote>> = HashMap::new();
    quote_sets.insert(
        "disc".to_string(),
        disc_quotes.iter().cloned().map(MarketQuote::Rates).collect(),
    );
    quote_sets.insert(
        "fra".to_string(),
        fra_quotes.iter().cloned().map(MarketQuote::Rates).collect(),
    );

    let settings = CalibrationConfig {
        tolerance: 1e-12,
        max_iterations: 200,
        ..Default::default()
    };

    let plan = CalibrationPlanV2 {
        id: "plan".to_string(),
        description: None,
        quote_sets,
        settings,
        steps: vec![
            CalibrationStepV2 {
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
            CalibrationStepV2 {
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

    let envelope = CalibrationEnvelopeV2 {
        schema: "finstack.calibration/2".to_string(),
        plan,
        initial_market: None,
    };

    let ctx = run_plan(&envelope);

    for q in &fra_quotes {
        let (start, end, rate, conventions) = match q {
            RatesQuote::FRA {
                start,
                end,
                rate,
                conventions,
            } => (*start, *end, *rate, conventions),
            _ => continue,
        };

        let day_count = conventions.day_count.unwrap_or_else(|| {
            InstrumentConventions::default_money_market_day_count(currency)
        });

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
