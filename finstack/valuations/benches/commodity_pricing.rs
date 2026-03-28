//! Commodity instrument pricing benchmarks.
#![allow(clippy::unwrap_used)]

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount, Tenor, TenorUnit};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::math::interp::InterpStyle;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::commodity::commodity_forward::{CommodityForward, Position};
use finstack_valuations::instruments::commodity::commodity_option::CommodityOption;
use finstack_valuations::instruments::commodity::commodity_swap::CommoditySwap;
use finstack_valuations::instruments::CommodityUnderlyingParams;
use finstack_valuations::instruments::{
    Attributes, ExerciseStyle, Instrument, OptionType, PayReceive, PricingOptions,
    PricingOverrides, SettlementType,
};
use finstack_valuations::metrics::MetricId;
#[allow(dead_code, unused_imports, clippy::expect_used, clippy::unwrap_used)]
mod test_utils {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/support/test_utils.rs"
    ));
}
use rust_decimal::Decimal;
use std::hint::black_box;
use time::Month;

fn base_date() -> Date {
    Date::from_calendar_date(2025, Month::January, 1).unwrap()
}

/// USD-OIS discount, WTI forward curve (contango), and flat WTI vol surface.
fn create_commodity_market() -> MarketContext {
    let as_of = base_date();

    let discount_curve = DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .knots([
            (0.0, 1.0),
            (0.5, 0.975),
            (1.0, 0.95),
            (2.0, 0.90),
            (5.0, 0.78),
            (10.0, 0.60),
        ])
        .interp(InterpStyle::Linear)
        .build()
        .unwrap();

    let price_curve = test_utils::contango_price_curve("WTI-FWD", as_of, 75.0, 0.05, 5.0);
    let vol_surface = test_utils::flat_vol_surface(
        "WTI-VOL",
        &[0.25, 0.5, 1.0, 2.0],
        &[50.0, 60.0, 70.0, 80.0, 90.0],
        0.30,
    );

    MarketContext::new()
        .insert(discount_curve)
        .insert(price_curve)
        .insert_surface(vol_surface)
}

fn wti_underlying() -> CommodityUnderlyingParams {
    CommodityUnderlyingParams::new("Energy", "CL", "BBL", Currency::USD)
}

fn forward_contract(label: &str, maturity: Date) -> CommodityForward {
    CommodityForward::builder()
        .id(InstrumentId::new(label))
        .underlying(wti_underlying())
        .quantity(1000.0)
        .multiplier(1.0)
        .maturity(maturity)
        .position(Position::Long)
        .contract_price_opt(Some(72.0))
        .forward_curve_id(CurveId::new("WTI-FWD"))
        .discount_curve_id(CurveId::new("USD-OIS"))
        .attributes(Attributes::default())
        .pricing_overrides(PricingOverrides::default())
        .build()
        .expect("commodity forward for benchmark")
}

fn commodity_swap_contract(label: &str, end: Date) -> CommoditySwap {
    let start = base_date();
    CommoditySwap::builder()
        .id(InstrumentId::new(label))
        .underlying(wti_underlying())
        .quantity(10_000.0)
        .fixed_price(Decimal::try_from(75.0).expect("fixed price"))
        .floating_index_id(CurveId::new("WTI-FWD"))
        .side(PayReceive::PayFixed)
        .start_date(start)
        .maturity(end)
        .frequency(Tenor::new(1, TenorUnit::Months))
        .discount_curve_id(CurveId::new("USD-OIS"))
        .attributes(Attributes::default())
        .pricing_overrides(PricingOverrides::default())
        .build()
        .expect("commodity swap for benchmark")
}

fn commodity_call_option(label: &str, expiry: Date) -> CommodityOption {
    CommodityOption::builder()
        .id(InstrumentId::new(label))
        .underlying(wti_underlying())
        .strike(75.0)
        .option_type(OptionType::Call)
        .exercise_style(ExerciseStyle::European)
        .expiry(expiry)
        .quantity(1000.0)
        .multiplier(1.0)
        .settlement(SettlementType::Cash)
        .forward_curve_id(CurveId::new("WTI-FWD"))
        .discount_curve_id(CurveId::new("USD-OIS"))
        .vol_surface_id(CurveId::new("WTI-VOL"))
        .day_count(DayCount::Act365F)
        .pricing_overrides(PricingOverrides::default())
        .attributes(Attributes::default())
        .build()
        .expect("commodity option for benchmark")
}

fn bench_commodity_forward_pv(c: &mut Criterion) {
    let mut group = c.benchmark_group("commodity_forward_pv");
    let market = create_commodity_market();
    let as_of = base_date();

    let fwd_3m = forward_contract(
        "WTI-FWD-3M",
        Date::from_calendar_date(2025, Month::April, 1).unwrap(),
    );
    let fwd_6m = forward_contract(
        "WTI-FWD-6M",
        Date::from_calendar_date(2025, Month::July, 1).unwrap(),
    );
    let fwd_1y = forward_contract(
        "WTI-FWD-1Y",
        Date::from_calendar_date(2026, Month::January, 1).unwrap(),
    );

    for (label, fwd) in [("3M", &fwd_3m), ("6M", &fwd_6m), ("1Y", &fwd_1y)] {
        group.bench_with_input(BenchmarkId::from_parameter(label), fwd, |b, fwd| {
            b.iter(|| fwd.value(black_box(&market), black_box(as_of)));
        });
    }
    group.finish();
}

fn bench_commodity_swap_pv(c: &mut Criterion) {
    let mut group = c.benchmark_group("commodity_swap_pv");
    let market = create_commodity_market();
    let as_of = base_date();

    let swap_1y = commodity_swap_contract(
        "WTI-SWAP-1Y",
        Date::from_calendar_date(2026, Month::January, 1).unwrap(),
    );
    let swap_2y = commodity_swap_contract(
        "WTI-SWAP-2Y",
        Date::from_calendar_date(2027, Month::January, 1).unwrap(),
    );
    let swap_5y = commodity_swap_contract(
        "WTI-SWAP-5Y",
        Date::from_calendar_date(2030, Month::January, 1).unwrap(),
    );

    for (label, swap) in [("1Y", &swap_1y), ("2Y", &swap_2y), ("5Y", &swap_5y)] {
        group.bench_with_input(BenchmarkId::from_parameter(label), swap, |b, swap| {
            b.iter(|| swap.value(black_box(&market), black_box(as_of)));
        });
    }
    group.finish();
}

fn bench_commodity_option_pv(c: &mut Criterion) {
    let mut group = c.benchmark_group("commodity_option_pv");
    let market = create_commodity_market();
    let as_of = base_date();

    let opt_3m = commodity_call_option(
        "WTI-OPT-3M",
        Date::from_calendar_date(2025, Month::April, 1).unwrap(),
    );
    let opt_6m = commodity_call_option(
        "WTI-OPT-6M",
        Date::from_calendar_date(2025, Month::July, 1).unwrap(),
    );
    let opt_12m = commodity_call_option(
        "WTI-OPT-12M",
        Date::from_calendar_date(2026, Month::January, 1).unwrap(),
    );

    for (label, opt) in [("3M", &opt_3m), ("6M", &opt_6m), ("12M", &opt_12m)] {
        group.bench_with_input(BenchmarkId::from_parameter(label), opt, |b, opt| {
            b.iter(|| opt.value(black_box(&market), black_box(as_of)));
        });
    }
    group.finish();
}

fn bench_commodity_option_greeks(c: &mut Criterion) {
    let mut group = c.benchmark_group("commodity_option_greeks");
    let market = create_commodity_market();
    let as_of = base_date();

    let opt_6m = commodity_call_option(
        "WTI-OPT-6M-GREEKS",
        Date::from_calendar_date(2025, Month::July, 1).unwrap(),
    );

    let greeks = vec![
        MetricId::Delta,
        MetricId::Gamma,
        MetricId::Vega,
        MetricId::Theta,
    ];

    group.bench_function("6M", |b| {
        b.iter(|| {
            opt_6m.price_with_metrics(
                black_box(&market),
                black_box(as_of),
                black_box(&greeks),
                PricingOptions::default(),
            )
        });
    });
    group.finish();
}

criterion_group!(
    commodity_benches,
    bench_commodity_forward_pv,
    bench_commodity_swap_pv,
    bench_commodity_option_pv,
    bench_commodity_option_greeks
);
criterion_main!(commodity_benches);
