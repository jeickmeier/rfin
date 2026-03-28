//! Fixed income misc pricing benchmarks.

#![allow(clippy::unwrap_used)]

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, StubKind, Tenor};
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::CurveId;
use finstack_valuations::cashflow::builder::specs::CouponType;
use finstack_valuations::instruments::fixed_income::mbs_passthrough::AgencyMbsPassthrough;
use finstack_valuations::instruments::fixed_income::revolving_credit::{
    BaseRateSpec, DrawRepaySpec, RevolvingCredit, RevolvingCreditFees,
};
use finstack_valuations::instruments::fixed_income::tba::AgencyTba;
use finstack_valuations::instruments::fixed_income::term_loan::{
    AmortizationSpec, RateSpec, TermLoan,
};
use finstack_valuations::instruments::Instrument;
use rust_decimal::Decimal;
use std::hint::black_box;
use time::Month;

#[allow(dead_code, unused_imports, clippy::expect_used, clippy::unwrap_used)]
mod test_utils {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/support/test_utils.rs"
    ));
}

fn as_of() -> Date {
    Date::from_calendar_date(2025, Month::January, 1).unwrap()
}

fn term_loan(maturity: Date) -> TermLoan {
    let base = as_of();
    TermLoan::builder()
        .id("TL-BENCH".into())
        .currency(Currency::USD)
        .notional_limit(Money::new(10_000_000.0, Currency::USD))
        .issue_date(base)
        .maturity(maturity)
        .rate(RateSpec::Fixed { rate_bp: 500 })
        .frequency(Tenor::semi_annual())
        .day_count(DayCount::Act360)
        .bdc(BusinessDayConvention::ModifiedFollowing)
        .calendar_id_opt(None)
        .stub(StubKind::None)
        .discount_curve_id(CurveId::from("USD-OIS"))
        .amortization(AmortizationSpec::None)
        .coupon_type(CouponType::Cash)
        .upfront_fee_opt(None)
        .ddtl_opt(None)
        .covenants_opt(None)
        .pricing_overrides(Default::default())
        .attributes(Default::default())
        .build()
        .unwrap()
}

fn revolving_credit_floating(maturity: Date) -> RevolvingCredit {
    RevolvingCredit::builder()
        .id("RC-BENCH".into())
        .commitment_amount(Money::new(10_000_000.0, Currency::USD))
        .drawn_amount(Money::new(5_000_000.0, Currency::USD))
        .commitment_date(as_of())
        .maturity(maturity)
        .base_rate_spec(BaseRateSpec::Floating(
            finstack_valuations::cashflow::builder::FloatingRateSpec {
                index_id: "USD-SOFR-3M".into(),
                spread_bp: Decimal::try_from(100.0).unwrap(),
                gearing: Decimal::try_from(1.0).unwrap(),
                gearing_includes_spread: true,
                floor_bp: None,
                all_in_floor_bp: None,
                cap_bp: None,
                index_cap_bp: None,
                reset_freq: Tenor::quarterly(),
                reset_lag_days: 2,
                dc: DayCount::Act360,
                bdc: BusinessDayConvention::ModifiedFollowing,
                calendar_id: "weekends_only".to_string(),
                fixing_calendar_id: None,
                end_of_month: false,
                overnight_compounding: None,
                fallback: Default::default(),
                payment_lag_days: 0,
            },
        ))
        .day_count(DayCount::Act360)
        .frequency(Tenor::quarterly())
        .fees(RevolvingCreditFees::flat(25.0, 10.0, 0.0))
        .draw_repay_spec(DrawRepaySpec::Deterministic(vec![]))
        .discount_curve_id("USD-OIS".into())
        .build()
        .unwrap()
}

fn revolving_credit_market(as_of: Date) -> MarketContext {
    let disc = test_utils::flat_discount("USD-OIS", as_of, 0.05);
    let fwd =
        finstack_core::market_data::term_structures::ForwardCurve::builder("USD-SOFR-3M", 0.25)
            .base_date(as_of)
            .knots([
                (0.0, 0.04),
                (1.0, 0.045),
                (3.0, 0.048),
                (5.0, 0.05),
                (7.0, 0.052),
            ])
            .build()
            .unwrap();
    MarketContext::new().insert(disc).insert(fwd)
}

fn bench_term_loan_pv(c: &mut Criterion) {
    let mut group = c.benchmark_group("term_loan_pv");
    let as_of = as_of();
    let disc = test_utils::flat_discount("USD-OIS", as_of, 0.05);
    let market = MarketContext::new().insert(disc);

    for (label, maturity) in [
        (
            "3Y",
            Date::from_calendar_date(2028, Month::January, 1).unwrap(),
        ),
        (
            "5Y",
            Date::from_calendar_date(2030, Month::January, 1).unwrap(),
        ),
        (
            "7Y",
            Date::from_calendar_date(2032, Month::January, 1).unwrap(),
        ),
    ] {
        let loan = term_loan(maturity);
        group.bench_with_input(BenchmarkId::from_parameter(label), &label, |b, _| {
            b.iter(|| loan.value(black_box(&market), black_box(as_of)));
        });
    }
    group.finish();
}

fn bench_revolving_credit_pv(c: &mut Criterion) {
    let mut group = c.benchmark_group("revolving_credit_pv");
    let as_of = as_of();
    let market = revolving_credit_market(as_of);

    for (label, maturity) in [
        (
            "3Y",
            Date::from_calendar_date(2028, Month::January, 1).unwrap(),
        ),
        (
            "5Y",
            Date::from_calendar_date(2030, Month::January, 1).unwrap(),
        ),
    ] {
        let facility = revolving_credit_floating(maturity);
        group.bench_with_input(BenchmarkId::from_parameter(label), &label, |b, _| {
            b.iter(|| facility.value(black_box(&market), black_box(as_of)));
        });
    }
    group.finish();
}

fn bench_agency_mbs_pv(c: &mut Criterion) {
    let mut group = c.benchmark_group("agency_mbs_pv");
    let as_of = as_of();
    let disc = test_utils::flat_discount("USD-OIS", as_of, 0.05);
    let market = MarketContext::new().insert(disc);

    let mbs = AgencyMbsPassthrough::example().unwrap();
    group.bench_function("passthrough_example", |b| {
        b.iter(|| mbs.value(black_box(&market), black_box(as_of)));
    });

    let tba = AgencyTba::example().unwrap();
    group.bench_function("tba_example", |b| {
        b.iter(|| tba.value(black_box(&market), black_box(as_of)));
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_term_loan_pv,
    bench_revolving_credit_pv,
    bench_agency_mbs_pv
);
criterion_main!(benches);
