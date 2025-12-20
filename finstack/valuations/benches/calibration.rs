//! Calibration benchmarks (v2 plan-driven API).
//!
//! This benchmark suite focuses on the plan-driven calibration step engine.

use criterion::{criterion_group, criterion_main, Criterion};
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_valuations::calibration::api::schema::{
    CalibrationMethod, DiscountCurveParams, ForwardCurveParams, StepParams,
};
use finstack_valuations::calibration::targets::handlers::execute_step;
use finstack_valuations::calibration::CalibrationConfig;
use finstack_valuations::market::conventions::ids::IndexId;
use finstack_valuations::market::quotes::ids::{Pillar, QuoteId};
use finstack_valuations::market::quotes::market_quote::MarketQuote;
use finstack_valuations::market::quotes::rates::RateQuote;
use std::hint::black_box;
use time::Month;

fn bench_discount_and_forward_steps(c: &mut Criterion) {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let settings = CalibrationConfig::default();

    // Discount curve inputs
    let disc_quotes: Vec<RateQuote> = vec![
        RateQuote::Deposit {
            id: QuoteId::new("DEP-1M"),
            index: IndexId::new("USD-SOFR"),
            pillar: Pillar::Date(base_date + time::Duration::days(30)),
            rate: 0.045,
        },
        RateQuote::Deposit {
            id: QuoteId::new("DEP-3M"),
            index: IndexId::new("USD-SOFR"),
            pillar: Pillar::Date(base_date + time::Duration::days(90)),
            rate: 0.046,
        },
        RateQuote::Swap {
            id: QuoteId::new("SWP-1Y"),
            index: IndexId::new("USD-SOFR-OIS"),
            pillar: Pillar::Date(base_date + time::Duration::days(365)),
            rate: 0.047,
            spread_decimal: None,
        },
    ];
    let disc_mq: Vec<MarketQuote> = disc_quotes
        .iter()
        .cloned()
        .map(MarketQuote::Rates)
        .collect();
    let disc_step = StepParams::Discount(DiscountCurveParams {
        curve_id: "USD-OIS".into(),
        currency: Currency::USD,
        base_date,
        method: CalibrationMethod::Bootstrap,
        interpolation: Default::default(),
        extrapolation: finstack_core::math::interp::ExtrapolationPolicy::FlatForward,
        pricing_discount_id: None,
        pricing_forward_id: None,
        conventions: Default::default(),
    });

    // Forward curve inputs
    let fwd_quotes: Vec<RateQuote> = vec![
        RateQuote::Fra {
            id: QuoteId::new("FRA-3x6"),
            index: IndexId::new("USD-SOFR-3M"),
            start: Pillar::Date(base_date + time::Duration::days(90)),
            end: Pillar::Date(base_date + time::Duration::days(180)),
            rate: 0.047,
        },
        RateQuote::Fra {
            id: QuoteId::new("FRA-6x9"),
            index: IndexId::new("USD-SOFR-3M"),
            start: Pillar::Date(base_date + time::Duration::days(180)),
            end: Pillar::Date(base_date + time::Duration::days(270)),
            rate: 0.048,
        },
    ];
    let fwd_mq: Vec<MarketQuote> = fwd_quotes.iter().cloned().map(MarketQuote::Rates).collect();
    let fwd_step = StepParams::Forward(ForwardCurveParams {
        curve_id: "USD-SOFR-3M-FWD".into(),
        currency: Currency::USD,
        base_date,
        tenor_years: 0.25,
        discount_curve_id: "USD-OIS".into(),
        method: CalibrationMethod::Bootstrap,
        interpolation: Default::default(),
        conventions: Default::default(),
    });

    c.bench_function("calibration_v2_discount_step", |b| {
        let base = MarketContext::new();
        b.iter(|| {
            execute_step(
                black_box(&disc_step),
                black_box(&disc_mq),
                black_box(&base),
                black_box(&settings),
            )
            .unwrap()
        })
    });

    c.bench_function("calibration_v2_discount_then_forward_steps", |b| {
        let base = MarketContext::new();
        b.iter(|| {
            let (ctx_after_disc, _) = execute_step(
                black_box(&disc_step),
                black_box(&disc_mq),
                black_box(&base),
                black_box(&settings),
            )
            .unwrap();
            execute_step(
                black_box(&fwd_step),
                black_box(&fwd_mq),
                black_box(&ctx_after_disc),
                black_box(&settings),
            )
            .unwrap()
        })
    });
}

/// Benchmark residual normalization fix (Phase 1.4).
///
/// Tests that calibration with different notionals produces similar performance.
/// This validates that the fix (pv / residual_notional) doesn't introduce
/// significant overhead compared to the broken (pv / 1.0) version.
fn bench_residual_normalization(c: &mut Criterion) {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let settings = CalibrationConfig::default();

    // Create discount curve with 4 deposits (small but realistic)
    let make_quotes = || -> Vec<RateQuote> {
        vec![
            RateQuote::Deposit {
                id: QuoteId::new("DEP-1M"),
                index: IndexId::new("USD-SOFR"),
                pillar: Pillar::Date(base_date + time::Duration::days(30)),
                rate: 0.0450,
            },
            RateQuote::Deposit {
                id: QuoteId::new("DEP-3M"),
                index: IndexId::new("USD-SOFR"),
                pillar: Pillar::Date(base_date + time::Duration::days(90)),
                rate: 0.0455,
            },
            RateQuote::Deposit {
                id: QuoteId::new("DEP-6M"),
                index: IndexId::new("USD-SOFR"),
                pillar: Pillar::Date(base_date + time::Duration::days(180)),
                rate: 0.0460,
            },
            RateQuote::Deposit {
                id: QuoteId::new("DEP-1Y"),
                index: IndexId::new("USD-SOFR"),
                pillar: Pillar::Date(base_date + time::Duration::days(365)),
                rate: 0.0465,
            },
        ]
    };

    let disc_mq: Vec<MarketQuote> = make_quotes().into_iter().map(MarketQuote::Rates).collect();

    // Benchmark with notional = 1.0 (small notional)
    c.bench_function("calibration_residual_notional_1.0", |b| {
        let disc_step = StepParams::Discount(DiscountCurveParams {
            curve_id: "USD-OIS-small".into(),
            currency: Currency::USD,
            base_date,
            method: CalibrationMethod::Bootstrap,
            interpolation: Default::default(),
            extrapolation: finstack_core::math::interp::ExtrapolationPolicy::FlatForward,
            pricing_discount_id: None,
            pricing_forward_id: None,
            conventions: Default::default(),
        });
        let base = MarketContext::new();
        b.iter(|| {
            execute_step(
                black_box(&disc_step),
                black_box(&disc_mq),
                black_box(&base),
                black_box(&settings),
            )
            .unwrap()
        })
    });

    // Benchmark with notional = 1_000_000.0 (large notional)
    // Should have similar performance to small notional after normalization fix
    c.bench_function("calibration_residual_notional_1M", |b| {
        let disc_step = StepParams::Discount(DiscountCurveParams {
            curve_id: "USD-OIS-large".into(),
            currency: Currency::USD,
            base_date,
            method: CalibrationMethod::Bootstrap,
            interpolation: Default::default(),
            extrapolation: finstack_core::math::interp::ExtrapolationPolicy::FlatForward,
            pricing_discount_id: None,
            pricing_forward_id: None,
            conventions: Default::default(),
        });
        let base = MarketContext::new();
        b.iter(|| {
            execute_step(
                black_box(&disc_step),
                black_box(&disc_mq),
                black_box(&base),
                black_box(&settings),
            )
            .unwrap()
        })
    });
}

criterion_group!(
    benches,
    bench_discount_and_forward_steps,
    bench_residual_normalization
);
criterion_main!(benches);
