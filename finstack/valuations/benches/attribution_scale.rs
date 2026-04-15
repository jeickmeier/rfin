//! P&L attribution scaling benchmarks.
//!
//! Measures the per-instrument cost of each public attribution entry point
//! in [`finstack_valuations::attribution`] across realistic portfolio sizes
//! (N ∈ {10, 100, 1000}). All methodologies run against the same pair of
//! market states (`market_t0`, `market_t1`) and as-of dates so the numbers
//! are directly comparable. The shift between `market_t0` and `market_t1`
//! is a 1bp parallel move of the flat USD discount curve — small enough to
//! be realistic, large enough that every methodology has something to
//! decompose.
//!
//! The bench group name is `"attribution"` to match the existing style of
//! `attribution.rs`; individual bench ids are `"<method>/<N>"`.
//!
//! Note: `simple_pnl_bridge` is the minimal baseline (two reprices, no
//! factor loop). The other methodologies all add factor iteration on top
//! and should be benchmarked against the baseline to quantify that cost.

#![allow(clippy::unwrap_used)]

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use finstack_core::config::FinstackConfig;
use finstack_core::currency::Currency;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::math::interp::InterpStyle;
use finstack_core::money::Money;
use finstack_valuations::attribution::{
    attribute_pnl_metrics_based, attribute_pnl_parallel, attribute_pnl_taylor,
    attribute_pnl_waterfall, default_waterfall_order, simple_pnl_bridge, TaylorAttributionConfig,
};
use finstack_valuations::instruments::fixed_income::bond::Bond;
use finstack_valuations::instruments::internal::InstrumentExt;
use finstack_valuations::instruments::PricingOptions;
use finstack_valuations::metrics::MetricId;
use std::hint::black_box;
use std::sync::Arc;
use time::{Date, Month};

// ---------------------------------------------------------------------------
// Shared fixture
// ---------------------------------------------------------------------------

const CURVE_ID: &str = "USD-OIS";
const BASE_RATE: f64 = 0.04;
const SHIFT_BP: f64 = 1.0;
const PORTFOLIO_SIZES: &[usize] = &[10, 100, 1000];

/// Build a flat USD-OIS-style discount curve at the given continuously
/// compounded zero rate.
fn build_flat_curve(as_of: Date, rate: f64) -> DiscountCurve {
    let tenors = [0.0_f64, 1.0, 2.0, 3.0, 5.0, 7.0, 10.0, 20.0, 30.0];
    let knots: Vec<(f64, f64)> = tenors.iter().map(|&t| (t, (-rate * t).exp())).collect();

    DiscountCurve::builder(CURVE_ID)
        .base_date(as_of)
        .knots(knots)
        .interp(InterpStyle::Linear)
        .build()
        .unwrap()
}

/// Rolling-maturity fixed-rate USD corporate bond. Using a short-dated
/// (1–10y) vanilla fixed-coupon bond keeps every pricing path warm without
/// pulling in options/volatility machinery.
fn sample_bond(idx: usize) -> Bond {
    let issue = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    // Spread maturities across 1..=10 years so the portfolio isn't a
    // degenerate duplicate of a single instrument.
    let years = 1 + (idx % 10) as i32;
    let maturity = Date::from_calendar_date(2025 + years, Month::January, 1).unwrap();
    Bond::fixed(
        format!("BENCH-BOND-{idx}"),
        Money::new(1_000_000.0, Currency::USD),
        0.05,
        issue,
        maturity,
        CURVE_ID,
    )
    .unwrap()
}

/// Shared inputs for every methodology. Built once per N per benchmark run
/// so we don't re-allocate curves inside `b.iter`.
struct Fixture {
    bonds: Vec<Arc<dyn InstrumentExt>>,
    market_t0: MarketContext,
    market_t1: MarketContext,
    as_of_t0: Date,
    as_of_t1: Date,
    config: FinstackConfig,
}

impl Fixture {
    fn new(n: usize) -> Self {
        let as_of_t0 = Date::from_calendar_date(2025, Month::January, 15).unwrap();
        let as_of_t1 = Date::from_calendar_date(2025, Month::January, 16).unwrap();

        let curve_t0 = build_flat_curve(as_of_t0, BASE_RATE);
        let curve_t1 = build_flat_curve(as_of_t1, BASE_RATE + SHIFT_BP / 10_000.0);

        let market_t0 = MarketContext::new().insert(curve_t0);
        let market_t1 = MarketContext::new().insert(curve_t1);

        let bonds: Vec<Arc<dyn InstrumentExt>> = (0..n)
            .map(|i| Arc::new(sample_bond(i)) as Arc<dyn InstrumentExt>)
            .collect();

        Self {
            bonds,
            market_t0,
            market_t1,
            as_of_t0,
            as_of_t1,
            config: FinstackConfig::default(),
        }
    }
}

/// Metrics requested for the metrics-based methodology. Limited to the
/// bond-applicable subset so `price_with_metrics` does not fail under
/// strict mode; `attribute_pnl_metrics_based` handles missing metrics
/// gracefully via `measures.get()`.
fn bond_attribution_metrics() -> Vec<MetricId> {
    vec![MetricId::Dv01, MetricId::Theta, MetricId::Convexity]
}

// ---------------------------------------------------------------------------
// Per-methodology inner loops
// ---------------------------------------------------------------------------

fn run_simple_bridge(fx: &Fixture) {
    for bond in &fx.bonds {
        let pnl = simple_pnl_bridge(
            bond,
            black_box(&fx.market_t0),
            black_box(&fx.market_t1),
            black_box(fx.as_of_t0),
            black_box(fx.as_of_t1),
            Currency::USD,
        )
        .unwrap();
        black_box(pnl);
    }
}

fn run_metrics_based(fx: &Fixture) {
    let metrics = bond_attribution_metrics();
    let opts = PricingOptions::default();
    for bond in &fx.bonds {
        let val_t0 = bond
            .price_with_metrics(&fx.market_t0, fx.as_of_t0, &metrics, opts.clone())
            .unwrap();
        let val_t1 = bond
            .price_with_metrics(&fx.market_t1, fx.as_of_t1, &metrics, opts.clone())
            .unwrap();
        let attr = attribute_pnl_metrics_based(
            bond,
            black_box(&fx.market_t0),
            black_box(&fx.market_t1),
            &val_t0,
            &val_t1,
            black_box(fx.as_of_t0),
            black_box(fx.as_of_t1),
        )
        .unwrap();
        black_box(attr);
    }
}

fn run_parallel(fx: &Fixture) {
    for bond in &fx.bonds {
        let attr = attribute_pnl_parallel(
            bond,
            black_box(&fx.market_t0),
            black_box(&fx.market_t1),
            black_box(fx.as_of_t0),
            black_box(fx.as_of_t1),
            &fx.config,
            None,
        )
        .unwrap();
        black_box(attr);
    }
}

fn run_waterfall(
    fx: &Fixture,
    factor_order: &[finstack_valuations::attribution::AttributionFactor],
) {
    for bond in &fx.bonds {
        let attr = attribute_pnl_waterfall(
            bond,
            black_box(&fx.market_t0),
            black_box(&fx.market_t1),
            black_box(fx.as_of_t0),
            black_box(fx.as_of_t1),
            &fx.config,
            factor_order.to_vec(),
            false,
            None,
        )
        .unwrap();
        black_box(attr);
    }
}

fn run_taylor(fx: &Fixture, taylor_cfg: &TaylorAttributionConfig) {
    for bond in &fx.bonds {
        let attr = attribute_pnl_taylor(
            bond,
            black_box(&fx.market_t0),
            black_box(&fx.market_t1),
            black_box(fx.as_of_t0),
            black_box(fx.as_of_t1),
            taylor_cfg,
        )
        .unwrap();
        black_box(attr);
    }
}

// ---------------------------------------------------------------------------
// Criterion entry point
// ---------------------------------------------------------------------------

fn bench_attribution_scale(c: &mut Criterion) {
    let mut group = c.benchmark_group("attribution");
    // Full-fat sampling on a 1000-instrument portfolio with waterfall/parallel
    // attribution is pathologically slow, so we shrink the sample count. The
    // default (100) would take minutes per size; 10 samples is enough to see
    // scaling trends for regression tracking.
    group.sample_size(10);

    let waterfall_order = default_waterfall_order();
    let taylor_cfg = TaylorAttributionConfig::default();

    for &n in PORTFOLIO_SIZES {
        let fx = Fixture::new(n);
        group.throughput(Throughput::Elements(n as u64));

        group.bench_with_input(BenchmarkId::new("simple_bridge", n), &fx, |b, fx| {
            b.iter(|| run_simple_bridge(fx));
        });

        group.bench_with_input(BenchmarkId::new("metrics_based", n), &fx, |b, fx| {
            b.iter(|| run_metrics_based(fx));
        });

        group.bench_with_input(BenchmarkId::new("parallel", n), &fx, |b, fx| {
            b.iter(|| run_parallel(fx));
        });

        group.bench_with_input(BenchmarkId::new("waterfall", n), &fx, |b, fx| {
            b.iter(|| run_waterfall(fx, &waterfall_order));
        });

        group.bench_with_input(BenchmarkId::new("taylor", n), &fx, |b, fx| {
            b.iter(|| run_taylor(fx, &taylor_cfg));
        });
    }

    group.finish();
}

criterion_group!(benches, bench_attribution_scale);
criterion_main!(benches);
