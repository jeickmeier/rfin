use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::deposit::Deposit;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::metrics::{MetricId, MetricRegistry};
use std::sync::Arc;

fn date(y: i32, m: u8, d: u8) -> Date {
    Date::from_calendar_date(y, time::Month::try_from(m).unwrap(), d).unwrap()
}

fn ctx_with_flat_disc(base: Date, id: &str) -> MarketContext {
    let disc = DiscountCurve::builder(id)
        .base_date(base)
        .knots([(0.0, 1.0), (1.0, 0.98), (2.0, 0.96)])
        .build()
        .unwrap();
    MarketContext::new().insert_discount(disc)
}

fn sample_deposit(base: Date) -> Deposit {
    Deposit::builder()
        .id(InstrumentId::new("DEP-TEST"))
        .notional(Money::new(1_000_000.0, Currency::USD))
        .start(date(base.year(), 1, 3))
        .end(date(base.year(), 7, 3))
        .day_count(finstack_core::dates::DayCount::Act360)
        .disc_id(CurveId::new("USD-OIS"))
        .build()
        .unwrap()
}

#[test]
fn df_metrics_match_engine_basis() {
    let base = date(2025, 1, 1);
    let ctx = ctx_with_flat_disc(base, "USD-OIS");
    let dep = sample_deposit(base);

    // Pull DF via curve API
    let disc = ctx.get_ref::<DiscountCurve>("USD-OIS").unwrap();
    let df_start_curve = disc.df_on_date_curve(dep.start);
    let df_end_curve = disc.df_on_date_curve(dep.end);

    // Compute via metrics registry
    let mut registry = MetricRegistry::new();
    finstack_valuations::instruments::deposit::metrics::register_deposit_metrics(&mut registry);
    let base_val = dep.value(&ctx, base).unwrap();
    let instrument_arc: Arc<dyn Instrument> = Arc::new(dep.clone());
    let mut ctx_metrics = finstack_valuations::metrics::MetricContext::new(
        instrument_arc,
        Arc::new(ctx.clone()),
        base,
        base_val,
    );
    let measures = registry
        .compute(&[MetricId::DfStart, MetricId::DfEnd], &mut ctx_metrics)
        .unwrap();

    assert!((measures[&MetricId::DfStart] - df_start_curve).abs() < 1e-12);
    assert!((measures[&MetricId::DfEnd] - df_end_curve).abs() < 1e-12);
}

#[test]
fn par_rate_makes_pv_close_to_zero_when_quote_is_set() {
    let base = date(2025, 1, 1);
    let ctx = ctx_with_flat_disc(base, "USD-OIS");
    let mut dep = sample_deposit(base);

    // Compute par rate via metrics
    let mut registry = MetricRegistry::new();
    finstack_valuations::instruments::deposit::metrics::register_deposit_metrics(&mut registry);
    let base_val = dep.value(&ctx, base).unwrap();
    let instrument_arc: Arc<dyn Instrument> = Arc::new(dep.clone());
    let mut ctx_metrics = finstack_valuations::metrics::MetricContext::new(
        instrument_arc,
        Arc::new(ctx.clone()),
        base,
        base_val,
    );
    let measures = registry
        .compute(
            &[MetricId::Yf, MetricId::DfStart, MetricId::DfEnd, MetricId::DepositParRate],
            &mut ctx_metrics,
        )
        .unwrap();
    let par = measures[&MetricId::DepositParRate];

    // Set quote to par and price
    dep.quote_rate = Some(par);
    let pv = dep.value(&ctx, base).unwrap();
    assert!(pv.amount().abs() < 1.0, "PV not near zero: {}", pv.amount());
}

#[test]
fn zero_rate_and_zero_length_edge_cases() {
    let base = date(2025, 1, 1);
    let ctx = ctx_with_flat_disc(base, "USD-OIS");
    // zero-rate case
    let mut dep = sample_deposit(base);
    dep.quote_rate = Some(0.0);
    let pv_zero_rate = dep.value(&ctx, base).unwrap();
    assert!(pv_zero_rate.amount() <= 0.0);

    // zero-length case: start == end → yf = 0, redemption = notional
    let mut dep_zero_len = Deposit::builder()
        .id(InstrumentId::new("DEP-ZERO"))
        .notional(Money::new(1_000_000.0, Currency::USD))
        .start(base)
        .end(base)
        .day_count(finstack_core::dates::DayCount::Act360)
        .disc_id(CurveId::new("USD-OIS"))
        .build()
        .unwrap();
    dep_zero_len.quote_rate = Some(0.05);
    let pv_zero_len = dep_zero_len.value(&ctx, base).unwrap();
    // PV should reduce to -N + N = 0, ignoring tiny numerical noise
    assert!(pv_zero_len.amount().abs() < 1e-9);
}


