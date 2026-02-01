//! CDS pricing consistency tests via canonical metrics API.
//!
//! These tests exercise protection/premium leg metrics without accessing
//! legacy pricer entry points.

use crate::finstack_test_utils as test_utils;
use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::{DiscountCurve, HazardCurve};
use finstack_core::money::Money;
use finstack_valuations::instruments::credit_derivatives::cds::CreditDefaultSwap;
use finstack_valuations::instruments::Instrument;
use finstack_valuations::metrics::MetricId;
use time::macros::date;

fn build_curves(as_of: Date) -> (DiscountCurve, HazardCurve) {
    let disc = DiscountCurve::builder("USD_OIS")
        .base_date(as_of)
        .day_count(DayCount::Act360)
        .knots([(0.0, 1.0), (1.0, 0.95), (5.0, 0.78), (10.0, 0.61)])
        .build()
        .unwrap();

    let hazard = HazardCurve::builder("CORP")
        .base_date(as_of)
        .recovery_rate(0.40)
        .knots([(0.0, 0.02), (1.0, 0.02), (5.0, 0.025), (10.0, 0.03)])
        .build()
        .unwrap();

    (disc, hazard)
}

fn create_test_cds(as_of: Date, end: Date) -> CreditDefaultSwap {
    test_utils::cds_buy_protection(
        "INTEGRATION_TEST",
        Money::new(10_000_000.0, Currency::USD),
        100.0,
        as_of,
        end,
        "USD_OIS",
        "CORP",
    )
    .expect("CDS construction should succeed")
}

fn metric_value(
    cds: &CreditDefaultSwap,
    market: &MarketContext,
    as_of: Date,
    metric: MetricId,
) -> f64 {
    let result = cds
        .price_with_metrics(market, as_of, std::slice::from_ref(&metric))
        .expect("metric should compute");
    result.measures[&metric]
}

#[test]
fn test_protection_leg_metric_positive() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);
    let (disc, hazard) = build_curves(as_of);
    let market = MarketContext::new()
        .insert_discount(disc)
        .insert_hazard(hazard);

    let cds = create_test_cds(as_of, end);
    let protection_pv = metric_value(&cds, &market, as_of, MetricId::ProtectionLegPv);

    assert!(protection_pv > 0.0);
    assert!(protection_pv.is_finite());
}

#[test]
fn test_premium_leg_metric_positive() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);
    let (disc, hazard) = build_curves(as_of);
    let market = MarketContext::new()
        .insert_discount(disc)
        .insert_hazard(hazard);

    let cds = create_test_cds(as_of, end);
    let premium_pv = metric_value(&cds, &market, as_of, MetricId::PremiumLegPv);

    assert!(premium_pv > 0.0);
    assert!(premium_pv.is_finite());
}

#[test]
fn test_par_spread_metric_positive() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);
    let (disc, hazard) = build_curves(as_of);
    let market = MarketContext::new()
        .insert_discount(disc)
        .insert_hazard(hazard);

    let cds = create_test_cds(as_of, end);
    let par_spread = metric_value(&cds, &market, as_of, MetricId::ParSpread);

    assert!(par_spread > 0.0);
    assert!(par_spread.is_finite());
}
